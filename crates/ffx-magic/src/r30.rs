//! Round 30's ports (magic-r30): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The clip region of [`clip_0434`]: the band `top..bottom` and the
/// sides `left`/`right`, `cc = top - bottom`, `e = right bottom - left
/// top` (the diagonal's constant).
#[derive(Clone, Copy)]
struct Region {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
    cc: f32,
}

/// A vertex of the buffer (x, y, z; 16 bytes apart).
#[derive(Clone, Copy)]
struct Vtx {
    x: f32,
    y: f32,
    z: f32,
}

fn vtx<G: Game>(g: &G, at: u32) -> Vtx {
    Vtx {
        x: g.read::<f32>(at),
        y: g.read::<f32>(at + 4),
        z: g.read::<f32>(at + 8),
    }
}

/// `comiss q, p; jbe`: `(p, q)` when `q > p`, else `(q, p)`.
fn range(p: f32, q: f32) -> (f32, f32) {
    if q > p { (p, q) } else { (q, p) }
}

/// `lo > v` or `v > hi` (`comiss` + `ja`: an unordered `v` is inside).
fn outside((lo, hi): (f32, f32), v: f32) -> bool {
    lo > v || v > hi
}

/// The constants one edge test of [`clip_0434`] reads (operand offsets in
/// the function): `zero` the `dx` it compares with (`None`: `xorps`, 0),
/// `one` for `1 - t`, `t1`/`w1` the `t` and `w` when `dx` is that, the
/// two `andpd` masks.
struct EdgeOps {
    zero: Option<u32>,
    one: u32,
    t1: u32,
    w1: u32,
    abs: (u32, u32),
}

/// Where the edge `p`→`q` crosses the diagonal of the region, if it does
/// inside both the edge's and the region's box: `(x, y, z, w, t)`, `t`
/// measured from `p` (`|x - p.x| / |dx|` in `f64`, or `t1` when `dx` is
/// `zero`; `z` then `p.z`).
fn edge<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    r: Region,
    p: Vtx,
    q: Vtx,
    ops: &EdgeOps,
) -> Option<(f32, f32, f32, f32, f32)> {
    let k = |o: u32| g.read::<f32>(img.word(f + o));
    let dx = sub(q.x, p.x);
    let dy = sub(p.y, q.y);
    let lr = sub(r.left, r.right);
    let p1 = mul(dy, lr);
    let p2 = mul(r.cc, dx);
    let den = sub(p1, p2);
    if den == 0.0 {
        return None;
    }
    let cr = sub(mul(p.x, q.y), mul(q.x, p.y));
    let e = sub(mul(r.right, r.bottom), mul(r.left, r.top));
    let x = div(sub(mul(e, dx), mul(cr, lr)), den);
    let y = div(sub(mul(e, dy), mul(cr, r.cc)), sub(p2, p1));
    if outside(range(p.x, q.x), x)
        || outside(range(p.y, q.y), y)
        || outside(range(r.right, r.left), x)
        || outside(range(r.top, r.bottom), y)
    {
        return None;
    }
    let zero = ops.zero.map_or(0.0, k);
    #[allow(clippy::float_cmp)] // ucomiss: the original's exact test
    let eq = dx == zero;
    let (t, z) = if eq {
        (k(ops.t1), p.z)
    } else {
        // cvtps2pd; andpd (the masks); divsd; cvtpd2ps
        let m0 = g.read::<u64>(img.word(f + ops.abs.0));
        let m1 = g.read::<u64>(img.word(f + ops.abs.1));
        let a = f64::from_bits(f64::from(sub(x, p.x)).to_bits() & m0);
        let b = f64::from_bits(f64::from(dx).to_bits() & m1);
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let t = (a / b) as f32;
        (t, add(mul(sub(k(ops.one), t), p.z), mul(q.z, t)))
    };
    Some((x, y, z, k(ops.w1), t))
}

/// `(from (0x400 - t) + to t) / 1024`, toward 0 (`cdq; and edx, 0x3ff;
/// add; sar 10`).
fn lerp(from: u32, to: u32, t: i32) -> u32 {
    let s = from
        .cast_signed()
        .wrapping_mul(0x400i32.wrapping_sub(t))
        .wrapping_add(to.cast_signed().wrapping_mul(t));
    (s / 1024).cast_unsigned()
}

/// A triangle's attributes: its 12 colour bytes (three RGBA words) and
/// the 20 bytes of its record (three u16 vertex indices, a pad, three u16
/// pairs of texture coordinates).
#[derive(Clone, Copy)]
struct Tri {
    col: [u32; 3],
    rec: [u8; 20],
}

impl Tri {
    fn read<G: Game>(g: &G, col: u32, rec: u32) -> Tri {
        let mut r = [0u8; 20];
        g.read_bytes(rec, &mut r);
        Tri {
            col: [0, 4, 8].map(|o| g.read::<u32>(col + o)),
            rec: r,
        }
    }

    fn uv(&self, i: usize) -> [u32; 2] {
        let h = |o: usize| u32::from(u16::from_le_bytes([self.rec[o], self.rec[o + 1]]));
        [h(8 + 4 * i), h(10 + 4 * i)]
    }

    /// Vertex `i` replaced by the new vertex `n` with its `uv` and colour.
    fn set(&mut self, i: usize, n: u32, uv: u32, col: u32) {
        self.rec[2 * i..2 * i + 2].copy_from_slice(&n.to_le_bytes()[..2]);
        self.rec[8 + 4 * i..12 + 4 * i].copy_from_slice(&uv.to_le_bytes());
        self.col[i] = col;
    }

    fn write<G: Game>(&self, g: &mut G, at: u32) {
        for (i, c) in (0u32..).zip(self.col) {
            g.write::<u32>(at + 4 * i, c);
        }
        g.write_bytes(at + 12, &self.rec);
    }
}

/// The new vertex between vertices `i` and `j` of `tri` at `t` (fixed
/// point, 0x400 = `j`): its packed texture coordinates and its colour
/// (alpha 0x80).
fn mix(tri: &Tri, i: usize, j: usize, t: i32) -> (u32, u32) {
    let (a, b) = (tri.uv(i), tri.uv(j));
    let uv = (lerp(a[0], b[0], t) & 0xffff) | (lerp(a[1], b[1], t) << 16);
    let byte = |c: u32, k: u32| (c >> (8 * k)) & 0xff;
    let mut col = 0x8000_0000;
    for k in 0..3 {
        col |= (lerp(byte(tri.col[i], k), byte(tri.col[j], k), t) & 0xff) << (8 * k);
    }
    (uv, col)
}

