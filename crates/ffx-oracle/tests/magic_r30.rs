//! Round 30's ports (`ffx_magic::r30`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;

use ffx_magic::{Ret, r30};
use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem};
use magic_owna::{filler, rec, twin};

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// A small deterministic generator (the test's triangles).
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }

    /// One of `-n..=n` eighths.
    fn eighths(&mut self, n: i32) -> f32 {
        let k = (self.next() % (2 * n as u32 + 1)) as i32 - n;
        k as f32 / 8.0
    }
}

/// The triangles of one case, in units of the region (`x` of `R`, `y` of
/// `s`; the region is `-R..R` by `-s..s`, its diagonal from `(R, -s)` to
/// `(-R, s)`): random ones and the edge cases (a vertex on the diagonal
/// or on the band's edges, an edge parallel to the diagonal, a vertical
/// edge, all above, all below, crossings outside the edges' boxes).
fn triangles(seed: u32, n: usize) -> Vec<[(f32, f32); 3]> {
    let mut g = Lcg(seed);
    let mut v: Vec<[(f32, f32); 3]> = vec![
        // the lone vertex A, B, C on each side (both lists)
        [(-0.5, 0.25), (0.75, 0.5), (0.5, 0.75)],
        [(0.75, 0.5), (-0.5, 0.25), (0.5, 0.75)],
        [(0.75, 0.5), (0.5, 0.75), (-0.5, 0.25)],
        [(0.5, 0.5), (-0.5, -0.25), (-0.25, 0.5)],
        [(-0.25, 0.5), (0.5, 0.5), (-0.5, -0.25)],
        [(-0.5, -0.25), (-0.25, 0.5), (0.5, 0.5)],
        // a vertex on the diagonal (one crossing found twice or not)
        [(0.0, 0.0), (0.75, 0.5), (-0.5, 0.25)],
        [(0.5, -0.5), (-0.5, 0.5), (0.75, 0.5)],
        // an edge parallel to the diagonal (den 0) for each edge
        [(1.0, 0.0), (0.0, 1.0), (-0.5, -0.5)],
        [(-0.5, -0.5), (1.0, 0.0), (0.0, 1.0)],
        [(0.0, 1.0), (-0.5, -0.5), (1.0, 0.0)],
        // vertical edges (dx 0) for each edge
        [(0.25, -0.5), (0.25, 0.5), (-0.75, 0.5)],
        [(-0.75, 0.5), (0.25, -0.5), (0.25, 0.5)],
        [(0.25, 0.5), (-0.75, 0.5), (0.25, -0.5)],
        [(0.25, -0.75), (0.25, 0.75), (0.75, 0.0)],
        // above, below, at the band's edges, outside on the left
        [(0.0, -1.5), (0.5, -1.0), (-0.5, -1.25)],
        [(0.0, 1.5), (0.5, 1.0), (-0.5, 1.25)],
        // below with a vertex exactly at the bottom's corner (inside by the
        // edge tests, so only `y >= bottom` keeps it whole)
        [(-1.0, 1.0), (0.0, 1.5), (0.5, 1.25)],
        [(0.0, -1.0), (0.5, 1.0), (-0.5, -1.0)],
        [(-1.5, 0.0), (-1.25, 0.5), (-1.25, -0.5)],
        [(-1.5, 0.5), (0.5, 0.5), (-1.25, -0.25)],
        // two vertices above, one inside or outside
        [(0.0, -1.5), (-0.5, 0.25), (0.5, -1.25)],
        [(0.0, -1.5), (0.75, 0.75), (0.5, -1.25)],
        // inside, outside on the right
        [(-0.5, -0.5), (-0.25, 0.25), (-0.75, 0.0)],
        [(1.25, 0.5), (0.75, 0.75), (1.5, -0.25)],
        // long edges crossing far outside the region's box
        [(-6.0, 0.25), (6.0, 0.5), (0.25, 0.75)],
        [(0.25, -0.75), (-3.0, 0.75), (3.0, 0.5)],
    ];
    for _ in 0..n {
        v.push([0; 3].map(|_: i32| (g.eighths(12), g.eighths(12))));
    }
    v
}

