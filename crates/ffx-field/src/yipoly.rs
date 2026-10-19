//! The yi renderer's load-time packet builders: they run a map part's
//! VIF chain on the poly buffer (`yiPolyBufExecVif`) and rebuild its
//! triangles as group-tagged packets.
//!
//! A chain is a list of nodes (`+0x44` the VIF data, `+0x48` its size in
//! quadwords, `+0x54` the next node). Each microprogram call leaves a
//! block in the poly buffer's VU memory at `16 * [+0x406c]` (the `TOPS`
//! it ran on): its triangle count at `+0x00`, a texture flag `+0x20`, the
//! texture registers `+0x30`/`+0x34` (and `+0x40`/`+0x44` when `+0x48`),
//! `+0x50`; then from quadword 10 three GIF `RGBAQ` quadwords a triangle,
//! from `3 n + 10` three `UV` quadwords and from `6 n + 10` three
//! positions (`n` triangles).
//!
//! A packet starts with a 0x20-byte head ([`write_head`]): `+0x00`
//! 0x400, `+0x04` 0x20, `+0x08` its size in bytes (once built), `u16
//! +0x10` the records after it, `i16 +0x12` the vertices. Group tags
//! ([`GroupTag`], 0x20 bytes, `u16 +0x02` the triangles after it) open
//! each node and each textured run; a tag of kind 0xff ends the
//! packet. Then the vertices follow (x, y, z, 1.0; through the matrix
//! when there is one).

use ffx_bridge::Mem;

use crate::x87;

use crate::yi::bridge::{
    yi_free, yi_malloc_quiet, yi_poly_buf_clear, yi_poly_buf_exec_vif, yi_rect_tri_overlap_xz,
    yi_tri_colors_from_gif, yi_tri_colors_uv_from_gif,
};

/// The poly buffer (`0x025d1800`): 0x4000 bytes of VU memory, then the
/// VIF registers (`yiPolyBufExecVif`).
pub const POLY_BUF: u32 = 0x025d_1800;
/// The `TOPS` of the last microprogram call (`POLY_BUF + 0x406c`).
const LAST_TOPS: u32 = 0x406c;
/// The four VIF codes every node starts with (`.rdata 0x00c5db80`:
/// `MARK 0`, `BASE 0x60`, `OFFSET 0x1d0`, `NOP`); the node's own first
/// four words are skipped.
pub const SETUP_CODES: u32 = 0x00c5_db80;
/// `FFXVu0ApplyMatrix(dst, m, src)`.
const APPLY_MATRIX: u32 = crate::yi::VU0_APPLY_MATRIX;

/// [`yi_poly_split_rect_xz`]'s scratch: `yiRectTriOverlapXZ`'s locals
/// (0xa4 bytes) at 0, the triangle record (0x20 bytes) at `+0xc0`.
pub const SPLIT_WORK: u32 = 0xe0;
const RECORD: u32 = 0xc0;

/// The two packets of a split: the triangles that overlap the rectangle,
/// and the others.
const INSIDE: usize = 0;
const OUTSIDE: usize = 1;

/// A packet head: 0x400, `second`, then zeros. The original copies it
/// from a stack template whose bytes `+0x16..+0x1c` it never sets; they
/// are 0 here.
fn write_head(mem: &mut impl Mem, packet: u32, second: u32) {
    mem.write(packet, [0x400u32, second, 0, 0, 0, 0, 0, 0]);
}

/// A group tag (0x20 bytes): `kind` at byte 1 (0x20 untextured, 0x22
/// textured, 0xff the end), its triangle count `u16 +0x02` (0 here,
/// counted in the packet), `second` at `+0x04`, the texture registers at
/// `+0x08..+0x18` and `+0x18`/`+0x1c`. `extra` keeps the last textured
/// block's values to the end, as the originals' templates do. Bytes the
/// originals copy from stack they never set are 0 here: in
/// [`yi_poly_split_rect_xz`] byte 0 (an empty chain), `+0x04` and
/// `extra` before the first textured block; in [`yi_poly_select_rect_xz`]
/// `extra[0]` before it (the template is the head's, so `+0x04` is 0x20).
#[derive(Clone, Copy, Default)]
struct GroupTag {
    kind: u8,
    second: u32,
    tex: [u32; 4],
    extra: [u32; 2],
}