/// An actor's model clipped against a triangle of the screen (internal:
/// `ecx` = the actor `C`, `edx` = a matrix `W`, the stack `(_, out, &vb,
/// &first, &second, &f1, &f2)`; a /GS frame). `C`'s model is `0x10fb`,
/// `0x10fc`, `0x10fd` or `0x1100` ("special") or not.
///
/// - `A` = slot 562; slots 357 `(C, -1, 0x3c00)`, 356 `(C, -1, 0x2ee0)`,
///   362 `(A, C, 1, 0)`; a special model copies `A` (slot 363 `(C, 1,
///   1)` bytes, `memcpy`) to `G_BUF` and sets `G_SET`;
/// - the vertices: [`parts_vertices`](crate::s5_misc::parts_vertices)
///   `(A, W)` into `vb` with the box `lo`/`hi` (frame `ebp - 0x28`,
///   `ebp - 0x18`), colours at `A + 0xa0000`. Not special: the parts
///   drawn ([`model_parts_draw`](crate::s5_misc::model_parts_draw) with
///   `G_MTX`), the box replaced by slot 924 `(C, hi, lo)` (models 0x1054
///   and 0x1026 and `0x112b`: kind 2 or 1) or slot 934 `(C, ebp - 0x68,
///   ebp - 0x78, hi, lo)` whose box is scaled (z by `K1`, x and y by `K2`,
///   the low and high corners swapped);
/// - the region: `h = (hi.y - lo.y) K3`, `s = (slot 404 (h K4) + K6) + h
///   K5`, `mid = lo.y + h`: the band `mid - s .. mid + s`, the sides
///   `lo.x`, `hi.x`. The tables `T + 16 i` (`i = *G_IDX`) get the box's
///   centre and 1, `-(top - bottom)`, `right - left`, 0, 0; `*f1 = (h K3 +
///   lo.y) K7`, `*f2 = mid`;
/// - each part's strips (slot 941 of its first word: `D`, `D + D[1]`,
///   the i16 `D + 0x10` triangles in all) are copied to two lists (the
///   first at `out + 0x20`, the second at `A + 0x50000`, 32-byte headers
///   whose u16 `+2` becomes the count each list got). Each triangle of a
///   kind-0x10 strip (its record 0x1c bytes, colours 12) goes whole to
///   the first list (above the band, or inside the region), to the second
///   (below the band, or outside), or is split where two of its edges
///   cross the region's diagonal: two new vertices appended to `vb` and
///   three triangles, the two on the lone vertex's other side in one list
///   and the third in the other;
/// - the end: `out`'s header (u16 `+0x10` the first list's count, `+4`
///   0x20) and `*first = out`; the second list's header after the first
///   and `*second` = it, the second list copied behind it; `*vb` = the
///   end of the vertices; slot 327 `(W, W)` and slot 229 `(v, W, v)` for
///   each vertex. Returns the end of the copy.
///
/// `magic_0434` 0x10001400 (9159 bytes; also `magic_0610`).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn clip_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0xb7), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let (actor, wm) = (fr.ecx, fr.edx);
    let out = fr.arg(g, 1);
    let pvb = fr.arg(g, 2);
    let (p_first, p_second, p_f1, p_f2) = (fr.arg(g, 3), fr.arg(g, 4), fr.arg(g, 5), fr.arg(g, 6));
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let id = g.read::<u16>(actor);
    let special = matches!(id, 0x10fb..=0x10fd | 0x1100);
    // call width: 0
    let a = g.call(slot(g, 562), 0, &[]).eax;
    // call width: 3
    g.call(slot(g, 357), 0, &[actor, u32::MAX, 0x3c00]);
    // call width: 3
    g.call(slot(g, 356), 0, &[actor, u32::MAX, 0x2ee0]);
    // call width: 4
    g.call(slot(g, 362), 0, &[a, actor, 1, 0]);
    if special {
        // call width: 3
        let n = g.call(slot(g, 363), 0, &[actor, 1, 1]).eax;
        let mut buf = vec![0u8; n as usize];
        g.read_bytes(a, &mut buf);
        g.write_bytes(w(0x133), &buf);
        g.write::<u32>(w(0x141), 1);
    }
    let (lo, hi) = (l(0x28), l(0x18));
    let vb = g.read::<u32>(pvb);
    let colours = a.wrapping_add(0xa_0000);
    // the five pushes of the call (the fourth word is one the callee never
    // reads: the original's leftover ecx; 0 here)
    let sp = l(0x198);
    for (i, v) in (0u32..).zip([vb, lo, hi, 0, colours]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let pv = Frame {
        at: img.rel32_target(f + 0x19c),
        esp: sp.wrapping_sub(4),
        eax: 0,
        ecx: a,
        edx: wm,
        xmm: [0; 4],
    };
    let nv = crate::s5_misc::parts_vertices(g, img, &pv);
    let mut fresh = vb.wrapping_add(nv << 4);
    let mut count = nv;
    if !special {
        let draw = img.rel32_target(f + 0x247);
        crate::s5_misc::model_parts_draw(g, img, draw, l(0x188), actor, w(0x240));
        if matches!(id, 0x1054 | 0x1026 | 0x112b) {
            // call width: 5
            g.call(slot(g, 934), 0, &[actor, l(0x68), l(0x78), hi, lo]);
        } else {
            // call width: 3
            g.call(slot(g, 924), 0, &[actor, hi, lo]);
        }
        // either box scaled: z by K1, x and y by K2 (the corners swapped)
        let z0 = mul(rf(g, l(0x20)), k(g, 0x298));
        g.write::<f32>(l(0x20), z0);
        let z1 = mul(rf(g, l(0x10)), k(g, 0x2c6));
        let k2 = k(g, 0x2a0);
        let (lx, ly, hx, hy) = (rf(g, lo), rf(g, lo + 4), rf(g, hi), rf(g, hi + 4));
        g.write::<f32>(l(0x10), z1);
        g.write::<f32>(lo, mul(hx, k2));
        g.write::<f32>(hi, mul(lx, k2));
        g.write::<f32>(lo + 4, mul(hy, k2));
        g.write::<f32>(hi + 4, mul(ly, k2));
    }
    // the region
    let (lx, ly, hx, hy) = (rf(g, lo), rf(g, lo + 4), rf(g, hi), rf(g, hi + 4));
    let h = mul(sub(hy, ly), k(g, 0x313));
    // call width: 1
    let st = st0_f32(
        g.call(slot(g, 404), 0, &[mul(h, k(g, 0x31e)).to_bits()])
            .st0,
    );
    let s = add(add(st, k(g, 0x358)), mul(h, k(g, 0x34b)));
    let mid = add(ly, h);
    let k3 = k(g, 0x396);
    let r = Region {
        top: sub(mid, s),
        bottom: add(mid, s),
        left: lx,
        right: hx,
        cc: sub(sub(mid, s), add(mid, s)),
    };
    let idx = g.read::<u32>(w(0x367)) << 4;
    let centre = [
        mul(add(hx, lx), k3),
        mul(add(hy, ly), k3),
        mul(add(rf(g, l(0x10)), rf(g, l(0x20))), k3),
    ];
    for (o, v) in [0x3aeu32, 0x3dd, 0x428].into_iter().zip(centre) {
        g.write::<f32>(w(o).wrapping_add(idx), v);
    }
    g.write::<u32>(w(0x435).wrapping_add(idx), 0x3f80_0000);
    let neg = r.cc.to_bits() ^ g.read::<u32>(w(0x456));
    g.write::<u32>(w(0x45e).wrapping_add(idx), neg);
    g.write::<f32>(w(0x46d).wrapping_add(idx), sub(hx, lx));
    g.write::<u32>(w(0x47f).wrapping_add(idx), 0);
    g.write::<u32>(w(0x489).wrapping_add(idx), 0);
    let f1 = mul(add(mul(h, k3), ly), k(g, 0x4ab));
    g.write::<f32>(p_f1, f1);
    g.write::<f32>(p_f2, mid);
    // the parts
    let second_base = a.wrapping_add(0x5_0000);
    let (mut first, mut second) = (out.wrapping_add(0x20), second_base);
    let (mut first_total, mut second_total) = (0u32, 0u32);
    let mut cols = colours;
    let ops = [
        EdgeOps {
            zero: Some(0xd55),
            one: 0xd75,
            t1: 0xdd5,
            w1: 0xdf7,
            abs: (0xd88, 0xd93),
        },
        EdgeOps {
            zero: Some(0x1057),
            one: 0x109a,
            t1: 0x10c5,
            w1: 0x10e7,
            abs: (0x107d, 0x1085),
        },
        EdgeOps {
            zero: None,
            one: 0x134c,
            t1: 0x137c,
            w1: 0x139e,
            abs: (0x132f, 0x1337),
        },
    ];
    let parts = u32::from(g.read::<u16>(a + 4));
    for j in 0..parts {
        let pp = a.wrapping_add(0x10 + 16 * j);
        let pw = g.read::<u32>(pp);
        // call width: 1
        let d = g.call(slot(g, 941), 0, &[pw]).eax;
        let mut strip = d.wrapping_add(g.read::<u32>(d + 4));
        let mut rem = i32::from(g.read::<i16>(d + 0x10));
        while rem > 0 {
            let mut hdr = [0u8; 0x20];
            g.read_bytes(strip, &mut hdr);
            g.write_bytes(first, &hdr);
            g.write_bytes(second, &hdr);
            let (h1, h2) = (first, second);
            first = first.wrapping_add(0x20);
            second = second.wrapping_add(0x20);
            let mut rec = strip.wrapping_add(0x20);
            let (mut n1, mut n2) = (0u32, 0u32);
            if g.read::<u8>(strip + 1) == 0x10 {
                let mut i = 0i32;
                while i < i32::from(g.read::<i16>(strip + 2)) {
                    let v = |g: &G, o: u32| {
                        vtx(g, vb.wrapping_add(u32::from(g.read::<u16>(rec + o)) << 4))
                    };
                    let (va, vbb, vc) = (v(g, 0), v(g, 2), v(g, 4));
                    let tri = Tri::read(g, cols, rec);
                    match classify(r, [va, vbb, vc]) {
                        Class::First => {
                            tri.write(g, first);
                            first = first.wrapping_add(0x20);
                            n1 += 1;
                        }
                        Class::Second => {
                            tri.write(g, second);
                            second = second.wrapping_add(0x20);
                            n2 += 1;
                        }
                        Class::Split(inside) => {
                            let ab = edge(g, img, f, r, va, vbb, &ops[0]);
                            let bc = edge(g, img, f, r, vbb, vc, &ops[1]);
                            let ca = edge(g, img, f, r, vc, va, &ops[2]);
                            // exactly two crossings: the lone vertex
                            // (between the two edges that cross), the two
                            // new vertices in the order the original
                            // appends them, and the K1024 operands of
                            // their `t`
                            let pick = match (ab, bc, ca) {
                                (None, Some(bc), Some(ca)) => {
                                    Some((2usize, bc, ca, 0x1428, 0x1478))
                                }
                                (Some(ab), None, Some(ca)) => Some((0, ca, ab, 0x193f, 0x18e1)),
                                (Some(ab), Some(bc), None) => Some((1, ab, bc, 0x18e1, 0x1dad)),
                                _ => None,
                            };
                            if let Some((lone, e0, e1, k0, k1)) = pick {
                                let t0 = cvtt(mul(e0.4, k(g, k0)));
                                let t1 = cvtt(mul(e1.4, k(g, k1)));
                                for (o, e) in [(0u32, e0), (0x10, e1)] {
                                    let at = fresh.wrapping_add(o);
                                    g.write::<f32>(at, e.0);
                                    g.write::<f32>(at + 4, e.1);
                                    g.write::<f32>(at + 8, e.2);
                                    g.write::<f32>(at + 0xc, e.3);
                                }
                                let (n0, n1v) = (count, count.wrapping_add(1));
                                // the edges' ends: (from, to) for each new
                                // vertex, and the three triangles
                                let (pair, recs) = split(lone);
                                let m0 = mix(&tri, pair[0].0, pair[0].1, t0);
                                let m1 = mix(&tri, pair[1].0, pair[1].1, t1);
                                let mk = |set: &[(usize, usize)]| {
                                    let mut t = tri;
                                    for &(at, which) in set {
                                        let (n, (uv, col)) =
                                            if which == 0 { (n0, m0) } else { (n1v, m1) };
                                        t.set(at, n, uv, col);
                                    }
                                    t
                                };
                                let (r1, r2, r3) = (mk(recs[0]), mk(recs[1]), mk(recs[2]));
                                // the vertex after the lone one decides which
                                // list takes the two
                                let side = inside[(lone + 1) % 3];
                                let (two, one) = if side {
                                    (&mut first, &mut second)
                                } else {
                                    (&mut second, &mut first)
                                };
                                r1.write(g, *two);
                                r2.write(g, two.wrapping_add(0x20));
                                *two = two.wrapping_add(0x40);
                                r3.write(g, *one);
                                *one = one.wrapping_add(0x20);
                                if side {
                                    n1 += 2;
                                    n2 += 1;
                                } else {
                                    n1 += 1;
                                    n2 += 2;
                                }
                                count = count.wrapping_add(2);
                                fresh = fresh.wrapping_add(0x20);
                            } else {
                                tri.write(g, first);
                                first = first.wrapping_add(0x20);
                                n1 += 1;
                            }
                        }
                    }
                    i += 1;
                    rec = rec.wrapping_add(0x1c);
                    cols = cols.wrapping_add(0xc);
                }
            }
            first_total = first_total.wrapping_add(n1);
            g.write::<u16>(h1 + 2, n1 as u16);
            second_total = second_total.wrapping_add(n2);
            g.write::<u16>(h2 + 2, n2 as u16);
            rem = rem.wrapping_sub(i32::from(g.read::<i16>(strip + 2)));
            strip = rec;
        }
    }
    // the end
    g.write::<u16>(out + 0x10, first_total as u16);
    g.write::<u32>(out + 4, 0x20);
    g.write::<u32>(p_first, out);
    g.write::<u16>(first + 0x10, second_total as u16);
    g.write::<u32>(first + 4, 0x20);
    let mut end = first.wrapping_add(0x20);
    let words = second.wrapping_sub(second_base).cast_signed() / 4;
    for i in 0..words.max(0).cast_unsigned() {
        let v = g.read::<u32>(second_base.wrapping_add(4 * i));
        g.write::<u32>(end, v);
        end = end.wrapping_add(4);
    }
    g.write::<u32>(p_second, first);
    g.write::<u32>(pvb, fresh);
    // call width: 2
    g.call(slot(g, 327), 0, &[wm, wm]);
    for i in 0..count.cast_signed().max(0).cast_unsigned() {
        let v = vb.wrapping_add(i << 4);
        // call width: 3
        g.call(slot(g, 229), 0, &[v, wm, v]);
    }
    end
}