/// `magic_0434` 0x10001400 and `magic_0610` 0x100072b0
/// (`r30::clip_0434`, internal): special models (the box from the
/// vertices, the model copied) and others (slot 924's box, slot 934's for
/// three models, both scaled); no parts, parts without strips, strips of
/// another kind between kind-0x10 ones; each triangle case (above, below,
/// inside, outside, split around each vertex with either side, a vertex
/// on the diagonal, edges parallel to it or vertical, crossings outside
/// the boxes).
#[test]
fn clip_0434() {
    for (n, addr) in [(434u16, 0x1400u32), (610, 0x72b0)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let a = SCRATCH + 0x1_0000 * 0x10;
        let actor = SCRATCH + 0x100;
        let model = SCRATCH + 0x3f0;
        let vb = SCRATCH + 0x2_0000;
        let pvb = SCRATCH + 0x7f0;
        let out = SCRATCH + 0x2_8000;
        let ptrs = SCRATCH + 0x7c0;
        let wm = SCRATCH + 0x4800;
        let colours = a + 0xa_0000;
        // the slots the vertices' and the parts' callees use
        t.stub(544, |m, a| {
            for i in 0..4 {
                m.w32(a[1] + 4 * i, (a[0] as f32 + 0.25 * i as f32).to_bits());
            }
            0x544
        });
        rec(&mut t, 553, 5, |_, _| 0x553);
        t.stub(545, |_, _| SCRATCH + 0x4f00);
        t.write(SCRATCH + 0x4f00, &magic_h6::varied(0x40, 1));
        rec(&mut t, 230, 2, |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x230
        });
        rec(&mut t, 941, 1, |_, a| {
            if a[0] & 0x1000 != 0 {
                SCRATCH + 0x5000 + (a[0] & 0xf) * 0x40
            } else {
                SCRATCH + 0x1_0000 + (a[0] & 0xf) * 0x1000
            }
        });
        filler(&mut t, 231, 3, 16);
        // the vertices copied as they are; the end's calls (in place)
        // move them
        rec(&mut t, 229, 3, |m, a| {
            for i in 0..4 {
                let v = rf(m, a[2] + 4 * i);
                let v = if a[0] == a[2] {
                    v * 0.5 + 1.0 + i as f32
                } else {
                    v
                };
                wf(m, a[0] + 4 * i, v);
            }
            0x229
        });
        t.stub(562, move |_, _| a);
        for (s, n) in [(357u32, 3), (356, 3), (362, 4), (327, 2), (843, 2)] {
            rec(&mut t, s, n, move |_, a| 0x5a5a_0000 + s + a[0]);
        }
        rec(&mut t, 363, 3, |_, _| 0x44);
        // the boxes: lo -1/1024, hi 1/1024 (x and y; scaled by -1000 and
        // swapped, the region's sides are -R..R, R = 125/128)
        let bx = 1.0f32 / 1024.0;
        // (a word at `flip` set: the x sides swapped, left > right)
        let flip = SCRATCH + 0x7e8;
        let boxes = move |m: &mut dyn StubMem, hi: u32, lo: u32, z: f32| {
            let fx = if m.r32(flip) != 0 { -1.0 } else { 1.0 };
            for (at, s) in [(lo, -1.0f32), (hi, 1.0)] {
                wf(m, at, s * bx * fx);
                wf(m, at + 4, s * bx);
                wf(m, at + 8, z * s);
                wf(m, at + 12, 1.0);
            }
        };
        rec(&mut t, 924, 3, move |m, a| {
            boxes(m, a[1], a[2], 0.375);
            0x924
        });
        rec(&mut t, 934, 5, move |m, a| {
            boxes(m, a[3], a[4], 0.625);
            for at in [a[1], a[2]] {
                for i in 0..4 {
                    wf(m, at + 4 * i, 3.0 + i as f32);
                }
            }
            0x934
        });
        let r = 125.0f32 / 128.0;
        let k5 = f32::from_bits(0x3d4c_cccd);
        // (model, parts: (triangles, strips), slot 404's value, seed)
        // (the last: the sides swapped, so that the top edge's test can
        // fail)
        let cases: [(u16, &[(usize, bool)], f64, u32); 9] = [
            (0x10fb, &[(30, false)], -9.3, 1),
            (0x1234, &[(30, true), (12, false)], -9.25, 2),
            (0x1054, &[(26, false)], -9.5, 3),
            (0x112b, &[(20, true)], -9.125, 4),
            (0x1026, &[(16, false), (0, false), (10, true)], -9.0, 5),
            (0x1100, &[(10, false), (8, true)], -9.4, 6),
            (0x10fd, &[], -9.3, 7),
            (0x1200, &[(0, true)], -9.3, 8),
            (0x1234, &[(30, false)], -9.3, 9),
        ];
        for (case, &(id, parts, st, seed)) in cases.iter().enumerate() {
            let case = case as u32;
            t.set_u32(flip, u32::from(case == 8));
            // the tables' index (their cells junked first)
            t.set_u32(t.img.word(f + 0x367), 1 + case % 3);
            for o in [0x3aeu32, 0x45e] {
                let at = t.img.word(f + o);
                t.write(at, &magic_h6::varied(0x50, case as u8 + 0x31));
            }
            let s = (st as f32 + 10.0) + r * k5;
            t.write(actor, &magic_h6::varied(0x900, case as u8));
            t.set_u16(actor, id);
            t.set_u32(actor + 0x1c0, SCRATCH + 0x600);
            t.write(SCRATCH + 0x600, &[0; 0x20]);
            t.set_u32(actor + 0x830, 0x0830_0000 + case);
            t.write(a, &magic_h6::varied(0x200, case as u8 + 3));
            t.set_u32(a, model);
            t.set_u16(model, 0x2345);
            t.set_u16(a + 4, parts.len() as u16);
            // the vertices (all parts) and the triangles' colours and
            // records, written by slot 404 after the vertices are made
            let mut verts: Vec<u32> = Vec::new();
            let mut cols: Vec<u8> = Vec::new();
            let mut total = 0u32;
            for (j, &(ntri, other)) in parts.iter().enumerate() {
                let j = j as u32;
                let p = a + 0x10 + 16 * j;
                t.set_u32(p, j + 2 * case);
                t.set_u32(p + 4, 0x1000 + j);
                let d = SCRATCH + 0x1_0000 + ((j + 2 * case) & 0xf) * 0x1000;
                t.write(d, &magic_h6::varied(0x1000, (j + case) as u8));
                // `ntri` of the edge cases and random ones, from an offset
                let all = triangles(seed * 17 + j, 40);
                let from = (seed * 7 + j * 5) as usize;
                let tris: Vec<_> = (0..ntri).map(|i| all[(from + i) % all.len()]).collect();
                let nv = 3 * tris.len() as u32 + 2;
                t.set_u32(d + 4, 0x40);
                t.set_u32(d + 8, 0x800);
                t.set_u16(d + 0x12, nv as u16);
                // the box of the vertices made: -R..R (the special models'
                // region); the rest inside
                for k in 0..nv {
                    let v = d + 0x800 + 16 * k;
                    let (x, y) = match k {
                        0 => (-r, -r),
                        1 => (r, r),
                        _ => (0.125, -0.25),
                    };
                    for (o, x) in [x, y, 0.5, 1.0].into_iter().enumerate() {
                        t.set_u32(v + 4 * o as u32, x.to_bits());
                    }
                }
                // strips: half the triangles, (another kind), the rest
                let half = tris.len() / 2;
                let mut strips: Vec<(u8, usize, usize)> = vec![(0x10, 0, half)];
                if other {
                    strips.push((0x7, 0, 0));
                }
                strips.push((0x10, half, tris.len() - half));
                t.set_u16(d + 0x10, tris.len() as u16);
                let mut e = d + 0x40;
                for &(kind, from, cnt) in &strips {
                    t.set_u8(e + 1, kind);
                    t.set_u16(e + 2, cnt as u16);
                    let mut rec = e + 0x20;
                    if kind == 0x10 {
                        for i in 0..cnt {
                            let base = 2 + 3 * (from + i) as u32;
                            for c in 0..3u32 {
                                t.set_u16(rec + 2 * c, (base + c) as u16);
                            }
                            rec += 0x1c;
                        }
                    }
                    e = rec;
                }
                if tris.is_empty() && other {
                    // no triangles: a count of 0 strips
                    t.set_u16(d + 0x10, 0);
                }
                for k in 0..nv {
                    let (x, y) = if k < 2 {
                        ((k as f32 * 2.0 - 1.0) * r, (k as f32 * 2.0 - 1.0) * r)
                    } else {
                        let (u, w) = tris[(k as usize - 2) / 3][(k as usize - 2) % 3];
                        (u * r, w * s)
                    };
                    let z = (k as f32 * 0.375 + j as f32) - 2.0;
                    verts.extend([x, y, z, 1.0].map(f32::to_bits));
                }
                for _ in 0..tris.len() {
                    for c in 0..12u32 {
                        cols.push(((total + c) * 37 + case * 11 + 5) as u8);
                    }
                    total += 1;
                }
            }
            t.write(colours, &[0x11; 0x1000]);
            rec(&mut t, 404, 1, move |m, _| {
                for (i, v) in (0u32..).zip(&verts) {
                    m.w32(vb + 4 * i, *v);
                }
                for (i, c) in (0u32..).zip(&cols) {
                    let at = colours + (i & !3);
                    let w = m.r32(at);
                    let sh = 8 * (i & 3);
                    m.w32(at, (w & !(0xff << sh)) | (u32::from(*c) << sh));
                }
                0x404
            });
            t.stub_st0(404, move |_, _| st);
            // the records' texture coordinates (the strips' bytes are
            // varied already)
            t.set_u32(pvb, vb);
            t.write(vb, &[0x22; 0x2000]);
            t.write(out, &magic_h6::varied(0x4000, case as u8 + 7));
            t.write(a + 0x5_0000, &magic_h6::varied(0x2000, case as u8 + 9));
            t.write(ptrs, &[0x33; 0x10]);
            t.write(wm, &magic_h6::varied(0x40, case as u8));
            magic_h6::scrub(&mut t);
            // the colours parts_vertices reads from its frame ([ebp-0x148..
            // -0x140], never written): 0 on both sides
            t.write(STACK_TOP - 0x100 - 0x300, &[0; 0x30]);
            let c = Call {
                ecx: actor,
                edx: wm,
                ..Call::default()
            };
            t.check(
                f,
                c,
                &[0x5555, out, pvb, ptrs, ptrs + 4, ptrs + 8, ptrs + 0xc],
                Eax::Checked,
                |g, img, fr| Ret::eax(r30::clip_0434(g, img, fr)),
            );
            // triangles were split (new vertices behind the made ones)
            let made: u32 = parts.iter().map(|&(n, _)| 16 * (3 * n as u32 + 2)).sum();
            if parts.iter().any(|&(n, _)| n > 8) {
                assert!(t.u32(pvb) > vb + made, "{n}: case {case} split nothing");
            }
        }
    }
}