impl GroupTag {
    fn write(self, mem: &mut impl Mem, at: u32) {
        let [t0, t1, t2, t3] = self.tex;
        let [e0, e1] = self.extra;
        mem.write(
            at,
            [
                u32::from(self.kind) << 8,
                self.second,
                t0,
                t1,
                t2,
                t3,
                e0,
                e1,
            ],
        );
    }

    /// A textured block's registers (`+0x30`/`+0x34`, `+0x40`/`+0x44`
    /// when `+0x48`, else the first pair again, `+0x50`).
    fn textured(&mut self, mem: &impl Mem, block: u32) {
        let [u, v] = mem.read::<[u32; 2]>(block + 0x30);
        let [u2, v2] = if mem.read::<u32>(block + 0x48) == 0 {
            [u, v]
        } else {
            mem.read::<[u32; 2]>(block + 0x40)
        };
        self.kind = 0x22;
        self.tex = [u, v, u2, v2];
        self.extra = [mem.read::<u32>(block + 0x50), 0];
    }
}

fn bump_u16(mem: &mut impl Mem, at: u32) {
    let v = mem.read::<u16>(at);
    mem.write(at, v.wrapping_add(1));
}

/// The packet's size in words if `cur` is its end and its vertices
/// followed (`(cur - packet) / 4 + 4 (vertices + 0x6b)`, 30 bits): what
/// the capacity check compares.
fn packet_words(mem: &impl Mem, packet: u32, cur: u32) -> u32 {
    let head = (cur.wrapping_sub(packet).cast_signed() >> 2).cast_unsigned();
    let verts = i32::from(mem.read::<i16>(packet + 0x12)).wrapping_add(0x6b);
    head.wrapping_add(verts.wrapping_mul(4).cast_unsigned()) & 0x3fff_ffff
}

/// One side of the split: its packet, where it is written up to, its
/// open group tag, and the vertex buffer the triangles' positions are
/// kept in until the end.
struct Side {
    packet: u32,
    cur: u32,
    group: u32,
    /// Whether a tag for the current textured run has been written.
    open: bool,
    verts: u32,
    verts_base: u32,
}

impl Side {
    /// A triangle to this side: its vertex numbers into the record
    /// (`+0x0c..+0x12`, the packet's count +3), its three 16-byte
    /// vertices to the vertex buffer (x, y, z; w too when `with_w`).
    fn take(&mut self, mem: &mut impl Mem, record: u32, tri: u32, with_w: bool) {
        let first = mem.read::<u16>(self.packet + 0x12);
        let ids = [first, first.wrapping_add(1), first.wrapping_add(2)];
        mem.write(record + 0xc, ids);
        mem.write(self.packet + 0x12, first.wrapping_add(3));
        for v in 0..3 {
            let (src, dst) = (
                tri.wrapping_add(0x10 * v),
                self.verts.wrapping_add(0x10 * v),
            );
            if with_w {
                let xyzw = mem.read::<[u32; 4]>(src);
                mem.write(dst, xyzw);
            } else {
                let xyz = mem.read::<[u32; 3]>(src);
                mem.write(dst, xyz);
            }
        }
        self.verts = self.verts.wrapping_add(0x30);
    }

    /// A triangle record (and a textured group tag first, once a run)
    /// at the end.
    fn put_triangle(&mut self, mem: &mut impl Mem, record: u32, tag: Option<GroupTag>) {
        if let Some(tag) = tag {
            if !self.open {
                self.open_group(mem, tag);
                self.open = true;
            }
            self.count(mem);
            self.put_record(mem, record, 0x20);
        } else {
            self.count(mem);
            self.put_record(mem, record, 0x14);
        }
    }

