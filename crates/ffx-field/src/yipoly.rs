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

/// A texture region for [`yi_poly_tex_map_xz`] (the original's last four
/// arguments): the texture's height, the GS `TEX0` buffer word `tbp`
/// (base bits 0..8, buffer width bits 16..21 in 64 texels, format bits
/// 24..29; 0 for the default), and the `CLAMP` ranges `u` and `v` (the
/// first texel in bits 0..10, the last in 16..26; `u` 0 for the default
/// 256 x 256 region).
#[derive(Clone, Copy, Debug)]
pub struct TexRegion {
    pub height: u32,
    pub tbp: u32,
    pub u: u32,
    pub v: u32,
}

/// The highest set bit of `x`'s low 16 bits, or -1 (the `TW`/`TH` loops).
fn log2_16(x: u32) -> u32 {
    (0..16)
        .rev()
        .find(|&i| x & (1 << i) != 0)
        .unwrap_or(u32::MAX)
}

/// `fistp` under the round-toward-zero control word the original sets,
/// low 16 bits (NaN and out of range give the integer indefinite, whose
/// low half is 0).
#[allow(clippy::cast_possible_truncation)] // the u16 store
fn chop16(x: f64) -> u16 {
    ffx_bridge::x87::cvttsd2si(x) as u16
}