/// `magic_0165` 0x10001bd0 and `magic_0606` 0x10006dd0
/// (`r30::sparks_0165`, internal): no strips (slot 329's `eax`
/// returned); kind-1 strips with records, an empty one and one of another
/// kind between them; the angle wrapped up, down and not (the second
/// angle `3pi/2 - atan2` is at least `pi/2`, so its upward wrap cannot
/// run); every colour set and both signs of the directions (`rand`).
#[test]
fn sparks_0165() {
    for (n, addr) in [(165u16, 0x1bd0u32), (606, 0x6dd0)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let s = SCRATCH + 0x1_0000;
        let r = SCRATCH + 0x8000;
        t.set_u32(t.img.word(f + 0x1b), s);
        rec(&mut t, 719, 3, move |_, _| r);
        filler(&mut t, 235, 1, 16);
        rec(&mut t, 329, 3, |m, a| {
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + f32::from_bits(a[2]) * 0.125 - 0.25 * i as f32;
                wf(m, a[0] + 4 * i, v);
            }
            0x329 ^ a[2]
        });
        rec(&mut t, 229, 3, |m, a| {
            let v: Vec<f32> = (0..4)
                .map(|i| {
                    rf(m, a[2] + 4 * i) * 0.75
                        + rf(m, a[1] + 4 * i) * 0.5
                        + rf(m, a[1] + 0x20 + 4 * i)
                        - 1.5
                })
                .collect();
            for (i, x) in (0u32..).zip(v) {
                wf(m, a[0] + 4 * i, x);
            }
            0x229
        });
        let calls = SCRATCH + 0x7e0;
        for sl in [314u32, 404] {
            rec(&mut t, sl, 1, move |_, a| sl ^ a[0]);
            t.stub_st0(sl, move |m, a| {
                let k = m.r32(calls);
                m.w32(calls, k + 1);
                f64::from(f32::from_bits(a[0])) * (f64::from(k % 5) * 0.3 - 0.55)
                    + f64::from(sl % 7)
            });
        }
        // (angle, strips: (kind, records))
        let cases: [(f32, &[(i8, u16)]); 4] = [
            (0.5, &[(1, 4), (2, 3), (1, 0), (1, 5)]),
            (-7.0, &[(1, 12)]),
            (-1.0, &[(3, 2), (1, 6)]),
            (0.25, &[]),
        ];
        for (case, &(angle, strips)) in cases.iter().enumerate() {
            let case = case as u32;
            t.write(s, &magic_h6::varied(0x6000, case as u8));
            t.set_u32(s + 0x5ee8, angle.to_bits());
            t.write(r, &magic_h6::varied(0x2000, case as u8 + 1));
            t.set_u32(r + 4, 0x40);
            t.set_u32(r + 8, 0x1000);
            let total: u16 = strips.iter().map(|&(_, c)| c).sum();
            t.set_u16(r + 0x10, total);
            let mut e = r + 0x40;
            let mut vi = 0u16;
            for &(kind, cnt) in strips {
                t.set_u8(e + 1, kind as u8);
                t.set_u16(e + 2, cnt);
                let mut rec = e + 0x10;
                if kind == 1 {
                    for _ in 0..cnt {
                        t.set_u16(rec + 0xc, vi);
                        let v = r + 0x1000 + 6 * u32::from(vi);
                        let x = (i32::from(vi) * 37 - 200 + case as i32 * 11) as i16;
                        let z = (i32::from(vi) * 53 % 300 - 120) as i16;
                        t.set_u16(v, x as u16);
                        t.set_u16(v + 2, 7);
                        t.set_u16(v + 4, z as u16);
                        vi += 1;
                        rec += 0x1c;
                    }
                }
                e = rec;
            }
            t.set_u32(calls, case);
            magic_h6::scrub(&mut t);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(r30::sparks_0165(g, img, fr))
            });
        }
    }
}

/// `magic_0152` 0x1000b860 and `magic_0533` 0x1000b290
/// (`r30::start_up_0152`, an entry): the targets (none and two), the
/// files with the load flag, every static node appended (the root list
/// empty at first, the static links after), slot 601's angle on both
/// sides of 3pi/2.
#[test]
fn start_up_0152() {
    for (n, addr) in [(152u16, 0xb860u32), (533, 0xb290)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let targets = t.img.rel32_target(f + 0xe);
        let count_at = t.img.word(targets + 0x35);
        let list = SCRATCH + 0x600;
        let flag = SCRATCH + 0x700;
        t.set_u32(t.img.word(f + 0x5e), flag);
        rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
        rec(&mut t, 218, 2, move |m, a| {
            m.w32(a[1], m.r32(list + 0x7c));
            list
        });
        rec(&mut t, 531, 2, |m, a| {
            if a[1] > 0x1000_0000 && a[1] < 0x4000_0000 {
                m.w32(a[1], a[0] ^ 0x5310);
            }
            0x531
        });
        rec(&mut t, 289, 1, |_, _| 0x289);
        t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
        rec(&mut t, 711, 3, move |m, a| {
            // the load flag while the file starts
            m.w32(list + 0x40, m.r32(flag));
            a[0] ^ 0x711
        });
        rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1] ^ (a[2] << 4));
        rec(&mut t, 601, 2, move |m, a| {
            let k = m.r32(list + 0x78);
            m.w32(a[1], (1.25f32 + k as f32 * 2.5).to_bits());
            0x601
        });
        for (s, words) in [
            (718u32, 1usize),
            (710, 4),
            (222, 4),
            (221, 4),
            (551, 3),
            (325, 2),
            (215, 1),
            (290, 2),
            (27, 2),
            (4, 1),
            (281, 1),
            (694, 4),
        ] {
            rec(&mut t, s, words, move |_, a| (s << 16) ^ a[0]);
        }
        for (k, count) in [0u32, 2].into_iter().enumerate() {
            let k = k as u32;
            t.write(list, &[0x80, 0x81, 0x83, 0x84]);
            t.set_u32(list + 0x7c, count);
            t.set_u32(list + 0x78, k);
            t.set_u32(count_at, 0x3333);
            t.set_u32(flag, 0x0f0f_0000 + k);
            // junk in the globals the start reads or clears
            for o in [0xa8u32, 0x158, 0x428, 0x56e, 0x60d, 0x78a, 0x79b, 0x7a5] {
                let at = t.img.word(f + o);
                t.set_u32(at, 0x5a5a_0000 ^ o ^ k);
            }
            for o in [0x81au32, 0x96e] {
                let at = t.img.word(f + o);
                t.set_u8(at, 0x40 + o as u8 + k as u8);
            }
            for o in [0x61du32, 0x640, 0x8e4, 0xa14, 0xb65, 0xca6] {
                let at = t.img.word(f + o);
                t.set_u32(at, 0x7777_0000 ^ o);
            }
            magic_h6::scrub(&mut t);
            t.check_entry(f, Call::default(), &[0x30 + k], Eax::Checked);
        }
    }
}