/// Where a triangle goes: whole to the first or second list, or split
/// (`inside` per vertex, the region tests).
enum Class {
    First,
    Second,
    Split([bool; 3]),
}

/// Above the band (`top >= y`) all three: the first list; below (`y >=
/// bottom`) all three: the second. Otherwise each vertex not above is
/// tested against the region's edges (the top, the diagonal from `(right,
/// top)` to `(left, bottom)`, the left side; the original's products,
/// `top - top` and `left - left` included): all inside, the first list;
/// none, the second; else split.
fn classify(r: Region, v: [Vtx; 3]) -> Class {
    let up = v.map(|p| r.top >= p.y);
    let dn = v.map(|p| p.y >= r.bottom);
    if dn.iter().all(|&x| x) {
        return Class::Second;
    }
    if up.iter().all(|&x| x) {
        return Class::First;
    }
    let mut inside = [true; 3];
    for i in 0..3 {
        if up[i] {
            continue;
        }
        let (x, y) = (v[i].x, v[i].y);
        let d = sub(y, r.top);
        let s1 = sub(
            mul(sub(x, r.left), sub(r.top, r.top)),
            mul(sub(r.right, r.left), d),
        );
        if s1 > 0.0 {
            inside[i] = false;
            continue;
        }
        let s2 = sub(
            mul(sub(x, r.right), sub(r.bottom, r.top)),
            mul(sub(r.left, r.right), d),
        );
        if s2 > 0.0 {
            inside[i] = false;
            continue;
        }
        let s3 = sub(
            mul(sub(x, r.left), r.cc),
            mul(sub(r.left, r.left), sub(y, r.bottom)),
        );
        // setbe: at most 0, or unordered
        inside[i] = s3.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater);
    }
    if inside[0] {
        if inside[1] && inside[2] {
            return Class::First;
        }
    } else if !inside[1] && !inside[2] {
        return Class::Second;
    }
    Class::Split(inside)
}