/// `yiPolyTexMapXZ` (`0x00912b70`, name ours): maps a packet built by
/// [`yi_poly_split_rect_xz`] onto a texture laid over the x/z rectangle
/// `min..max` (vectors; x and z read). Every group tag gets the region's
/// `TEX0` low word at `+0x08` and `+0x10` (`tbp`'s base, buffer width and
/// format; `TW`/`TH` the highest set bit below 16 of the width and the
/// height, or -1; 0x221a00 and those for `tbp` 0), its `CLAMP` at `+0x18`
/// (region clamp; 0x3fc00a for `u` 0) and 0 at `+0x0c`, `+0x14`, `+0x1c`.
/// The width is the buffer width's (256 for `u` 0, the height 256 too).
/// Untextured triangles (tag 0x20, 0x14 bytes each) get grey colours
/// (0x80808080); textured ones (0x22, 0x20 bytes) too, and UVs (`u16
/// +0x14..+0x1e`, in 1/4096) from their vertices' x and z (vertex
/// indices `u16 +0x0c..+0x10`, 0x10 bytes each from `packet + [+0x08]`):
/// `(x - min.x) / (max.x - min.x)` and `(-z - min.z) / (max.z - min.z)`,
/// scaled to the region (its first texel over the width, its texel count
/// over the width; v over the height), plus half a texel, each step
/// stored as `f32`, then truncated (`fistp` under chop; out of range 0).
/// The triangles of other tags are counted over without moving on; a tag
/// of kind 0xff ends the packet, else its `i16 +0x10` records (tags and
/// triangles) do.
///
/// The original leaves its loop registers in `eax`; its one caller
/// (`0x00911fc0`) clears it.
pub fn yi_poly_tex_map_xz(mem: &mut impl Mem, packet: u32, min: u32, max: u32, tex: TexRegion) {
    use ffx_bridge::x87::{add, div, ftol, mul, sub};
    let st = |x: f64| x87::load(x87::store(x));

    // the region in texture coordinates, and the texture's size
    let (offset, scale, width, height, clamp);
    if tex.u == 0 {
        (offset, scale) = ([0.0, 0.0], [1.0, 1.0]);
        (width, height, clamp) = (0x100, 0x100, 0x003f_c00a);
    } else {
        let (u0, u1) = (tex.u & 0x7ff, (tex.u >> 16) & 0x7ff);
        let (v0, v1) = (tex.v & 0x7ff, (tex.v >> 16) & 0x7ff);
        width = ((tex.tbp >> 16) & 0x3f) << 6;
        height = tex.height;
        clamp = u0 << 4 | v0 << 24 | u1 << 14 | 0xa;
        let w = f64::from(width);
        let h = st(f64::from(height.cast_signed())); // fild, then an f32 store
        // the region's size in texels, through ftol2_sse
        let span = |lo: u32, hi: u32| {
            let n = ftol(mem, add(sub(f64::from(hi), f64::from(lo)), 1.0));
            f64::from(n.cast_signed())
        };
        offset = [st(div(f64::from(u0), w)), st(div(f64::from(v0), h))];
        scale = [st(div(span(u0, u1), w)), st(div(span(v0, v1), h))];
    }
    let half = [
        st(div(0.5, f64::from(width))),
        st(div(0.5, st(f64::from(height.cast_signed())))),
    ];
    let (tw, th) = (log2_16(width), log2_16(height));
    let tex0 = if tex.tbp == 0 {
        th << 30 | tw << 26 | 0x0022_1a00
    } else {
        let t = tex.tbp;
        ((t >> 24) & 0x3f) << 20 | ((t >> 16) & 0x3f) << 14 | th << 30 | tw << 26 | (t & 0x1ff) << 5
    };

    let size = [
        st(sub(x87::load(mem.read(max)), x87::load(mem.read(min)))),
        st(sub(
            x87::load(mem.read(max + 8)),
            x87::load(mem.read(min + 8)),
        )),
    ];
    let verts = packet.wrapping_add(mem.read::<u32>(packet + 8));
    let mut at = packet.wrapping_add(0x20);
    let mut done = 0;
    while done < i32::from(mem.read::<i16>(packet + 0x10)) {
        let tag = at;
        at = at.wrapping_add(0x20);
        done += 1;
        if mem.read::<u8>(tag + 1) == 0xff {
            break;
        }
        mem.write(tag + 0x18, clamp);
        mem.write(tag + 8, tex0);
        mem.write(tag + 0x10, tex0);
        mem.write(tag + 0xc, 0u32);
        mem.write(tag + 0x14, 0u32);
        mem.write(tag + 0x1c, 0u32);
        let mut i = 0;
        while i < i32::from(mem.read::<i16>(tag + 2)) {
            match mem.read::<u8>(tag + 1) {
                0x20 => {
                    mem.write(at, [0x8080_8080u32; 3]);
                    at = at.wrapping_add(0x14);
                    done += 1;
                }
                0x22 => {
                    let tri = at;
                    at = at.wrapping_add(0x20);
                    mem.write(tri, [0x8080_8080u32; 3]);
                    // the original rereads min for every vertex; nothing is
                    // written between those reads
                    let origin = [x87::load(mem.read(min)), x87::load(mem.read(min + 8))];
                    let mut uv = [[0u16; 2]; 3];
                    for (k, pair) in (0u32..).zip(uv.iter_mut()) {
                        let index = u32::from(mem.read::<u16>(tri + 0xc + 2 * k));
                        let v = verts.wrapping_add(index * 16);
                        let x = x87::load(mem.read(v));
                        let z = x87::load(mem.read(v + 8));
                        let s = [
                            st(div(sub(x, origin[0]), size[0])),
                            st(div(sub(-z, origin[1]), size[1])),
                        ];
                        for (j, out) in pair.iter_mut().enumerate() {
                            let coord = st(add(mul(s[j], scale[j]), offset[j]));
                            let coord = st(add(coord, half[j]));
                            *out = chop16(mul(coord, 4096.0));
                        }
                    }
                    for (k, pair) in (0u32..).zip(uv) {
                        mem.write(tri + 0x14 + 4 * k, pair);
                    }
                    done += 1;
                }
                _ => {}
            }
            i += 1;
        }
    }
}

/// The default render target of [`yi_tex_target_clear`]: a GS `FRAME`
/// word (buffer 0xd0, width 8 x 64 pixels, format 2: 16-bit colour).
pub const DEFAULT_FRAME: u32 = 0x0208_00d0;
/// `0x00912690(packet, qwc)`: sends a DMA packet on channel 2 (path 3)
/// after syncing the MFIFO and the GS paths; returns `sceGsSyncPath`'s.
pub const SEND_GS_PACKET: u32 = 0x0091_2690;
/// [`yi_tex_target_clear`]'s scratch: the packet, up to 0x150 bytes
/// (16-byte aligned for the DMA).
pub const CLEAR_WORK: u32 = 0x150;

/// GS register numbers in the packet's A+D quadwords.
const PRIM: u32 = 0x00;
const RGBAQ: u32 = 0x01;
const XYZ2: u32 = 0x05;
const XYZ3: u32 = 0x0d;
const XYOFFSET_1: u32 = 0x18;
const SCISSOR_1: u32 = 0x40;
const TEST_1: u32 = 0x47;
const FRAME_1: u32 = 0x4c;
const ZBUF_1: u32 = 0x4e;
const TEXFLUSH: u32 = 0x3f;