/// `magic_0432` 0x100014d0, `magic_0434` 0x10005db0 and `magic_0610`
/// 0x1000aca0 (`r30::trail_0432`, internal): one and two points (no
/// trail), the third (the first trail), a full ring wrapping its index;
/// slot 773 saying 0 and not (the flag); colour bytes above 0x7f.
#[test]
fn trail_0432() {
    for (n, addr) in [(432u16, 0x14d0u32), (434, 0x5db0), (610, 0xaca0)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let o = SCRATCH + 0x1000;
        let v = SCRATCH + 0x800;
        let val = SCRATCH + 0x810;
        let lock = SCRATCH + 0x700;
        let work = SCRATCH + 0x2_0000;
        let log = SCRATCH + 0x7f0;
        t.set_u32(t.img.word(f + 0x37), lock);
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
        t.stub_edx(772, |_, a| 0x0772_1000 ^ a[1]);
        rec(&mut t, 773, 2, move |m, a| {
            m.w32(log + 4, m.r32(lock) ^ a[1]);
            m.r32(log + 8)
        });
        rec(&mut t, 382, 2, |_, _| 0x382);
        filler(&mut t, 723, 1, 16);
        filler(&mut t, 724, 1, 16);
        filler(&mut t, 235, 1, 16);
        rec(&mut t, 231, 3, |m, a| {
            for i in 0..16 {
                let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) * 0.25;
                wf(m, a[0] + 4 * i, x);
            }
            0x231
        });
        t.stub(562, move |_, _| work);
        rec(&mut t, 831, 3, move |m, a| {
            m.w32(log + 0xc, m.r32(lock) ^ a[2]);
            0x831
        });
        // each packet folded into a log word (the packets are rewritten)
        rec(&mut t, 832, 3, move |m, a| {
            let mut h = m.r32(log) ^ m.r32(lock);
            if a[1] != 0 {
                for i in 0..0x20 {
                    h = h.rotate_left(5) ^ m.r32(a[1] + 0x20 + 4 * i);
                }
            }
            m.w32(log, h);
            0x832
        });
        // (count before, ring index, slot 773's answer)
        let cases: [(u32, u32, u32); 5] = [(0, 3, 0), (1, 5, 1), (2, 8, 0), (8, 2, 7), (9, 8, 0)];
        for (case, &(count, index, answer)) in cases.iter().enumerate() {
            let case = case as u32;
            t.write(o, &magic_h6::varied(0x140, case as u8));
            for i in 0..18u32 {
                let at = o + 16 * i;
                let s = i as f32 + case as f32 * 0.5;
                for (c, x) in [s * 1.5 - 4.0, (s * 0.7).sin() * 3.0, s * s * 0.125, 1.0]
                    .into_iter()
                    .enumerate()
                {
                    t.set_u32(at + 4 * c as u32, x.to_bits());
                }
            }
            t.write(
                o + 0x120,
                &[0x10, 0x80, 0xff, 0x40, 0x7f, 0x01, 0xc8, 0xa0 + case as u8],
            );
            t.set_u32(o + 0x128, index);
            t.set_u32(o + 0x12c, count);
            for (c, x) in [12.5f32, -3.25, 0.5, 1.0].into_iter().enumerate() {
                t.set_u32(v + 4 * c as u32, (x + case as f32).to_bits());
                t.set_u32(val + 4 * c as u32, (x * 0.5 - case as f32).to_bits());
            }
            t.set_u32(log, 0);
            t.set_u32(log + 8, answer);
            t.set_u32(lock, 0x0101_0000 + case);
            t.write(work, &[0x44; 0x400]);
            magic_h6::scrub(&mut t);
            let c = Call {
                ecx: o,
                edx: v,
                ..Call::default()
            };
            t.check(f, c, &[val], Eax::Checked, |g, img, fr| {
                Ret::eax(r30::trail_0432(g, img, fr))
            });
            // the trail ran: its packets reached slot 832
            if count >= 2 {
                assert_ne!(t.u32(log), 0, "{n}: case {case} drew no packet");
            }
        }
    }
}