/// The split around the lone vertex `lone`: the two new vertices' edges
/// (vertex indices from, to: their `t` runs from the first) and the three
/// triangles as (corner, new vertex 0 or 1) replacements, in the order
/// the original writes them (the first two to one list, the third to the
/// other).
type Recs = [&'static [(usize, usize)]; 3];

fn split(lone: usize) -> ([(usize, usize); 2], Recs) {
    match lone {
        // A alone (AB and CA cross): new 0 = CA, new 1 = AB;
        // (CA, B, C), (AB, B, CA) | (A, AB, CA)
        0 => (
            [(2, 0), (0, 1)],
            [&[(0, 0)], &[(0, 1), (2, 0)], &[(1, 1), (2, 0)]],
        ),
        // B alone (AB and BC cross): new 0 = AB, new 1 = BC;
        // (A, AB, C), (AB, BC, C) | (AB, B, BC)
        1 => (
            [(0, 1), (1, 2)],
            [&[(1, 0)], &[(0, 0), (1, 1)], &[(0, 0), (2, 1)]],
        ),
        // C alone (BC and CA cross): new 0 = BC, new 1 = CA;
        // (A, B, BC), (A, BC, CA) | (CA, BC, C)
        _ => (
            [(1, 2), (2, 0)],
            [&[(2, 0)], &[(1, 0), (2, 1)], &[(0, 1), (1, 0)]],
        ),
    }
}

/// [`clip_0434`] (the `rel32`s: `memcpy`,
/// [`parts_vertices`](crate::s5_misc::parts_vertices),
/// [`model_parts_draw`](crate::s5_misc::model_parts_draw) and two
/// `__security_check_cookie`s).
pub const CLIP_0434: Sig = Sig {
    len: 9159,
    hash: 0x89c0_362b_7bc5_c7ed,
    calls: &[
        (0x138, Callee::Import("memcpy")),
        (0x19c, Callee::Sig(&crate::s5_misc::PARTS_VERTICES)),
        (0x247, Callee::Sig(&crate::s5_misc::MODEL_PARTS_DRAW)),
        (0x23ad, Callee::Any),
        (0x23bf, Callee::Any),
    ],
    imports: &[],
};

/// The signed remainder of `rand()` by `n` (`cdq; idiv`).
fn rand_mod<G: Game>(g: &mut G, n: i32) -> i32 {
    g.rand().cast_signed() % n
}

/// A summon's sparks laid on its model (cdecl, no arguments; an aligned
/// /GS frame `B`, the matrices `M` (`B + 0x68`) and `M2` (`B + 0xa8`), the
/// vectors `V` (`B + 0x48`) and `W` (`B + 0x58`)). `S = *G_S`, its angle
/// `a = S + 0x5ee8`; the model `R` = slot 719 `(NAME, 4, 0)`, its strips
/// at `R + R[1]` (the i16 `R + 0x10` records in all; a strip's i8 `+1`
/// kind, i16 `+2` count, 0x1c-byte records from `+0x10`, the u16 `+0xc`
/// a vertex: three i16 at `R + R[2] + 6 n`):
/// - the vertices' z range over the kind-1 strips (`lo`, `hi`, from
///   50000 and -50000);
/// - `M` turned by `a` (slots 235, 329), `V = M (0, 0, 70, 0)` (slot
///   229); `a1 = a + pi` wrapped; `M` = 0.01 scaled, turned by `a1`;
/// - for each record of each kind-1 strip a spark `P` (0xc8 bytes from
///   `S + 0xa00`): the vertex's x and z (y 0, w 1), its height share
///   (`rand % 3 - (z - lo) / (hi - lo) -10`, truncated) at `+0xac`; `P`
///   moved by `M` (slot 229); its angle around `V` (`3pi/2 -
///   atan2(dz, dx)`, wrapped) turns `M2`; `W = M2 (0, 0, -(slot 314
///   (1.25) + 1.25), 1)`; slot 314 and slot 404 values for its angles,
///   spins, scales and colours (three sets by `rand % 3`), random
///   counts and directions (`rand % 2`), its speed from `W` and slot 404
///   (4.5).
///
/// Returns the last strip's i16 count (slot 329's `eax` when there are
/// no strips).
///
/// `magic_0165` 0x10001bd0 (2513 bytes; also `magic_0606`).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_0165<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x26), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let fcall = |g: &mut G, n: u32, x: u32| st0_f32(g.call(slot(g, n), 0, &[x]).st0);
    let cvt16 = |g: &G, at: u32| crate::sb_sse::cvt(i32::from(g.read::<i16>(at)).cast_unsigned());
    // and esp, -8; sub esp, 0xe8; push esi; push edi
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xf0);
    let (m, v, m2, wv) = (b + 0x68, b + 0x48, b + 0xa8, b + 0x58);
    let s = g.read::<u32>(w(0x1b));
    // call width: 3
    let r = g.call(slot(g, 719), 0, &[w(0x35), 4, 0]).eax;
    let verts = r.wrapping_add(g.read::<u32>(r + 8));
    let first = r.wrapping_add(g.read::<u32>(r + 4));
    let vertex = |g: &G, rec: u32| verts.wrapping_add(6 * u32::from(g.read::<u16>(rec + 0xc)));
    // the z range
    let (mut lo, mut hi) = (k(g, 0x49), k(g, 0x51));
    let mut rem = i32::from(g.read::<i16>(r + 0x10));
    let mut strip = first;
    while rem > 0 {
        let mut rec = strip.wrapping_add(0x10);
        if g.read::<i8>(strip + 1) == 1 {
            for _ in 0..i32::from(g.read::<i16>(strip + 2)).max(0) {
                let z = cvt16(g, vertex(g, rec) + 4);
                if lo > z {
                    lo = z;
                }
                if z > hi {
                    hi = z;
                }
                rec = rec.wrapping_add(0x1c);
            }
        }
        rem = rem.wrapping_sub(i32::from(g.read::<i16>(strip + 2)));
        strip = rec;
    }
    let range = sub(hi, lo);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    let a = rf(g, s + 0x5ee8);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, a.to_bits()]);
    for (i, x) in (0u32..).zip([0u32, 0, 0x428c_0000, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, v]);
    let a1 = add(rf(g, s + 0x5ee8), k(g, 0x159));
    let a1 = if a1 > k(g, 0x175) {
        sub(a1, k(g, 0x161))
    } else if k(g, 0x195) > a1 {
        add(a1, k(g, 0x161))
    } else {
        a1
    };
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    for o in [0u32, 0x14, 0x28] {
        g.write::<u32>(m + o, 0x3c23_d70a);
    }
    // call width: 3
    let mut eax = g.call(slot(g, 329), 0, &[m, m, a1.to_bits()]).eax;
    let mut rem = i32::from(g.read::<i16>(r + 0x10));
    let mut strip = first;
    let mut p = s + 0xa00;
    while rem > 0 {
        let mut rec = strip.wrapping_add(0x10);
        if g.read::<i8>(strip + 1) == 1 {
            let mut i = 0i32;
            while i < i32::from(g.read::<i16>(strip + 2)) {
                let vx = vertex(g, rec);
                g.write::<u32>(p + 4, 0);
                g.write::<f32>(p, cvt16(g, vx));
                g.write::<u32>(p + 0xc, 0x3f80_0000);
                g.write::<f32>(p + 8, cvt16(g, vx + 4));
                let r3 = rand_mod(g, 3);
                let share = cvtt(mul(div(sub(rf(g, p + 8), lo), range), k(g, 0x289)));
                g.write::<i32>(p + 0xac, r3.wrapping_sub(share));
                // call width: 3
                g.call(slot(g, 229), 0, &[p, m, p]);
                let dz = sub(rf(g, p + 8), rf(g, v + 8));
                let dx = sub(rf(g, p), rf(g, v));
                #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
                let a2 = sub(k(g, 0x2f1), ang);
                let a2 = if a2 > k(g, 0x300) {
                    sub(a2, k(g, 0x310))
                } else if k(g, 0x31a) > a2 {
                    add(a2, k(g, 0x327))
                } else {
                    a2
                };
                // call width: 1
                g.call(slot(g, 235), 0, &[m2]);
                // call width: 3
                g.call(slot(g, 329), 0, &[m2, m2, a2.to_bits()]);
                g.write::<u32>(wv, 0);
                g.write::<u32>(wv + 4, 0);
                let r1 = fcall(g, 314, 0x3fa0_0000);
                let nz = add(r1, k(g, 0x39c)).to_bits() ^ g.read::<u32>(w(0x3b2));
                g.write::<u32>(wv + 8, nz);
                g.write::<u32>(wv + 0xc, 0x3f80_0000);
                // call width: 3
                g.call(slot(g, 229), 0, &[wv, m2, wv]);
                g.write::<u32>(p + 0x10, 0);
                let t = fcall(g, 314, 0x3ea0_d97c);
                g.write::<u32>(p + 0x18, 0);
                g.write::<f32>(p + 0x14, add(t, a1));
                for o in [0x60u32, 0x64, 0x68] {
                    let t = fcall(g, 314, 0x4049_0fdb);
                    g.write::<f32>(p + o, t);
                }
                for o in [0x70u32, 0x74] {
                    let t = fcall(g, 314, 0x3d00_adfd);
                    g.write::<f32>(p + o, t);
                }
                g.write::<u32>(p + 0x78, 0);
                for (o, c) in [(0x90u32, 0x4ad), (0x94, 0x4db)] {
                    let t = fcall(g, 404, 0x3f80_0000);
                    g.write::<f32>(p + o, add(t, k(g, c)));
                }
                for o in [0x98u32, 0x9c] {
                    let t = fcall(g, 314, 0x3c23_d70a);
                    g.write::<f32>(p + o, t);
                }
                let c = rand_mod(g, 17) + 0x40;
                g.write::<i32>(p + 0xb8, c);
                let c = rand_mod(g, 17) + 0x20;
                g.write::<i32>(p + 0xbc, c);
                let set = rand_mod(g, 3);
                g.write::<i32>(p + 0xc0, set);
                // three colours: slot 314's argument and each one's operands
                let (x, ops): (u32, [[u32; 3]; 3]) = match set {
                    0 => (
                        0x3f00_0000,
                        [
                            [0x6bf, 0x6d3, 0x6db],
                            [0x6fd, 0x711, 0x719],
                            [0x73b, 0x743, 0x74b],
                        ],
                    ),
                    1 => (
                        0x3f00_0000,
                        [
                            [0x62b, 0x63f, 0x647],
                            [0x669, 0x67d, 0x685],
                            [0x603, 0x60b, 0x74b],
                        ],
                    ),
                    _ => (
                        0x3dcc_cccd,
                        [
                            [0x587, 0x59b, 0x5a3],
                            [0x5c5, 0x5d9, 0x5e1],
                            [0x603, 0x60b, 0x74b],
                        ],
                    ),
                };
                for (o, [ka, kb, kc]) in [0x80u32, 0x84, 0x88].into_iter().zip(ops) {
                    let t = fcall(g, 314, x);
                    let c = mul(mul(add(t, k(g, ka)), k(g, kb)), k(g, kc));
                    g.write::<f32>(p + o, c);
                }
                let t = fcall(g, 314, 0x3e80_0000);
                let sc = add(mul(t, k(g, 0x77d)), k(g, 0x791));
                for (o, ka, kb) in [(0x40u32, 0x7b1, 0x7c5), (0x44, 0x7e4, 0x7f6)] {
                    let t = fcall(g, 404, 0x3dcc_cccd);
                    g.write::<f32>(p + o, add(mul(t, k(g, ka)), k(g, kb)));
                }
                g.write::<u32>(p + 0x48, 0);
                g.write::<u32>(p + 0x4c, 0);
                for (o, kn) in [(0x40u32, 0x81f), (0x44, 0x841)] {
                    if rand_mod(g, 2) == 0 {
                        let t = mul(rf(g, p + o), k(g, kn));
                        g.write::<f32>(p + o, t);
                    }
                }
                let d0 = mul(mul(rf(g, p + 0x40), k(g, 0x853)), k(g, 0x861));
                g.write::<u32>(p + 0x58, 0);
                g.write::<u32>(p + 0x5c, 0);
                g.write::<f32>(p + 0x50, d0);
                let d1 = mul(mul(rf(g, p + 0x44), k(g, 0x881)), k(g, 0x890));
                g.write::<f32>(p + 0x54, d1);
                let x20 = mul(rf(g, wv), sc);
                g.write::<f32>(p + 0x20, x20);
                let t = fcall(g, 404, 0x4090_0000);
                let ny = f32::from_bits(add(t, k(g, 0x8c0)).to_bits() ^ g.read::<u32>(w(0x8d8)));
                let z28 = mul(rf(g, wv + 8), sc);
                let k3 = k(g, 0x8e8);
                g.write::<f32>(p + 0x24, ny);
                g.write::<f32>(p + 0x28, z28);
                g.write::<u32>(p + 0x2c, 0x3f80_0000);
                g.write::<f32>(p + 0x30, mul(mul(x20, k(g, 0x8e0)), k3));
                g.write::<f32>(p + 0x34, mul(mul(ny, k(g, 0x8f5)), k3));
                g.write::<f32>(p + 0x38, mul(mul(z28, k(g, 0x902)), k3));
                g.write::<u32>(p + 0x3c, 0x3f80_0000);
                let c = rand_mod(g, 17);
                g.write::<i32>(p + 0xc4, c);
                let life = rand_mod(g, 21) + 0x2d;
                g.write::<i32>(p + 0xa8, life);
                let d = rand_mod(g, 11) + 0x14;
                let life = life.wrapping_add(d);
                g.write::<i32>(p + 0xb4, life);
                g.write::<i32>(p + 0xb0, life);
                g.write::<i32>(p + 0xa4, d);
                g.write::<i32>(p + 0xa0, d);
                rec = rec.wrapping_add(0x1c);
                p = p.wrapping_add(0xc8);
                i += 1;
            }
        }
        let n = i32::from(g.read::<i16>(strip + 2));
        eax = n.cast_unsigned();
        rem = rem.wrapping_sub(n);
        strip = rec;
    }
    eax
}