/// `yiTexTargetClear` (`0x009126f0`, name ours): one GS packet, sent with
/// [`SEND_GS_PACKET`] (its result returned), that clears the frame buffer
/// 0x108 (format 1, width 8) to black over the screen (offset 0x800, 0;
/// sprite `0x7000, 0x7300` to `0x9000, 0x8d00`, Z always) and then fills a rectangle of
/// `frame` (0 for [`DEFAULT_FRAME`]) with opaque black (`RGBAQ`
/// 0xff000000), centred at 0x800, 0x800 by `XYOFFSET` (12.4):
/// - `u` 0: `w` by `h` (`w`, `h` signed; offset `0x800 - w / 2`, `0x800 -
///   h / 2`, toward zero), no scissor;
/// - else `u` and `v` are `CLAMP`-style ranges (first texel bits 0..10,
///   last 16..26): the offset is `0x800 - (last + 1) / 2` on each axis,
///   the rectangle from first to last texel past it, inclusive, and `u`
///   itself is written as `SCISSOR_1` (its high word 0).
///
/// The `XYOFFSET` quadword's y half is 0 (the original drops it). Every
/// register is an A+D pair (value low and high words, register number);
/// the GIF tag is `NLOOP` qwc - 1, `EOP`, `NREG` 1 (`A+D`). Its one caller
/// (`0x00911fc0`) passes its region width as `v` when `u` is a range.
///
/// `work` is [`CLEAR_WORK`] bytes, 16-byte aligned.
pub fn yi_tex_target_clear<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
    mem: &mut M,
    frame: u32,
    (u, v): (u32, u32),
    h: u32,
    work: u32,
    call: &mut C,
) -> u32 {
    let frame = if frame == 0 { DEFAULT_FRAME } else { frame };
    let half = |n: u32| (n.cast_signed() / 2).cast_unsigned();
    // the offset, and the rectangle's corners in the offset's frame
    let (x0, y0, x1, y1);
    let mut regs: Vec<(u32, u32)> = vec![
        (0x800 << 4, XYOFFSET_1),
        (0x0108_0108, FRAME_1),
        (0x3100_0108, ZBUF_1),
        (0x0003_0000, TEST_1),
        (6, PRIM),
        (0, RGBAQ),
        (0x7300_7000, XYZ2),
        (0x8d00_9000, XYZ2),
        (frame, FRAME_1),
    ];
    if u == 0 {
        let (ox, oy) = (
            0x800u32.wrapping_sub(half(v)),
            0x800u32.wrapping_sub(half(h)),
        );
        (x0, y0, x1, y1) = (ox, oy, ox.wrapping_add(v), oy.wrapping_add(h));
        regs.push((ox << 4, XYOFFSET_1));
    } else {
        let (u0, u1) = (u & 0x7ff, (u >> 16) & 0x7ff);
        let (v0, v1) = (v & 0x7ff, (v >> 16) & 0x7ff);
        let (ox, oy) = (0x800 - u1.div_ceil(2), 0x800 - v1.div_ceil(2));
        regs.push((u, SCISSOR_1));
        regs.push((ox << 4, XYOFFSET_1));
        (x0, y0) = (ox + u0, oy + v0);
        (x1, y1) = (x0 + 1 + u1.wrapping_sub(u0), y0 + 1 + v1.wrapping_sub(v0));
    }
    let xy = |x: u32, y: u32| (y << 4) << 16 | x << 4;
    regs.extend([
        (0x3100_0108, ZBUF_1),
        (0x0003_0000, TEST_1),
        (6, PRIM),
        (0xff00_0000, RGBAQ),
        (xy(x0, y0), XYZ3),
        (xy(x1, y1), XYZ2),
        (0x0007_0000, TEST_1),
        (0, TEXFLUSH),
        (frame, FRAME_1),
    ]);
    let qwc = 1 + u32::try_from(regs.len()).unwrap_or(0);
    mem.write(work, [0x8000 | (qwc - 1), 0x1000_0000, 0xe, 0]);
    for (at, (value, reg)) in (work + 0x10..).step_by(0x10).zip(regs) {
        mem.write(at, [value, 0, reg, 0]);
    }
    let f = mem.rebase(SEND_GS_PACKET);
    call(mem, f, &[work, qwc])
}