/// `magic_0434` 0x10007950 and `magic_0610` 0x1000b880
/// (`r30::trail_task_0434`, an entry): the count running, the alpha
/// fading (from 0x40, 2 and 3), and the end at 1 and 0 (the task given
/// back, slot 831 under the lock).
#[test]
fn trail_task_0434() {
    for (n, addr) in [(434u16, 0x7950u32), (610, 0xb880)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let trail = t.img.rel32_target(f + 0xa6);
        let e = SCRATCH + 0x1_0000;
        let o = e + 0x1310;
        let lock = SCRATCH + 0x700;
        let log = SCRATCH + 0x7f0;
        let idx = SCRATCH + 0x7e0;
        let task = SCRATCH + 0x900;
        t.set_u32(t.img.word(f + 0x18), e);
        t.set_u32(t.img.word(f + 0x21), idx);
        t.set_u32(t.img.word(f + 0xc6), lock);
        t.set_u32(t.img.word(trail + 0x37), lock);
        let tab = t.img.word(f + 0x2d);
        for i in 0..4u32 {
            t.set_u32(tab + 4 * i, 0x0c40_0000 + i);
        }
        rec(&mut t, 268, 3, |m, a| {
            for i in 0..3 {
                let x = (a[0] & 0xff) as f32 * 0.5 + a[1] as f32 - i as f32 * 1.25;
                wf(m, a[2] + 4 * i, x);
            }
            0x268
        });
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
        t.stub_edx(772, |_, a| 0x0772_1000 ^ a[1]);
        rec(&mut t, 773, 2, |_, _| 0);
        rec(&mut t, 382, 2, |_, _| 0x382);
        for s in [723u32, 724, 235] {
            filler(&mut t, s, 1, 16);
        }
        rec(&mut t, 231, 3, |m, a| {
            for i in 0..16 {
                let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) * 0.25;
                wf(m, a[0] + 4 * i, x);
            }
            0x231
        });
        t.stub(562, |_, _| SCRATCH + 0x2_0000);
        rec(&mut t, 831, 3, move |m, a| {
            m.w32(log + 0xc, m.r32(lock) ^ a[2]);
            0x831
        });
        rec(&mut t, 832, 3, move |m, a| {
            let mut h = m.r32(log) ^ m.r32(lock);
            if a[1] != 0 {
                for i in 0..0x20 {
                    h = h.rotate_left(5) ^ m.r32(a[1] + 0x20 + 4 * i);
                }
            }
            m.w32(log, h);
            0x832
        });
        // (count, alpha)
        for (case, (count, alpha)) in [(5i32, 0x40u8), (0, 0x40), (-2, 3), (0, 2), (0, 1), (0, 0)]
            .into_iter()
            .enumerate()
        {
            let case = case as u32;
            t.write(e, &magic_h6::varied(0x1500, case as u8));
            for i in 0..18u32 {
                let s = i as f32 + case as f32 * 0.25;
                for (c, x) in [s * 1.5 - 4.0, s * 0.75, s * s * 0.125, 1.0]
                    .into_iter()
                    .enumerate()
                {
                    t.set_u32(o + 16 * i + 4 * c as u32, x.to_bits());
                }
            }
            t.set_u8(o + 0x127, alpha);
            t.set_u32(o + 0x128, case % 9);
            t.set_u32(o + 0x12c, 2 + case % 3);
            t.set_u32(o + 0x130, count.cast_unsigned());
            t.set_u8(idx, case as u8 % 4);
            t.set_u32(lock, 0x0101_0000 + case);
            t.write(task, &[0; 0x20]);
            t.set_u32(task + 8, SCRATCH + 0x980);
            t.write(SCRATCH + 0x980, &[0; 0x20]);
            magic_h6::scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// `magic_0432` 0x10001de0 (`r30::trail_task_0432`, an entry): the alpha
/// fading (from 0x40, 3 and 2) and the end at 1 and 0 (the task given
/// back, slot 831 under the lock).
#[test]
fn trail_task_0432() {
    let Some(mut t) = twin(432) else { return };
    let f = t.at(0x1de0);
    let trail = t.img.rel32_target(f + 0x81);
    let e = SCRATCH + 0x1_0000;
    let o = e + 0xf80;
    let lock = SCRATCH + 0x700;
    let log = SCRATCH + 0x7f0;
    let task = SCRATCH + 0x900;
    t.set_u32(t.img.word(f + 0x18), e);
    t.set_u32(t.img.word(f + 0xa1), lock);
    t.set_u32(t.img.word(trail + 0x37), lock);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..3 {
            let x = (a[0] & 0xff) as f32 * 0.5 + a[1] as f32 - i as f32 * 1.25;
            wf(m, a[2] + 4 * i, x);
        }
        0x268
    });
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
    t.stub_edx(772, |_, a| 0x0772_1000 ^ a[1]);
    rec(&mut t, 773, 2, |_, _| 3);
    rec(&mut t, 382, 2, |_, _| 0x382);
    for s in [723u32, 724, 235] {
        filler(&mut t, s, 1, 16);
    }
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) * 0.25;
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    t.stub(562, |_, _| SCRATCH + 0x2_0000);
    rec(&mut t, 831, 3, move |m, a| {
        m.w32(log + 0xc, m.r32(lock) ^ a[2]);
        0x831
    });
    rec(&mut t, 832, 3, move |m, a| {
        let mut h = m.r32(log) ^ m.r32(lock);
        if a[1] != 0 {
            for i in 0..0x20 {
                h = h.rotate_left(5) ^ m.r32(a[1] + 0x20 + 4 * i);
            }
        }
        m.w32(log, h);
        0x832
    });
    for (case, alpha) in [0x40u8, 3, 2, 1, 0].into_iter().enumerate() {
        let case = case as u32;
        t.write(e, &magic_h6::varied(0x1200, case as u8));
        for i in 0..18u32 {
            let s = i as f32 + case as f32 * 0.25;
            for (c, x) in [s * 1.5 - 4.0, s * 0.75, s * s * 0.125, 1.0]
                .into_iter()
                .enumerate()
            {
                t.set_u32(o + 16 * i + 4 * c as u32, x.to_bits());
            }
        }
        t.set_u8(o + 0x127, alpha);
        t.set_u32(o + 0x128, case % 9);
        t.set_u32(o + 0x12c, 2 + case % 3);
        for op in [0x29u32, 0x42] {
            t.set_u32(t.img.word(f + op), 0x0c40_0000 + op + case);
        }
        t.set_u32(lock, 0x0101_0000 + case);
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, SCRATCH + 0x980);
        t.write(SCRATCH + 0x980, &[0; 0x20]);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0432` 0x10001390 (`r30::aim_0432`, internal): the mode word 1
/// and not; the angle wrapped down, up and not.
#[test]
fn aim_0432() {
    let Some(mut t) = twin(432) else { return };
    let f = t.at(0x1390);
    let e = SCRATCH + 0x1_0000;
    let idx = SCRATCH + 0x7e0;
    t.set_u32(t.img.word(f + 0x19), e);
    t.set_u32(t.img.word(f + 0x4c), idx);
    let tab = t.img.word(f + 0x6d);
    for i in 0..4u32 {
        t.set_u32(tab + 4 * i, 0x0262_0000 + i);
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            let x = (a[0] & 0xfff) as f32 * 0.5 - i as f32;
            wf(m, a[1 + i as usize], x);
        }
        0x262
    });
    let ang = SCRATCH + 0x7f0;
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |m, _| f64::from(rf(m, ang)));
    for (case, (mode, a)) in [(1u32, 0.5f32), (0, 9.0), (5, 6.0), (1, 0.0)]
        .into_iter()
        .enumerate()
    {
        let case = case as u32;
        t.write(e + 0xf00, &magic_h6::varied(0x100, case as u8));
        t.write(e + 0x32b0, &magic_h6::varied(0x30, case as u8 + 1));
        t.set_u32(t.img.word(f + 0x41), mode);
        t.set_u32(t.img.word(f + 0x33), 0x0c40_0000 + case);
        t.set_u32(t.img.word(f + 0x122), 0x7777);
        t.set_u8(idx, case as u8);
        t.set_u32(ang, a.to_bits());
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r30::aim_0432(g, img, fr))
        });
    }
}

/// `magic_0426` 0x100035b0 and `magic_0297` 0x10003230
/// (`r26::mesh_packets_0426`, internal, `ecx` = the draw record; 0297's
/// block is 8 bytes longer): two parts, strips of three faces, none and
/// two (the last ended by 0xff), one part ended by another kind byte; the
/// cursor moved near the buffer's end (the swap) or past its start.
#[test]
fn mesh_packets_0297() {
    for (n, addr) in [(426u16, 0x35b0u32), (297, 0x3230)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let blk = SCRATCH + 0x1_0000;
        rec(&mut t, 264, 1, move |_, _| blk);
        rec(&mut t, 265, 1, |_, a| 0x0265_0000 ^ a[0]);
        let parts = SCRATCH + 0x6000;
        let cnt = SCRATCH + 0x7a0;
        let mode = SCRATCH + 0x7a4;
        rec(&mut t, 941, 1, move |m, a| {
            let k = m.r32(cnt).wrapping_add(1);
            m.w32(cnt, k);
            if k == 2 {
                match m.r32(mode) {
                    1 => {
                        let end = m.r32(blk + 0xd8);
                        m.w32(blk + 0xdc, end - 0x100);
                    }
                    2 => {
                        let start = m.r32(blk + 0xd4);
                        m.w32(blk + 0xdc, start + 0x40);
                    }
                    _ => {}
                }
            }
            parts + 0x400 * (a[0] & 3)
        });
        let model = SCRATCH + 0x5800;
        let r = SCRATCH + 0x5c00;
        for p in 0..4u32 {
            let at = parts + 0x400 * p;
            t.write(at, &magic_h6::varied(0x400, p as u8));
            t.set_u32(at + 4, 0x40);
            t.set_u32(at + 8, 0x200);
            let mut s = at + 0x40;
            for (k, (cnt_f, last)) in [(3u16, false), (0, false), (2, true)]
                .into_iter()
                .enumerate()
            {
                t.set_u8(s + 1, if p == 3 && k == 1 { 0x20 } else { 0x10 });
                t.set_u16(s + 2, cnt_f);
                let mut face = s + 0x20;
                for i in 0..u32::from(cnt_f) {
                    for (j, ix) in [i, i + 3, (i * 7 + p) % 0x20].into_iter().enumerate() {
                        t.set_u16(face + 2 * j as u32, ix as u16);
                    }
                    face += 0x1c;
                }
                if last {
                    t.set_u8(face + 1, 0xff);
                }
                s = face;
            }
        }
        for (k, (np, md)) in [(2u8, 0u32), (2, 1), (3, 2), (0, 0)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.write(blk, &magic_h6::varied(0x3a00, k as u8));
            t.write(model, &[0x29; 0x50]);
            t.set_u8(model + 4, np);
            for e in 0..3u32 {
                t.set_u32(model + 0x10 + 0x10 * e, e + k);
            }
            t.write(r, &magic_h6::varied(0x50, k as u8 + 4));
            t.set_u32(r + 0x48, model);
            t.set_u32(cnt, 0);
            t.set_u32(mode, md);
            magic_h6::scrub(&mut t);
            let c = Call {
                ecx: r,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(ffx_magic::r26::mesh_packets_0426(g, img, fr.at, fr.ecx))
            });
        }
    }
}

/// `magic_0432` 0x10001ec0 (`r30::trail_start_0432`, an entry): the
/// angle wrapped down, up and not; the task list empty and not.
#[test]
fn trail_start_0432() {
    let Some(mut t) = twin(432) else { return };
    let f = t.at(0x1ec0);
    let e = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0xb), e);
    magic_r11::pools(&mut t, f, 323);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x20);
    magic_r11::object_start_auto(&mut t, start);
    let (head, tail) = (t.img.word(f + 0x10b), t.img.word(f + 0x11d));
    let old = SCRATCH + 0x900;
    for (case, (a, listed)) in [(2.0f32, false), (-2.0, true), (-5.5, true), (0.25, false)]
        .into_iter()
        .enumerate()
    {
        let case = case as u32;
        t.write(e, &magic_h6::varied(0x3300, case as u8));
        t.set_u32(e + 0x32c0, a.to_bits());
        for (i, x) in [1.5f32, -0.75, 3.0, 1.0].into_iter().enumerate() {
            t.set_u32(e + 0xf70 + 4 * i as u32, (x - case as f32).to_bits());
        }
        t.write(old, &[0x6c; 0x20]);
        t.set_u32(head, if listed { old } else { 0 });
        t.set_u32(tail, if listed { old } else { 0x0bad_0000 });
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0432` 0x10002010 (`r30::spark_start_0432`, an entry): the pool
/// with a free record, one to reuse and none.
#[test]
fn spark_start_0432() {
    let Some(mut t) = twin(432) else { return };
    let f = t.at(0x2010);
    let take = t.img.rel32_target(f + 0x6);
    let recs = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(take + 0x16), recs);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x4f);
    magic_r11::object_start_auto(&mut t, start);
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 0.375 - 0.125);
    let src = SCRATCH + 0x900;
    // (records, the free one, reuse instead)
    for (case, (n, free, reuse)) in [(4u32, 2u32, false), (4, 9, true), (4, 9, false)]
        .into_iter()
        .enumerate()
    {
        let case = case as u32;
        t.write(recs, &magic_h6::varied(0x140 * 4, case as u8));
        for i in 0..4u32 {
            let r = recs + 0x140 * i;
            t.set_u32(r + 0x130, u32::from(i != free));
            t.set_u16(r + 0x124, if reuse && i == 3 { 0x0123 } else { 0x8123 });
        }
        t.set_u32(t.img.word(take + 0x09), n);
        t.set_u32(t.img.word(take + 0x01), 1);
        t.write(src, &magic_h6::varied(0x100, case as u8 + 2));
        // a point of plain numbers (`y - 1` must differ from `y`)
        for (i, x) in [0.5f32, 2.25, -1.5, 1.0].into_iter().enumerate() {
            t.set_u32(src + 0xd0 + 4 * i as u32, (x + case as f32).to_bits());
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[src], Eax::Checked);
    }
}