/// [`sparks_0165`] (`rand` through its import; the `rel32`s: `_CIatan2`
/// and `__security_check_cookie`).
pub const SPARKS_0165: Sig = Sig {
    len: 2513,
    hash: 0x94c5_9e12_b0e0_1696,
    calls: &[(0x2df, Callee::Import("_CIatan2")), (0x9c9, Callee::Any)],
    imports: &[(0x206, "rand")],
};

/// A summon's start-up with its static nodes (cdecl `(actor)`, an aligned
/// frame `A = (ebp & !7)`; `magic_0152` 0x1000b860 and `magic_0533`):
/// the targets ([`targets_init_c`](crate::s4_summon::targets_init_c) of
/// the actor), the summon's files (slots 718, 719, 711 with the load
/// flag `*G_F` set to 1 around it, 710, 222, 221, 551), the root lists
/// and static nodes (the root `R = G_ROOT`: each node appended last to
/// `R`'s children, as [`static_append`]), slot 531, 325, 215, 290 and 601
/// calls between them (601's angle `a`: `G_N = (3pi/2 - a) 32768 / pi`,
/// truncated), the node [`node_init`](crate::m0042::node_init)
/// `(G_NODE, R, *G_K, 0x6b)`, slots 27 and 4 for two sounds, slot 281
/// `(1)`, slot 694 `(1, 0, 0, 0)` and two links
/// ([`static_node_link_b`](crate::s4_small::static_node_link_b)).
/// Returns 0.
///
/// `magic_0152` 0x1000b860 (3658 bytes; also `magic_0533`).
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn start_up_0152<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x13), n);
    // `mov dword [op], op` / `[op], imm`
    let st = |g: &mut G, pairs: &[(u32, u32)]| {
        for &(a, v) in pairs {
            g.write::<u32>(w(a), w(v));
        }
    };
    let imm = |g: &mut G, pairs: &[(u32, u32)]| {
        for &(a, v) in pairs {
            g.write::<u32>(w(a), v);
        }
    };
    let root = w(0x2cb);
    // a static node appended last to the root's children (its u16 pass
    // operand `pass`, its handler table `vt` where the original stores it
    // with the links)
    let append = |g: &mut G, pass: u32, vt: Option<u32>| {
        let node = w(pass).wrapping_sub(0x1a);
        static_append(g, node, root);
        if let Some(v) = vt {
            g.write::<u32>(node, w(v));
        }
    };
    let a8 = fr.esp.wrapping_sub(4) & !7;
    let actor = fr.arg(g, 0);
    crate::s4_summon::targets_init_c(
        g,
        img,
        img.rel32_target(f + 0xe),
        a8.wrapping_sub(0x1c),
        actor,
    );
    st(g, &[(0x24, 0x28)]);
    // call width: 1
    g.call(slot(g, 718), 0, &[w(0x18)]);
    // call width: 1
    g.call(slot(g, 718), 0, &[w(0x3d)]);
    // call width: 3
    let res = g.call(slot(g, 719), 0, &[w(0x56), 8, 0]).eax;
    let flag = w(0x5e);
    let saved = g.read::<u32>(g.read::<u32>(flag));
    g.write::<u32>(w(0x6a), res);
    let at = g.read::<u32>(flag);
    g.write::<u32>(at, 1);
    // call width: 3
    g.call(slot(g, 711), 0, &[res, w(0x6f), 0]);
    let at = g.read::<u32>(w(0x88));
    g.write::<u32>(at, saved);
    let words = g.read::<u32>(w(0xa8));
    // call width: 4
    g.call(slot(g, 710), 0, &[w(0xad), words, w(0xa2), 0x10_0000]);
    // call width: 4
    g.call(slot(g, 222), 0, &[w(0xc5), 0, 0xb, 2]);
    // call width: 4
    g.call(slot(g, 221), 0, &[w(0xe0), 0, 0x3e8, 2]);
    for (o, part, n) in [(0xf6u32, 6u32, 0u32), (0x124, 7, 1)] {
        // call width: 3
        let r = g.call(slot(g, 719), 0, &[w(o), 6, part]).eax;
        // call width: 3
        g.call(slot(g, 551), 0, &[r, n, 2]);
    }
    // the root lists and their first nodes
    st(g, &[(0x149, 0x14d)]);
    let list = g.read::<u32>(w(0x158));
    st(
        g,
        &[
            (0x15e, 0x162),
            (0x168, 0x16c),
            (0x172, 0x176),
            (0x186, 0x18a),
            (0x190, 0x194),
        ],
    );
    st(g, &[(0x19a, 0x19e), (0x1a4, 0x1a8), (0x1b8, 0x1bc)]);
    imm(g, &[(0x17c, 1), (0x1ae, 1)]);
    g.write::<u16>(w(0x1c3), 0x1ff);
    st(g, &[(0x1cb, 0x1cf), (0x1d5, 0x1d9), (0x1e9, 0x1ed)]);
    imm(g, &[(0x1df, 0)]);
    g.write::<u16>(w(0x1f4), 0x1ff);
    st(g, &[(0x1fc, 0x200), (0x206, 0x20a), (0x21a, 0x21e)]);
    imm(g, &[(0x210, 0)]);
    g.write::<u16>(w(0x225), 0xff);
    st(g, &[(0x22d, 0x231), (0x237, 0x23b), (0x24b, 0x24f)]);
    imm(g, &[(0x241, 0)]);
    g.write::<u16>(w(0x256), 0xff);
    st(g, &[(0x25e, 0x262), (0x268, 0x26c)]);
    imm(g, &[(0x272, 0)]);
    // call width: 2
    g.call(slot(g, 531), 0, &[list, w(0x152)]);
    imm(g, &[(0x28c, 0x3f80_0000)]);
    append(g, 0x297, Some(0x300));
    // call width: 2
    g.call(slot(g, 325), 0, &[w(0x310), w(0x30b)]);
    let x = sub(k(g, 0x31a), k(g, 0x322));
    g.write::<f32>(w(0x345), x);
    append(g, 0x331, Some(0x3a2));
    // call width: 2
    g.call(slot(g, 325), 0, &[w(0x3b2), w(0x3ad)]);
    st(g, &[(0x3c2, 0x3c6)]);
    append(g, 0x3cd, None);
    let arg = g.read::<u32>(w(0x428));
    st(
        g,
        &[
            (0x438, 0x43c),
            (0x442, 0x446),
            (0x44c, 0x450),
            (0x456, 0x45a),
        ],
    );
    g.write::<u8>(w(0x460), 0);
    // call width: 1
    let r = g.call(slot(g, 215), 0, &[arg]).eax;
    g.write::<u32>(w(0x46e), r);
    append(g, 0x47d, Some(0x4f5));
    for (from, to) in [(0x4ce, 0x4dc), (0x4e4, 0x4fd)] {
        let q = g.read::<u64>(w(from));
        g.write::<u64>(w(to), q);
    }
    append(g, 0x504, Some(0x582));
    let list = g.read::<u32>(w(0x56e));
    // call width: 2
    g.call(slot(g, 290), 0, &[list, w(0x568)]);
    imm(g, &[(0x598, 0x3a83_126f)]);
    st(g, &[(0x5a2, 0x5a6)]);
    append(g, 0x5ad, None);
    // slot 601's angle into a frame word
    let local = a8.wrapping_sub(0x14);
    let list = g.read::<u32>(w(0x60d));
    imm(g, &[(0x61d, 0)]);
    // call width: 2
    g.call(slot(g, 601), 0, &[list, local]);
    let x = div(
        mul(sub(k(g, 0x631), g.read::<f32>(local)), k(g, 0x65f)),
        k(g, 0x67b),
    );
    imm(g, &[(0x640, 0)]);
    st(g, &[(0x64a, 0x64e)]);
    g.write::<i32>(w(0x698), cvtt(x));
    append(g, 0x655, None);
    st(
        g,
        &[
            (0x6dc, 0x6e0),
            (0x6e6, 0x6ea),
            (0x6f0, 0x6f4),
            (0x6fa, 0x6fe),
        ],
    );
    g.write::<u8>(w(0x704), 1);
    // the node's four words at `A - 0x10` (`ecx`)
    let block = a8.wrapping_sub(0x10);
    let kw = g.read::<u32>(w(0x6c1));
    for (i, v) in (0u32..).zip([w(0x70d), w(0x715), kw, 0x6b]) {
        g.write::<u32>(block + 4 * i, v);
    }
    crate::m0042::node_init(g, img, img.rel32_target(f + 0x726), block);
    st(g, &[(0x731, 0x735)]);
    append(g, 0x73c, None);
    for (from, to) in [(0x78a, 0x796), (0x79b, 0x7a0), (0x7a5, 0x7aa)] {
        let v = g.read::<u32>(w(from));
        g.write::<u32>(w(to), v);
    }
    st(g, &[(0x7b7, 0x7bb)]);
    append(g, 0x7c2, None);
    let b = u32::from(g.read::<u8>(w(0x81a)));
    st(
        g,
        &[
            (0x839, 0x83d),
            (0x843, 0x847),
            (0x84d, 0x851),
            (0x857, 0x85b),
        ],
    );
    g.write::<u8>(w(0x861), 0);
    // call width: 1
    let r = g.call(slot(g, 215), 0, &[b]).eax;
    g.write::<u32>(w(0x86f), r);
    append(g, 0x87e, Some(0x8de));
    imm(
        g,
        &[(0x8e4, 0), (0x8ee, 0), (0x8f8, 0), (0x902, 0x3f80_0000)],
    );
    append(g, 0x90d, Some(0x98d));
    let b = u32::from(g.read::<u8>(w(0x96e)));
    // call width: 2
    g.call(slot(g, 290), 0, &[b, w(0x973)]);
    imm(g, &[(0x9a3, 0x3a83_126f)]);
    st(g, &[(0x9ad, 0x9b1)]);
    append(g, 0x9b8, None);
    imm(g, &[(0xa14, 0), (0xa1e, 0), (0xa28, 0)]);
    st(g, &[(0xa32, 0xa36)]);
    append(g, 0xa3d, None);
    st(
        g,
        &[
            (0xaac, 0xab0),
            (0xab6, 0xaba),
            (0xac0, 0xac4),
            (0xaca, 0xace),
        ],
    );
    g.write::<u8>(w(0xad4), 1);
    sound(g, img, f, 0x300f);
    st(g, &[(0xafe, 0xb02)]);
    append(g, 0xb09, None);
    imm(g, &[(0xb65, 0), (0xb6f, 0), (0xb79, 0)]);
    append(g, 0xb84, Some(0xbf7));
    // call width: 2
    g.call(slot(g, 325), 0, &[w(0xc07), w(0xc02)]);
    let x = mul(k(g, 0xc11), k(g, 0xc19));
    g.write::<f32>(w(0xc29), x);
    append(g, 0xc30, Some(0xca0));
    let x = mul(k(g, 0xc81), k(g, 0xc89));
    imm(g, &[(0xca6, 0), (0xcb8, 0), (0xcc2, 0)]);
    g.write::<f32>(w(0xcb2), x);
    append(g, 0xccd, None);
    st(
        g,
        &[
            (0xd2c, 0xd30),
            (0xd36, 0xd3a),
            (0xd40, 0xd44),
            (0xd4a, 0xd4e),
        ],
    );
    append(g, 0xd55, None);
    st(
        g,
        &[
            (0xdc4, 0xdc8),
            (0xdce, 0xdd2),
            (0xdd8, 0xddc),
            (0xde2, 0xde6),
        ],
    );
    g.write::<u8>(w(0xdec), 1);
    sound(g, img, f, 0x3010);
    // call width: 1
    g.call(slot(g, 281), 0, &[1]);
    // call width: 4
    g.call(slot(g, 694), 0, &[1, 0, 0, 0]);
    for o in [0xe3au32, 0xe3f] {
        let link = Frame {
            at: img.rel32_target(f + o),
            esp: a8.wrapping_sub(0x1c),
            ..*fr
        };
        crate::s4_small::static_node_link_b(g, img, &link);
    }
    Ret::eax(0)
}