    /// After the end tag: the size to it at `+0x08`, the vertices (`w`
    /// 1.0, through `m` when not 0) and `*cap` the size in words with
    /// them.
    fn finish_vertices<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        &self,
        mem: &mut M,
        cap: u32,
        m: u32,
        call: &mut C,
    ) {
        let head = (self.cur.wrapping_sub(self.packet).cast_signed() >> 2).cast_unsigned();
        mem.write(self.packet + 8, head << 2);
        let n = i32::from(mem.read::<i16>(self.packet + 0x12));
        let mut i = 0;
        while i < n {
            let k = i.cast_unsigned() << 4;
            let (dst, src) = (self.cur.wrapping_add(k), self.verts_base.wrapping_add(k));
            let xyz = mem.read::<[u32; 3]>(src);
            mem.write(dst, xyz);
            mem.write(dst.wrapping_add(0xc), 1f32);
            if m != 0 {
                let f = mem.rebase(APPLY_MATRIX);
                call(mem, f, &[dst, m, dst]);
            }
            i += 1;
        }
        mem.write(cap, head.wrapping_add(n.wrapping_mul(4).cast_unsigned()));
    }

    /// One more record and one more triangle in the open group.
    fn count(&self, mem: &mut impl Mem) {
        bump_u16(mem, self.packet + 0x10);
        bump_u16(mem, self.group + 2);
    }

    /// A group tag at the end (one more record); it becomes the open one.
    fn open_group(&mut self, mem: &mut impl Mem, tag: GroupTag) {
        bump_u16(mem, self.packet + 0x10);
        tag.write(mem, self.cur);
        self.group = self.cur;
        self.cur += 0x20;
    }

    /// `n` bytes of the record to the end.
    fn put_record(&mut self, mem: &mut impl Mem, record: u32, n: u32) {
        for i in 0..n / 4 {
            let w = mem.read::<u32>(record + 4 * i);
            mem.write(self.cur + 4 * i, w);
        }
        self.cur += n;
    }
}

/// Splits the triangles of the VIF chain `list` by the x/z rectangle from
/// `rect[0]` to `rect[1]` (`0x00910a90`, name ours; at load time, from
/// `0x00912270`): `packets[0]` gets those that overlap it
/// (`yiRectTriOverlapXZ`, through the matrix `m` when not 0), `packets[1]`
/// the others.
///
/// Two vertex buffers of 16 bytes per chain quadword are allocated
/// (`yiMallocQuiet`; the outside one first). Without both, both packets
/// get an empty head (second word 0) and nothing is freed. Else the poly
/// buffer is cleared and each node runs: its setup ([`SETUP_CODES`]), an
/// untextured group tag in each packet, then microprogram block after
/// block while words are left and, unless `unlimited`, both packets'
/// [`packet_words`] stay below `*caps[0]` / `*caps[1]`. In a block's
/// triangles (the count reread each time):
/// - untextured: the colours (`yiTriColorsFromGif`) and vertex numbers,
///   a 0x14-byte record, counted in the node's group;
/// - textured (from the first block with the flag on, to the node's
///   end): the colours and UVs (`yiTriColorsUvFromGif`), a 0x20-byte
///   record, each side opening a textured group tag first (one per
///   textured block that has triangles on that side).
///
/// At the end each packet gets an end tag, its size at `+0x08` (bytes,
/// to the end tag's end), the vertices (`w` 1.0, through
/// `FFXVu0ApplyMatrix(v, m, v)` when `m`), and `*caps[i]` its size in
/// words with them. The buffers are freed (outside, inside). Returns 0.
///
/// `work` is [`SPLIT_WORK`] bytes of scratch.
#[allow(clippy::too_many_arguments)] // the original's nine
#[allow(clippy::too_many_lines)] // one pass, as the original
pub fn yi_poly_split_rect_xz<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
    mem: &mut M,
    list: u32,
    packets: [u32; 2],
    caps: [u32; 2],
    m: u32,
    rect: [u32; 2],
    unlimited: u32,
    work: u32,
    call: &mut C,
) -> u32 {
    let mut quads = 0u32;
    let mut node = list;
    while node != 0 {
        quads = quads.wrapping_add(mem.read::<u32>(node + 0x48));
        node = mem.read::<u32>(node + 0x54);
    }
    let bytes = quads << 4;
    let outside_buf = yi_malloc_quiet(mem, bytes, call);
    let inside_buf = if outside_buf == 0 {
        0
    } else {
        yi_malloc_quiet(mem, bytes, call)
    };
    if inside_buf == 0 {
        for p in packets {
            write_head(mem, p, 0);
        }
        return 0;
    }

    let poly = mem.rebase(POLY_BUF);
    let setup = mem.rebase(SETUP_CODES);
    let record = work + RECORD;
    yi_poly_buf_clear(mem, poly);
    for p in packets {
        write_head(mem, p, 0x20);
    }
    let side = |packet: u32, buf: u32| Side {
        packet,
        cur: packet + 0x20,
        group: 0,
        open: false,
        verts: buf,
        verts_base: buf,
    };
    let mut sides = [side(packets[0], inside_buf), side(packets[1], outside_buf)];
    let mut tag = GroupTag::default();

    let mut node = list;
    while node != 0 {
        let data = mem.read::<u32>(node + 0x44);
        let words = mem.read::<u32>(node + 0x48) << 2;
        let used = yi_poly_buf_exec_vif(mem, poly, setup, 4);
        let mut left = words.wrapping_sub(used).cast_signed();
        let mut src = data.wrapping_add(used << 2);
        tag.kind = 0x20;
        tag.tex = [0; 4];
        for s in &mut sides {
            s.open_group(mem, tag);
            s.open = false;
        }
        let mut textured = false;
        while left > 0 {
            let full = |mem: &M, s: &Side, cap: u32| {
                packet_words(mem, s.packet, s.cur) >= mem.read::<u32>(cap)
            };
            if unlimited == 0
                && (full(mem, &sides[INSIDE], caps[INSIDE])
                    || full(mem, &sides[OUTSIDE], caps[OUTSIDE]))
            {
                break;
            }
            let used = yi_poly_buf_exec_vif(mem, poly, src, left.cast_unsigned());
            left = left.wrapping_sub(used.cast_signed());
            src = src.wrapping_add(used << 2);
            let block = poly.wrapping_add(mem.read::<u32>(poly + LAST_TOPS) << 4);
            if mem.read::<u32>(block + 0x20) != 0 {
                tag.textured(mem, block);
                textured = true;
                for s in &mut sides {
                    s.open = false;
                }
            }
            let n = mem.read::<u32>(block);
            let mut color = 10u32;
            let mut uv = n.wrapping_mul(3).wrapping_add(10);
            let mut tri = block.wrapping_add(n.wrapping_mul(6).wrapping_add(10) << 4);
            let mut i = 0u32;
            while i < mem.read::<u32>(block) {
                if textured {
                    yi_tri_colors_uv_from_gif(mem, record, block, color, uv);
                    uv = uv.wrapping_add(3);
                } else {
                    yi_tri_colors_from_gif(mem, record, block, color);
                }
                color = color.wrapping_add(3);
                let over = yi_rect_tri_overlap_xz(mem, tri, m, rect, work, call);
                let s = &mut sides[if over == 0 { OUTSIDE } else { INSIDE }];
                s.take(mem, record, tri, false);
                s.put_triangle(mem, record, textured.then_some(tag));
                tri = tri.wrapping_add(0x30);
                i += 1;
            }
        }
        node = mem.read::<u32>(node + 0x54);
    }

    // the end tags first, then each packet's vertices
    let [inside, outside] = &mut sides;
    let end = GroupTag {
        kind: 0xff,
        tex: [0; 4],
        ..tag
    };
    inside.open_group(mem, end);
    outside.open_group(mem, end);
    inside.finish_vertices(mem, caps[INSIDE], m, call);
    outside.finish_vertices(mem, caps[OUTSIDE], m, call);
    yi_free(mem, outside_buf, call);
    yi_free(mem, inside_buf, call);
    0
}