/// `magic_0432` 0x10002110 (`r30::targets_start_0432`, an entry): no
/// targets, one and three; slot 390's kinds in the DLL's list and not
/// (both ways of the height); the point capped at the ground and not; the
/// turn wrapped.
#[test]
fn targets_start_0432() {
    let Some(mut t) = twin(432) else { return };
    let f = t.at(0x2110);
    let e = SCRATCH + 0x1_0000;
    let sel = SCRATCH + 0x180;
    let log = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(f + 0x17), e);
    t.set_u32(t.img.word(f + 0x80), sel);
    let tab = t.img.word(f + 0x8b);
    for i in 0..8u32 {
        t.set_u32(tab + 4 * i, 0x0c40_0000 + i);
    }
    magic_r11::pools(&mut t, f, 1093);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x32a);
    magic_r11::object_start_auto(&mut t, start);
    let check = t.img.rel32_target(f + 0x22a);
    let kinds = t.img.word(check + 0x28);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            let x = (a[0] & 0xff) as f32 * 0.75 - 1.5 + i as f32 * 0.5;
            wf(m, a[1 + i as usize], x);
        }
        0x262
    });
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * -0.25 + 0.5);
    rec(&mut t, 268, 3, move |m, a| {
        let cap = m.r32(log + 4);
        for i in 0..3u32 {
            let y = if i == 1 && cap != 0 {
                -9.0
            } else {
                2.0 + i as f32
            };
            wf(m, a[2] + 4 * i, y);
        }
        0x268
    });
    rec(&mut t, 314, 1, move |_, a| 0x0314_0000 ^ a[0]);
    // a kind not in the list: slot 534's answer 0 (the kind check says 0)
    rec(&mut t, 534, 1, |_, _| 0);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.5 + 0.0625);
    rec(&mut t, 390, 1, move |m, a| {
        let k = m.r32(log);
        if (k >> (a[0] & 7)) & 1 != 0 {
            m.r32(kinds)
        } else {
            0x7777
        }
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16u32 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { i });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m, a| {
            let v = m.r32(a[1]) ^ a[2];
            m.w32(a[0], v);
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4u32 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    // (targets, the kinds' bits, the cap)
    for (case, (n, bits, cap)) in [(0i32, 0u32, 0u32), (1, 0, 0), (3, 0b101, 1), (3, 0b010, 0)]
        .into_iter()
        .enumerate()
    {
        let case = case as u32;
        t.write(e, &magic_h6::varied(0x3300, case as u8));
        for i in 0..3u32 {
            t.set_u32(
                e + 0x1180 + 0x3a0 * i - 0xc,
                (1.0 + 1.5 * i as f32 + case as f32).to_bits(),
            );
        }
        t.set_u32(t.img.word(f + 0x62), n.cast_unsigned());
        for i in 0..8u8 {
            t.set_u8(sel + u32::from(i), i + case as u8);
        }
        t.set_u32(log, bits);
        t.set_u32(log + 4, cap);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0432` 0x10001210 (`r9_load::load_0432`, an entry) and its
/// start-up 0x10002560 (`r30::start_up_0432`): slot 225 busy, the first
/// and a later request, the data not loaded and loaded (the start-up then
/// runs: the sound, the aim, the records, two task nodes appended to an
/// empty list and to a list with a node).
#[test]
fn load_0432() {
    let Some(mut t) = twin(432) else { return };
    let at = t.at(0x1210);
    let w = |t: &magic::Twin, o: u32| t.img.word(at + o);
    let busy = SCRATCH + 0x100;
    let loaded = SCRATCH + 0x104;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    let start = t.img.rel32_target(at + 0x5b);
    let sound = t.img.rel32_target(start + 0x42);
    let lock = SCRATCH + 0x700;
    t.set_u32(t.img.word(sound + 0x04), lock);
    rec(&mut t, 711, 3, move |m, a| m.r32(lock) ^ a[0]);
    rec(&mut t, 710, 4, |_, a| 0x0710_0000 ^ a[2]);
    rec(&mut t, 271, 1, |_, a| 0x0271_0000 ^ a[0]);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            wf(m, a[1 + i as usize], (a[0] & 0xff) as f32 + i as f32);
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.75);
    let aim = t.img.rel32_target(start + 0x4a);
    t.set_u32(t.img.word(aim + 0x19), SCRATCH + 0x1_0000);
    t.set_u32(t.img.word(aim + 0x4c), SCRATCH + 0x7e0);
    t.set_u32(t.img.word(at + 0x26), 0x0000_5e5e);
    magic_r11::pools(&mut t, start, 309);
    let head = t.img.word(start + 0xbf);
    let old = SCRATCH + 0x900;
    for (k, (b, asked, l, listed)) in [
        (1u32, 0u32, 1u32, false),
        (0, 0, 0, false),
        (0, 1, 0xffff_ffff, false),
        (0, 0, 1, false),
        (0, 1, 2, true),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        t.set_u32(lock, 0x0c0c_0000 + k);
        t.write(old, &[0x6c; 0x20]);
        t.set_u32(head, if listed { old } else { 0 });
        t.set_u32(t.img.word(start + 0xd6), old);
        // the records' first words, junk before they are cleared
        let recs = t.img.word(start + 0x59);
        for i in 0..0x190u32 {
            t.set_u32(recs + 0x140 * i, 0x5151_0000 + i);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0171` 0x10001ad0 (`r30::sparks_0171`, an entry): enough free
/// records for both kinds (the search wrapping round), too few for the
/// first kind and for the second, none at all and an empty pool.
#[test]
fn sparks_0171() {
    let Some(mut t) = twin(171) else { return };
    let f = t.at(0x1ad0);
    let recs = SCRATCH + 0x1_0000;
    let tex = SCRATCH + 0x700;
    let obj = SCRATCH + 0x900;
    t.set_u32(t.img.word(f + 0x27), recs);
    t.set_u32(t.img.word(f + 0x80), tex);
    t.set_u32(tex, 0x7e00_0001);
    t.set_u32(tex + 4, 0x7e00_0002);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x325
    });
    let calls = SCRATCH + 0x7f0;
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| {
        let k = m.r32(calls);
        m.w32(calls, k + 1);
        f64::from(k % 9) * 0.375 - 0.25
    });
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
    // (records, the busy ones as a mask, the starting index)
    for (case, (n, busy, idx)) in [
        (16u32, 0x000fu32, 3u32),
        (16, 0x0f0f, 5),
        (8, 0x00f3, 1),
        (16, 0xffe0, 2),
        (6, 0x3f, 0),
        (0, 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let case = case as u32;
        t.write(recs, &magic_h6::varied(0x50 * 16, case as u8));
        for i in 0..16u32 {
            t.set_u32(recs + 0x50 * i + 0x48, (busy >> i) & 1);
            for j in 0..3u32 {
                t.set_u32(
                    recs + 0x50 * i + 0xc + 4 * j,
                    (0.5 * i as f32 - j as f32).to_bits(),
                );
            }
        }
        t.set_u32(t.img.word(f + 0x12), n);
        t.set_u32(t.img.word(f + 0x17), idx);
        t.write(obj, &magic_h6::varied(0x380, case as u8 + 3));
        for j in 0..4u32 {
            t.set_u32(obj + 0xd0 + 4 * j, (1.25 * j as f32 - 2.0).to_bits());
            t.set_u32(obj + 0x368 + 4 * j, (0.75 + j as f32 * 0.5).to_bits());
        }
        t.set_u32(calls, case);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// Every slot answers its first word xor the slot (logged), unless a
/// test stubs it again.
fn all_slots(t: &mut magic::Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            if let Some(w) = a.first() {
                m.w32(SCRATCH + 0xa10 + 4 * (k % 0x100), *w ^ s);
            }
            m.w32(SCRATCH + 0xa00, k + 1);
            a.first().copied().unwrap_or(0) ^ (s << 20)
        });
    }
}