/// A sound started and loaded (slot 27 `(id, 1)`, slot 4 `(id)`).
fn sound<G: Game>(g: &mut G, img: &Image, f: u32, id: u32) {
    let at = |g: &G, n: u32| slot_target(g, img.word(f + 0x13), n);
    // call width: 2
    g.call(at(g, 27), 0, &[id, 1]);
    // call width: 1
    g.call(at(g, 4), 0, &[id]);
}

/// A static node appended last to the children of `parent`: pass u16
/// 0xff, its own empty child list, the parent, the previous last
/// (`parent + 0x10`) before it and it after that (or the parent's first,
/// `+0xc`, when the list was empty), the count u16 `+0x18` up.
fn static_append<G: Game>(g: &mut G, node: u32, parent: u32) {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    g.write::<u32>(node + 4, parent);
    let prev = g.read::<u32>(parent + 0x10);
    g.write::<u32>(node + 8, prev);
    if prev == parent {
        g.write::<u32>(parent + 0xc, node);
    } else {
        g.write::<u32>(prev + 4, node);
    }
    let n = g.read::<u16>(parent + 0x18).wrapping_add(1);
    g.write::<u16>(parent + 0x18, n);
    g.write::<u32>(parent + 0x10, node);
}

/// [`start_up_0152`] (the `rel32`s: the targets, the node set-up and the
/// two links).
pub const START_UP_0152: Sig = Sig {
    len: 3658,
    hash: 0x61f1_5849_48f2_982f,
    calls: &[
        (0xe, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C)),
        (0x726, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0xe3a, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK_B)),
        (0xe3f, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK_B)),
    ],
    imports: &[],
};