/// `Sg_PacketGetPtr()` (`0x0083b650`): the free end of the frame's
/// packet buffer.
pub const SG_PACKET_GET_PTR: u32 = 0x0083_b650;

/// Keeps the triangles of the VIF chain `list` that overlap the x/z
/// rectangle from `rect[0]` to `rect[1]` (`0x009105b0`, name ours; from
/// `0x00912270` when it has no second packet): as
/// [`yi_poly_split_rect_xz`] with one packet and no allocation. The
/// vertices are kept at the packet buffer's free end (`Sg_PacketGetPtr`,
/// whole quadwords, `w` too) and the run also stops when 0x3f0 more would
/// pass 0x10000 quadwords there. A block's colours start at the quadword
/// its `+0x04` gives (UVs `3 n`, positions `6 n` after), its triangle
/// count is read once (none unless positive). Returns 0.
///
/// `work` is [`SPLIT_WORK`] bytes of scratch.
#[allow(clippy::too_many_arguments)] // the original's eight
pub fn yi_poly_select_rect_xz<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
    mem: &mut M,
    list: u32,
    packet: u32,
    cap: u32,
    m: u32,
    rect: [u32; 2],
    unlimited: u32,
    work: u32,
    call: &mut C,
) -> u32 {
    let poly = mem.rebase(POLY_BUF);
    let setup = mem.rebase(SETUP_CODES);
    let record = work + RECORD;
    yi_poly_buf_clear(mem, poly);
    let f = mem.rebase(SG_PACKET_GET_PTR);
    let verts = call(mem, f, &[]);
    write_head(mem, packet, 0x20);
    let mut s = Side {
        packet,
        cur: packet + 0x20,
        group: 0,
        open: false,
        verts,
        verts_base: verts,
    };
    let mut tag = GroupTag {
        second: 0x20,
        ..GroupTag::default()
    };

    let mut node = list;
    while node != 0 {
        let data = mem.read::<u32>(node + 0x44);
        let words = mem.read::<u32>(node + 0x48) << 2;
        let used = yi_poly_buf_exec_vif(mem, poly, setup, 4);
        let mut left = words.wrapping_sub(used).cast_signed();
        let mut src = data.wrapping_add(used << 2);
        tag.kind = 0x20;
        tag.tex = [0; 4];
        s.open_group(mem, tag);
        s.open = false;
        let mut textured = false;
        while left > 0 {
            if unlimited == 0 {
                let kept = (s.verts.wrapping_sub(verts).cast_signed() >> 4).cast_unsigned();
                if packet_words(mem, s.packet, s.cur) >= mem.read::<u32>(cap)
                    || kept.wrapping_add(0x3f0) >= 0x1_0000
                {
                    break;
                }
            }
            let used = yi_poly_buf_exec_vif(mem, poly, src, left.cast_unsigned());
            left = left.wrapping_sub(used.cast_signed());
            src = src.wrapping_add(used << 2);
            let block = poly.wrapping_add(mem.read::<u32>(poly + LAST_TOPS) << 4);
            if mem.read::<u32>(block + 0x20) != 0 {
                tag.textured(mem, block);
                textured = true;
                s.open = false;
            }
            let n = mem.read::<i32>(block);
            let first = mem.read::<u32>(block + 4);
            let n3 = n.cast_unsigned().wrapping_mul(3);
            let mut color = first;
            let mut uv = first.wrapping_add(n3);
            let mut tri = block.wrapping_add(first.wrapping_add(2 * n3) << 4);
            for _ in 0..n.max(0) {
                if textured {
                    yi_tri_colors_uv_from_gif(mem, record, block, color, uv);
                    uv = uv.wrapping_add(3);
                } else {
                    yi_tri_colors_from_gif(mem, record, block, color);
                }
                color = color.wrapping_add(3);
                if yi_rect_tri_overlap_xz(mem, tri, m, rect, work, call) != 0 {
                    s.take(mem, record, tri, true);
                    s.put_triangle(mem, record, textured.then_some(tag));
                }
                tri = tri.wrapping_add(0x30);
            }
        }
        node = mem.read::<u32>(node + 0x54);
    }

    s.open_group(
        mem,
        GroupTag {
            kind: 0xff,
            tex: [0; 4],
            ..tag
        },
    );
    s.finish_vertices(mem, cap, m, call);
    0
}