/// `magic_0171` 0x100014d0 (`r30::sequence_0171`, an entry): the wait
/// running, negative and running out; step 0 with an actor left and none;
/// step 1 for the last actor and not; step 2 (the objects, the emitter,
/// the hit: a short and a long distance); step 3 (sprites registered,
/// some slots empty); a step past 3.
#[test]
fn sequence_0171() {
    let Some(mut t) = twin(171) else { return };
    all_slots(&mut t);
    let f = t.at(0x14d0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    for call in [0x12du32, 0x285, 0x4ef] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(&mut t, start);
    }
    let chr = SCRATCH + 0x100;
    for op in [0x133u32, 0x15b, 0x1ca, 0x28b, 0x2b3] {
        t.set_u32(w(&t, op), chr);
    }
    let map = SCRATCH + 0x180;
    for i in 0..8u8 {
        t.set_u8(map + u32::from(i), i);
    }
    for (call, (sel_op, chrs_op)) in [(0x33fu32, (0x26u32, 0x31u32)), (0x502, (0x26, 0x31))] {
        let ap = t.img.rel32_target(f + call);
        t.set_u32(t.img.word(ap + sel_op), map);
        let chrs = t.img.word(ap + chrs_op);
        for i in 0..8u32 {
            t.set_u32(chrs + 4 * i, 0x0c40_0000 + i);
        }
    }
    let ta = t.img.rel32_target(f + 0x50c);
    t.set_u32(t.img.word(ta + 0x11), map);
    t.set_u32(w(&t, 0x543), map);
    let colours = SCRATCH + 0x1c0;
    t.set_u32(w(&t, 0x3b1), colours);
    t.set_u32(colours, 0x8040_2010);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            wf(
                m,
                a[1 + i as usize],
                (a[0] & 0xff) as f32 * 0.5 + i as f32 - 1.0,
            );
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..3u32 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 0xff) as f32 * 0.25 + i as f32 + 0.5,
            );
        }
        0x268
    });
    let far = SCRATCH + 0x7e0;
    rec(&mut t, 234, 3, move |m, a| {
        let s = rf(m, far);
        for i in 0..3u32 {
            let v = (rf(m, a[2] + 4 * i) - rf(m, a[1] + 4 * i)) * s;
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    for (s, v) in [(364u32, 2.5f64), (16, 0.75), (12, -0.5)] {
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 3) * 0.125);
    }
    // the caster's turn (a word per case: 9.0 wraps the yaw upward)
    let rot = SCRATCH + 0x7e4;
    t.stub_st0(333, move |m, a| {
        f64::from(rf(m, rot)) + f64::from(a[0] & 3) * 0.125
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16u32 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    let aim = t.img.rel32_target(f + 0x472);
    let take = t.img.rel32_target(aim + 0x1ea);
    let sprites = w(&t, 0x56c);
    let rl = t.img.rel32_target(f + 0x577);
    t.set_u32(t.img.word(rl + 0x21), SCRATCH + 0x7f8);
    // (wait, step, actor index, actors, the distance's scale)
    let cases: [(i32, i16, u32, u32, f32); 11] = [
        (5, 0, 1, 3, 1.0),
        (-2, 0, 1, 3, 1.0),
        (1, 0, 1, 3, 1.0),
        (0, 0, 3, 3, 1.0),
        (0, 1, 1, 3, 1.0),
        (0, 1, 3, 3, 1.0),
        (0, 2, 1, 3, 1.0),
        (0, 2, 2, 3, 40.0),
        (0, 3, 2, 3, 1.0),
        (0, 3, 2, 3, 1.0),
        (0, 7, 2, 3, 1.0),
    ];
    for (k, (wait, step, i, n, scale)) in (0u32..).zip(cases) {
        t.set_u32(w(&t, 0x15), wait.cast_unsigned());
        t.set_u16(w(&t, 0x31), step.cast_unsigned());
        t.set_u32(w(&t, 0x47), i);
        t.set_u32(w(&t, 0x4d), n);
        t.set_u32(w(&t, 0x90), 0x9090);
        t.set_u32(far, scale.to_bits());
        t.set_u32(rot, if scale > 2.0 { 9.0f32 } else { 1.25 }.to_bits());
        let objs = w(&t, 0x11a);
        t.write(objs, &magic_h6::varied(0x130 * 3, k as u8));
        let ems = w(&t, 0x27a);
        t.write(ems, &magic_h6::varied(0x3a0 * 3, k as u8 + 1));
        let hits = w(&t, 0x4e5);
        t.write(hits, &magic_h6::varied(0x130 * 3, k as u8 + 2));
        magic_hi::pool(&mut t, take, SCRATCH + 0x5000, 4);
        for j in 0..8u32 {
            t.set_u32(
                sprites + 4 * j,
                if (j + k) % 3 == 0 {
                    0
                } else {
                    SCRATCH + 0x6000 + 0x40 * j
                },
            );
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0262` 0x10007010 (`r30::summon_task_0262`, an entry): every
/// state below and at its frame count (the static nodes, the node
/// append and insert, the flag steps, the static link, the random signs,
/// the fade word below and at its top, the object to its node, the end
/// with the task unlinked), and states the dispatch passes by.
#[test]
fn summon_task_0262() {
    let Some(mut t) = twin(262) else { return };
    all_slots(&mut t);
    let f = t.at(0x7010);
    // the static nodes' root lists empty (their last is the sentinel)
    for (call, sent) in [(0x6fu32, 0x49u32), (0x235, 0x49)] {
        let sn = t.img.rel32_target(f + call);
        let s = t.img.word(sn + sent);
        t.set_u32(t.img.word(sn + 0x04), s);
    }
    let rnd = SCRATCH + 0x7e0;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(rnd);
        m.w32(rnd, k.wrapping_mul(5).wrapping_add(3));
        k
    });
    let task = SCRATCH + 0x900;
    let r = SCRATCH + 0x980;
    let node = SCRATCH + 0xa00;
    let (prev, next) = (SCRATCH + 0xa40, SCRATCH + 0xa80);
    // (state, frame count, the fade word)
    let cases: [(u32, u32, f32); 21] = [
        (0, 7, 0.5),
        (1, 3, 0.5),
        (1, 30, 0.5),
        (5, 3, 0.5),
        (10, 3, 0.5),
        (10, 45, 0.5),
        (11, 50, 0.5),
        (11, 2, 0.5),
        (12, 3, 0.5),
        (12, 90, 0.5),
        (13, 3, 0.5),
        (13, 30, 0.5),
        (16, 3, 0.5),
        (20, 3, 0.5),
        (21, 3, 0.25),
        (21, 10, 0.995),
        (22, 3, 0.5),
        (22, 50, 0.5),
        (0x2710, 3, 0.5),
        (30, 3, 0.5),
        (0x2711, 3, 0.5),
    ];
    for (k, (s, n, fade)) in (0u32..).zip(cases) {
        t.write(r, &magic_h6::varied(0x40, k as u8));
        t.set_u32(r + 0x18, s);
        t.set_u32(r + 0x2c, n);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.write(node, &[0x4e; 0x10]);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.write(prev, &[0x50; 0x10]);
        t.write(next, &[0x4f; 0x10]);
        t.set_u32(t.img.word(f + 0x1bd), fade.to_bits());
        t.set_u32(t.img.word(f + 0x3e8), fade.to_bits());
        t.set_u32(rnd, k);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0262` 0x100074e0 (`r30::summon_task_0262_b`, an entry): the
/// set-up (state 0, running on into 10 and 11), state 10 alone, 11 below
/// and at its count, 12, 60 (the flag bit set and clear), 61 (slot 411
/// saying 0 and not), the end, and states the dispatch passes by.
#[test]
fn summon_task_0262_b() {
    let Some(mut t) = twin(262) else { return };
    all_slots(&mut t);
    let f = t.at(0x74e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let sn = t.img.rel32_target(f + 0xdf);
    let s = t.img.word(sn + 0x3f);
    t.set_u32(t.img.word(sn + 0x04), s);
    let hz = t.img.rel32_target(f + 0x19f);
    let tab = SCRATCH + 0x1_0000;
    for i in 0..0x1000u32 {
        t.set_u32(tab + 4 * i, (0.5 + 0.0007 * i as f32).to_bits());
    }
    t.set_u32(t.img.word(hz + 0x2c), tab);
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            wf(
                m,
                a[1] + 4 * i,
                (a[0] & 0xff) as f32 * 0.25 + i as f32 - 1.5,
            );
        }
        0x217
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.0625 + 0.25);
    let say = SCRATCH + 0x7e8;
    rec(&mut t, 411, 1, move |m, _| m.r32(say));
    // the model start's ring tail, model list (slot 218) and root
    let ms = t.img.rel32_target(f + 0x15b);
    let models = SCRATCH + 0x700;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        models
    });
    t.write(models, &[3, 0, 5, 0, 0, 0, 0, 0]);
    let tail = SCRATCH + 0x500;
    t.write(tail, &[0x55; 0x10]);
    t.set_u32(t.img.word(ms + 0x07), tail);
    let root = t.img.word(ms + 0x62);
    t.set_u32(root + 0x10, root);
    // the DLL's node list (the start-up's work): empty, its last itself
    let list = w(&t, 0x6a);
    t.set_u32(list + 0x10, list);
    let pf = t.img.rel32_target(f + 0x26d);
    let plist = t.img.word(pf + 0x20);
    if t.u32(plist + 0x10) == 0 {
        t.set_u32(plist + 0x10, plist);
    }
    // the particle node's owner record and its programs (slot 941)
    let owner = SCRATCH + 0xb00;
    t.write(owner, &[0; 0x40]);
    t.set_u32(owner + 0x10, 0x0941);
    t.set_u32(t.img.word(pf + 0xd), owner);
    let programs = SCRATCH + 0xc00;
    for i in 0..8u32 {
        t.set_u32(programs + 4 * i, SCRATCH + 0xd00 + 0x40 * i);
    }
    rec(&mut t, 941, 1, move |_, _| programs);
    magic_r11::object_slots(&mut t);
    let task = SCRATCH + 0x900;
    let r = SCRATCH + 0x980;
    let node = SCRATCH + 0xa00;
    let (prev, next) = (SCRATCH + 0xa40, SCRATCH + 0xa80);
    let obj = w(&t, 0x18e);
    let flag = w(&t, 0x396);
    // (state, frame count, the object's bit 0, the flag byte, slot 411)
    let cases: [(u32, u32, u8, u8, u32); 15] = [
        (0, 3, 0, 0, 1),
        (10, 3, 0, 0, 1),
        (10, 3, 1, 0, 1),
        (11, 3, 0, 0, 1),
        (11, 25, 0, 0, 1),
        (12, 3, 0, 0, 1),
        (12, 5, 0, 0, 1),
        (60, 3, 0, 0, 1),
        (60, 135, 0, 1, 1),
        (60, 140, 0, 0, 1),
        (61, 3, 0, 0, 0),
        (61, 3, 0, 0, 7),
        (0x2710, 3, 0, 0, 1),
        (5, 3, 0, 0, 1),
        (70, 3, 0, 0, 1),
    ];
    for (k, (st, n, bit, fl, s411)) in (0u32..).zip(cases) {
        t.write(r, &magic_h6::varied(0x40, k as u8));
        t.set_u32(r + 0x18, st);
        t.set_u32(r + 0x2c, n);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.write(node, &[0x4e; 0x10]);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.write(prev, &[0x50; 0x10]);
        t.write(next, &[0x4f; 0x10]);
        let o = t.u32(obj + 0x90) & !1;
        t.set_u32(obj + 0x90, o | u32::from(bit));
        t.set_u8(flag, fl);
        t.set_u32(say, s411);
        magic_h6::scrub(&mut t);
        // the frame word state 10 reads unwritten ([B + 8]): 0 on both sides
        t.set_u32(STACK_TOP - 0x100 - 0x30, 0);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