/// A trail drawn through the last points of an object (internal: `ecx` =
/// the object `O`, `edx` = its new point `V`, the stack `(&val)`; an
/// aligned /GS frame `B`, the `esp` after the pushes). Slot 772 `(O, 0)`
/// and 773 `(its eax, edx)` under the lock `*G_LOCK` (set to 1 and put
/// back; `flag` = 773 said 0), the first packet `PK0` reset (a 0x70000002
/// header), slot 382 `(0, 0)`. `V` and `*val` go into `O`'s two rings
/// (nine 16-byte entries from `O` and `O + 0x90`, the index `O + 0x128`,
/// the count `O + 0x12c` up to 9). From three points on:
/// - both packets `PK0`/`PK1` get 0x70000009 headers; slots 723, 724
///   (local matrices), 235 and twice 231 build a matrix in the frame;
/// - the rings, newest first, into two point rows; slot 562's buffer
///   holds a spline through each ([`spline_setup`](crate::s7_misc::spline_setup));
/// - slot 831 `(O, 0x80, 1)` with `flag`, else `(O, 16 n - 16, 0)`, under
///   the lock;
/// - for each step `k` of `16 n - 16`, the points at `(k + 1) / 16 n` of
///   both splines ([`spline_point`](crate::s6_misc::spline_point)), and a
///   packet (`PK1` for odd `k`) with the two colours `O + 0x120..0x126`
///   / 255 (the fourth byte times 1/128; the second colour's alpha the
///   byte `O + 0x127` faded by the steps left), the last points and these
///   (slot 832 `(O, packet, flag)` under the lock, slot 382 `(0, 0)`);
/// - slot 832 `(O, 0, flag)` under the lock and slot 382 `(0, 0)` to end.
///
/// Returns slot 382's `eax` (the count while there are fewer than three
/// points).
///
/// `magic_0432` 0x100014d0 (2137 bytes; also `magic_0434`, `magic_0610`).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn trail_0432<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let (o, v) = (fr.ecx, fr.edx);
    let val = fr.arg(g, 0);
    // and esp, -8; sub esp, 0x39c; three pushes
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x3a8);
    let lock = |g: &G, op: u32| g.read::<u32>(w(op));
    // call width: 2
    let r = g.call(slot(g, 772), 0, &[o, 0]);
    let l = lock(g, 0x37);
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    // call width: 2
    let r = g.call(slot(g, 773), 0, &[r.eax, r.edx]).eax;
    let l = lock(g, 0x54);
    g.write::<u32>(l, saved);
    let flag = u32::from(r == 0);
    for op in [0x6du32, 0x75, 0x87] {
        g.write::<u64>(w(op), 0);
    }
    for (op, x) in [
        (0x7bu32, 0x7000_0002u32),
        (0x8d, 0xe),
        (0x97, 0),
        (0xa1, 0x48),
        (0xab, 0x80),
        (0xb5, 0x42),
        (0xbf, 0),
    ] {
        g.write::<u32>(w(op), x);
    }
    // call width: 2
    g.call(slot(g, 382), 0, &[0, 0]);
    // the rings
    let i = g.read::<u32>(o + 0x128);
    let mut q = [0u8; 16];
    g.read_bytes(v, &mut q);
    g.write_bytes(o.wrapping_add(i << 4), &q);
    let i = g.read::<u32>(o + 0x128);
    g.read_bytes(val, &mut q);
    g.write_bytes(o.wrapping_add((i + 9) << 4), &q);
    let i = g.read::<i32>(o + 0x128);
    let next = if i + 1 >= 9 { 0 } else { i + 1 };
    g.write::<i32>(o + 0x128, next);
    let n = g.read::<i32>(o + 0x12c);
    if n < 9 {
        g.write::<i32>(o + 0x12c, n + 1);
    }
    let n = g.read::<i32>(o + 0x12c);
    if n < 3 {
        return n.cast_unsigned();
    }
    for op in [0x16eu32, 0x176, 0x17e, 0x190, 0x1a8, 0x1ba] {
        g.write::<u64>(w(op), 0);
    }
    for (op, x) in [
        (0x184u32, 0x7000_0009u32),
        (0x196, 0x50),
        (0x1a0, 0),
        (0x1ae, 0x7000_0009),
        (0x1c0, 0x50),
        (0x1ca, 0),
    ] {
        g.write::<u32>(w(op), x);
    }
    let (mat_a, mat_b, mat_m) = (b + 0x320, b + 0x360, b + 0x1c0);
    // call width: 1
    g.call(slot(g, 723), 0, &[mat_a]);
    // call width: 1
    g.call(slot(g, 724), 0, &[mat_b]);
    // call width: 1
    g.call(slot(g, 235), 0, &[mat_m]);
    // call width: 3
    g.call(slot(g, 231), 0, &[mat_m, mat_b, mat_m]);
    // call width: 3
    g.call(slot(g, 231), 0, &[mat_m, mat_a, mat_m]);
    let col: [u32; 7] = std::array::from_fn(|j| u32::from(g.read::<u8>(o + 0x120 + j as u32)));
    // the rings newest first: points at `B + 0x290`, values at `B + 0x200`
    let n = g.read::<i32>(o + 0x12c);
    let (pts, vals) = (b + 0x290, b + 0x200);
    let mut r = i;
    for j in 0..n.max(0).cast_unsigned() {
        let ru = r.cast_unsigned();
        g.read_bytes(o.wrapping_add(ru << 4), &mut q);
        g.write_bytes(pts + 16 * j, &q);
        g.read_bytes(o.wrapping_add((ru + 9) << 4), &mut q);
        g.write_bytes(vals + 16 * j, &q);
        r -= 1;
        if r < 0 {
            r = 8;
        }
    }
    // call width: 0
    let work = g.call(slot(g, 562), 0, &[]).eax;
    // the splines' rows: (knots, three moment rows) per spline
    let rows_p = [b + 0x10c, b + 0x154, b + 0xe8, b + 0x130];
    let rows_v = [b + 0xc4, b + 0xa0, b + 0x178, b + 0x19c];
    // the five pushes of each call and its return address
    let sp = b.wrapping_sub(0x18);
    let frame = |g: &mut G, at: u32, ecx: u32, edx: u32, rows: [u32; 4], n: u32, x: f32| {
        for (j, word) in (0u32..).zip([rows[0], rows[1], rows[2], rows[3], n]) {
            g.write::<u32>(sp + 4 + 4 * j, word);
        }
        Frame {
            at,
            esp: sp,
            eax: 0,
            ecx,
            edx,
            xmm: [u128::from(x.to_bits()), 0, 0, 0],
        }
    };
    let setup = img.rel32_target(f + 0x36b);
    let point = img.rel32_target(f + 0x3eb);
    let n = g.read::<u32>(o + 0x12c);
    let fr1 = frame(g, setup, work, pts, rows_p, n, 0.0);
    crate::s7_misc::spline_setup(g, img, &fr1);
    let n = g.read::<u32>(o + 0x12c);
    let fr1 = frame(g, setup, work, vals, rows_v, n, 0.0);
    crate::s7_misc::spline_setup(g, img, &fr1);
    let n = g.read::<u32>(o + 0x12c);
    let steps = (n << 4).wrapping_sub(0x10);
    let (p0, q0, p1, q1) = (b + 0x40, b + 0x50, b + 0x60, b + 0x70);
    let fr1 = frame(g, point, p0, pts, rows_p, n, 0.0);
    crate::s6_misc::spline_point(g, img, &fr1);
    let fr1 = frame(g, point, q0, vals, rows_v, n, 0.0);
    crate::s6_misc::spline_point(g, img, &fr1);
    let l = lock(g, 0x427);
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    let args = if flag != 0 {
        [o, 0x80, flag]
    } else {
        [o, steps, 0]
    };
    // call width: 3
    g.call(slot(g, 831), 0, &args);
    let l = lock(g, 0x458);
    g.write::<u32>(l, saved);
    // the colours: (c0, c1, c2) / 255, c3 / 128; (c4, c5, c6) / 255
    let tbl = w(0x491);
    let to_f = |g: &G, x: u32| {
        let big = g.read::<f64>(tbl + 8 * (x >> 31));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let v = (f64::from(x.cast_signed()) + big) as f32;
        v
    };
    let k255 = k(g, 0x477);
    let ca = [
        div(to_f(g, col[0]), k255),
        div(to_f(g, col[1]), k255),
        div(to_f(g, col[2]), k255),
        mul(to_f(g, col[3]), k(g, 0x52b)),
    ];
    let cb = [
        div(to_f(g, col[4]), k255),
        div(to_f(g, col[5]), k255),
        div(to_f(g, col[6]), k255),
    ];
    let total = crate::sb_sse::cvt(n << 4);
    let mut step = 0u32;
    while step.cast_signed() < steps.cast_signed() {
        let left = steps.wrapping_sub(step);
        let alpha = (u32::from(g.read::<u8>(o + 0x127)).cast_signed())
            .wrapping_mul(left.cast_signed())
            .wrapping_div(steps.cast_signed());
        let t = div(crate::sb_sse::cvt(step + 1), total);
        let n = g.read::<u32>(o + 0x12c);
        let fr1 = frame(g, point, p1, pts, rows_p, n, t);
        crate::s6_misc::spline_point(g, img, &fr1);
        let fr1 = frame(g, point, q1, vals, rows_v, n, t);
        crate::s6_misc::spline_point(g, img, &fr1);
        let pk = if step & 1 != 0 { w(0x671) } else { w(0x676) };
        let fa = mul(to_f(g, alpha.cast_unsigned()), k(g, 0x69d));
        let rows = |g: &G, at: u32| [0u32, 4, 8, 12].map(|o| g.read::<u32>(at + o));
        let words = [
            ca.map(f32::to_bits),
            rows(g, p0),
            [
                cb[0].to_bits(),
                cb[1].to_bits(),
                cb[2].to_bits(),
                fa.to_bits(),
            ],
            rows(g, q0),
            ca.map(f32::to_bits),
            rows(g, p1),
            [
                cb[0].to_bits(),
                cb[1].to_bits(),
                cb[2].to_bits(),
                fa.to_bits(),
            ],
            rows(g, q1),
        ];
        for (r, row) in (0u32..).zip(words) {
            for (c, x) in (0u32..).zip(row) {
                g.write::<u32>(pk + 0x20 + 0x10 * r + 4 * c, x);
            }
        }
        let l = lock(g, 0x736);
        let saved = g.read::<u32>(l);
        g.write::<u32>(l, 1);
        // call width: 3
        g.call(slot(g, 832), 0, &[o, pk, flag]);
        let l = lock(g, 0x759);
        g.write::<u32>(l, saved);
        // call width: 2
        g.call(slot(g, 382), 0, &[0, 0]);
        // the new points become the last
        for (from, to) in [(p1, p0), (q1, q0)] {
            let r = rows(g, from);
            for (c, x) in (0u32..).zip(r) {
                g.write::<u32>(to + 4 * c, x);
            }
        }
        step += 1;
    }
    let l = lock(g, 0x806);
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    // call width: 3
    g.call(slot(g, 832), 0, &[o, 0, flag]);
    let l = lock(g, 0x827);
    g.write::<u32>(l, saved);
    // call width: 2
    g.call(slot(g, 382), 0, &[0, 0]).eax
}