/// `FFXVu0ScaleVector(dst, src, k)` (`0x006ed8f0`).
const SCALE_VECTOR: u32 = 0x006e_d8f0;

/// [`yi_rect_to_omd_build`]'s scratch: the rectangle's corners (`+0x00`,
/// `+0x10`), a transformed position (`+0x20`), the two capacities
/// (`+0x30`, `+0x34`), then the split's [`SPLIT_WORK`] at `+0x40`.
pub const BUILD_WORK: u32 = 0x40 + SPLIT_WORK;

/// The map parts' table in the renderer (`+0x68788` their count, `0xb4`
/// bytes each from `+0x6880e`).
const PARTS: u32 = 0x6_880e;
const PART_COUNT: u32 = 0x6_8788;

/// `yiRectToOMD`'s body (`0x00912270`, name ours; `r` the renderer):
/// gathers the VIF chains of every map part (not kind 1 at its `u16 +0`,
/// not state 3 at `u16 +2`; chain from `+0x26`, linked by `+0x4c`) whose
/// bounds touch the x/z rectangle from `a` to `b` (each corner's
/// components ordered first, through `FFXVu0ScaleVector(.., 1.0)`): a
/// node's centre (`+0x00..`, through `FFXVu0ApplyMatrix` by `m` with w 1.0
/// when `m`) plus or minus its radius `+0x2c` must reach the rectangle
/// on x and on z (`<=` / `>=`; a NaN fails the min side). They are linked
/// through `+0x54` in that order. With none, both packets that are not 0
/// get an empty head (0x400, the rest 0 but `+0x16..+0x1c`) and 0 is
/// returned. Else [`yi_poly_select_rect_xz`] (no `outside` packet) or
/// [`yi_poly_split_rect_xz`] build them with the capacities `size / 4`
/// (kept in `work`), and the chain's length is returned.
///
/// `work` is [`BUILD_WORK`] bytes of scratch.
#[allow(clippy::too_many_arguments)] // the original's nine
pub fn yi_rect_to_omd_build<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
    mem: &mut M,
    r: u32,
    (inside, inside_size): (u32, u32),
    (outside, outside_size): (u32, u32),
    m: u32,
    rect: [u32; 2],
    unlimited: u32,
    work: u32,
    call: &mut C,
) -> u32 {
    let (lo, hi, v) = (work, work + 0x10, work + 0x20);
    let f = mem.rebase(SCALE_VECTOR);
    call(mem, f, &[lo, rect[0], 1f32.to_bits()]);
    call(mem, f, &[hi, rect[1], 1f32.to_bits()]);
    for i in 0..4 {
        let (a, b) = (lo + 4 * i, hi + 4 * i);
        let (x, y) = (mem.read::<u32>(a), mem.read::<u32>(b));
        if x87::load(y) < x87::load(x) {
            mem.write(a, x87::copy(y));
            mem.write(b, x87::copy(x));
        }
    }
    let ld = |mem: &M, at: u32| x87::load(mem.read(at));
    let st = |x: f64| x87::load(x87::store(x));

    let mut count = 0u32;
    let mut first = 0u32;
    let mut prev = 0u32;
    let parts = mem.read::<u32>(r.wrapping_add(PART_COUNT));
    for k in 0..parts {
        let part = r.wrapping_add(PARTS + 0xb4 * k);
        if mem.read::<u16>(part) == 1 || mem.read::<u16>(part + 2) == 3 {
            continue;
        }
        let mut node = mem.read::<u32>(part + 0x26);
        while node != 0 {
            let radius = ld(mem, node + 0x2c);
            let (x, z) = if m == 0 {
                (ld(mem, node), ld(mem, node + 8))
            } else {
                for i in 0..3 {
                    let c = x87::copy(mem.read(node + 4 * i));
                    mem.write(v + 4 * i, c);
                }
                mem.write(v + 0xc, 1f32);
                let f = mem.rebase(APPLY_MATRIX);
                call(mem, f, &[v, m, v]);
                (ld(mem, v), ld(mem, v + 8))
            };
            let (min_x, max_x) = (st(x - radius), st(radius + x));
            let (min_z, max_z) = (st(z - radius), st(radius + z));
            // fcompp on x87: a NaN takes the "below" branch of the first test
            let reach = |lo: f64, hi: f64, min: f64, max: f64| {
                !(hi < min || min.is_nan() || hi.is_nan()) && lo <= max
            };
            if reach(ld(mem, lo), ld(mem, hi), min_x, max_x)
                && reach(ld(mem, lo + 8), ld(mem, hi + 8), min_z, max_z)
            {
                if first == 0 {
                    first = node;
                } else {
                    mem.write(prev + 0x54, node);
                }
                count += 1;
                prev = node;
                mem.write(node + 0x54, 0u32);
            }
            node = mem.read::<u32>(node + 0x4c);
        }
    }
    if first == 0 {
        for p in [inside, outside] {
            if p != 0 {
                mem.write(p, [0x400u32, 0, 0, 0]);
                mem.write(p + 0x10, 0u16);
                mem.write(p + 0x12, 0u32);
                mem.write(p + 0x1c, 0u32);
            }
        }
        return 0;
    }
    let caps = [work + 0x30, work + 0x34];
    mem.write(caps[0], inside_size >> 2);
    if outside == 0 {
        yi_poly_select_rect_xz(
            mem,
            first,
            inside,
            caps[0],
            m,
            [lo, hi],
            unlimited,
            work + 0x40,
            call,
        );
    } else {
        mem.write(caps[1], outside_size >> 2);
        yi_poly_split_rect_xz(
            mem,
            first,
            [inside, outside],
            caps,
            m,
            [lo, hi],
            unlimited,
            work + 0x40,
            call,
        );
    }
    count
}