/// [`trail_0432`] (the `rel32`s: [`spline_setup`](crate::s7_misc::spline_setup)
/// twice, [`spline_point`](crate::s6_misc::spline_point) four times and
/// `__security_check_cookie`).
pub const TRAIL_0432: Sig = Sig {
    len: 2137,
    hash: 0x41ab_2a5f_c3b3_c97c,
    calls: &[
        (0x36b, Callee::Sig(&crate::s7_misc::SPLINE_SETUP)),
        (0x3a2, Callee::Sig(&crate::s7_misc::SPLINE_SETUP)),
        (0x3eb, Callee::Sig(&crate::s6_misc::SPLINE_POINT)),
        (0x422, Callee::Sig(&crate::s6_misc::SPLINE_POINT)),
        (0x607, Callee::Sig(&crate::s6_misc::SPLINE_POINT)),
        (0x641, Callee::Sig(&crate::s6_misc::SPLINE_POINT)),
        (0x851, Callee::Any),
    ],
    imports: &[],
};

/// The trail's task (cdecl `(task)`, a task callback; a /GS frame with
/// the vectors `P` (`ebp - 0x24`) and `Q` (`ebp - 0x14`)): `E = *G_E`,
/// the trail object `O = E + 0x1310`; `Q` and `P` are slot 268 points
/// (`(chr, 0xc, Q)`, `(chr, 0xb, P)`, `chr` the table word of the byte
/// `**G_IDX`) with `w` 1.0. While the count `O + 0x130` is positive it
/// counts down; after that the alpha byte `O + 0x127` fades by 2 each
/// step. Each step draws the trail ([`trail_0432`] `(O, P, &Q)`) and
/// returns its `eax`; once the alpha is 1 or 0 the task is given back
/// ([`give_back`](crate::sa_pool::give_back)) and slot 831 `(O, 0, 0)`
/// runs under the lock `*G_LOCK`, whose pointer is returned.
///
/// `magic_0434` 0x10007950 (255 bytes; also `magic_0610`).
pub fn trail_task_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x32), n);
    let task = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, q) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let e = g.read::<u32>(w(0x18));
    for (idx_op, tab_op, part, at) in [(0x21u32, 0x2du32, 0xcu32, q), (0x43, 0x56, 0xb, p)] {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(idx_op))));
        let chr = g.read::<u32>(w(tab_op).wrapping_add(4 * b));
        // call width: 3
        g.call(slot(g, 268), 0, &[chr, part, at]);
        g.write::<u32>(at + 0xc, 0x3f80_0000);
    }
    let o = e.wrapping_add(0x1310);
    let n = g.read::<i32>(e.wrapping_add(0x1440));
    if n > 0 {
        g.write::<i32>(o + 0x130, n - 1);
    } else {
        let a = g.read::<u8>(o + 0x127);
        if a <= 1 {
            crate::sa_pool::give_back(g, img, img.rel32_target(f + 0xc1), task);
            let l = g.read::<u32>(w(0xc6));
            let saved = g.read::<u32>(l);
            g.write::<u32>(l, 1);
            // call width: 3
            g.call(slot(g, 831), 0, &[o, 0, 0]);
            let l = g.read::<u32>(w(0xe5));
            g.write::<u32>(l, saved);
            return Ret::eax(l);
        }
        g.write::<u8>(o + 0x127, a - 2);
    }
    // push &Q; the call (three pushes before it)
    let sp = ebp.wrapping_sub(0x34);
    g.write::<u32>(sp, q);
    let draw = Frame {
        at: img.rel32_target(f + 0xa6),
        esp: sp.wrapping_sub(4),
        eax: 0,
        ecx: o,
        edx: p,
        xmm: [0; 4],
    };
    Ret::eax(trail_0432(g, img, &draw))
}

/// [`trail_task_0434`] (the `rel32`s: [`trail_0432`], the give-back and
/// two `__security_check_cookie`s).
pub const TRAIL_TASK_0434: Sig = Sig {
    len: 255,
    hash: 0xc489_9585_83b8_fe37,
    calls: &[
        (0xa6, Callee::Sig(&TRAIL_0432)),
        (0xb6, Callee::Any),
        (0xc1, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xf7, Callee::Any),
    ],
    imports: &[],
};
