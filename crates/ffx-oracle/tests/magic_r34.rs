//! Round 34's ports (`ffx_magic::r34`) against each original. Needs
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
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, r34};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    magic_h6::scrub(&mut t);
    Some(t)
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn kf(t: &Twin, f: u32, o: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(f + o)))
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

const CHRS: u32 = SCRATCH + 0x800;
/// The rotation slot 333 answers (ST0).
const ROT: u32 = SCRATCH + 0x24;

/// The slots the placements and turns call (positions, the rotation at
/// [`ROT`], the scales, the matrices mixing every word they read, the
/// shows and hides) and one flat mesh (one kind-3 group with two
/// triangles) slot 719 answers with (as `magic_r27.rs`'s `stubs_0586`).
fn model_stubs(t: &mut Twin) {
    all_stubs(t);
    rec(t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + (a[0] & 3) as f32);
        }
        0x262
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |m, _| f64::from(rf(m, ROT)));
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 8));
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        let mix = (0..4).fold(0u32, |x, i| x.rotate_left(7) ^ m.r32(a[2] + 4 * i));
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i)
                ^ m.r32(a[1] + 0x30 + 4 * i)
                ^ m.r32(a[2] + 4 * i).rotate_left(i + 1)
                ^ mix.rotate_left(i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(3);
            m.w32(a[0] + 4 * i, v);
        }
        0x248 ^ a[0]
    });
    rec(t, 217, 2, |m, a| {
        m.w32(a[1], 0x0217_0000 ^ a[0]);
        0x217
    });
    rec(t, 255, 2, |m, a| m.r32(a[1]) ^ a[0]);
    for (s, n) in [
        (718u32, 1usize),
        (711, 3),
        (710, 4),
        (238, 2),
        (296, 2),
        (257, 2),
        (271, 1),
        (211, 2),
        (210, 2),
        (8, 2),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the flat coordinates' bounds
    rec(t, 232, 2, |m, a| {
        for i in 0..3u32 {
            let x = rf(m, a[0] + 4 * i) * 0.125;
            m.w32(a[1] + 4 * i, x.to_bits());
        }
        0x232
    });
    rec(t, 564, 3, |_, a| a[2] ^ 0x564);
    let mesh = SCRATCH + 0x1000;
    rec(t, 719, 3, move |_, a| mesh ^ (a[1] & 0x8000_0000));
    let verts = mesh + 0x800;
    let grps = mesh + 0x40;
    for i in 0..32i32 {
        let (x, y, z) = ((i * 37) % 13 - 6, (i * 11 + 3) % 9 - 3, (i * 5 % 17) * -3);
        for (o, v) in [(0u32, x), (2, y), (4, z)] {
            t.set_u16(verts + 6 * i as u32 + o, (v as i16).cast_unsigned());
        }
    }
    t.write(mesh, &[0x4d; 0x40]);
    t.set_u32(mesh + 4, grps - mesh);
    t.set_u32(mesh + 8, verts - mesh);
    t.set_u16(mesh + 0x10, 2);
    t.write(grps, &[0x47; 0x10]);
    t.set_u8(grps + 1, 3);
    t.set_u16(grps + 2, 2);
    for j in 0..2u32 {
        let at = grps + 0x10 + 0x28 * j;
        t.write(at, &[0x52; 0x28]);
        for (o, v) in [(0xcu32, j * 3 + 1), (0xe, j * 5 + 2), (0x10, j * 7 + 4)] {
            t.set_u16(at + o, v as u16);
        }
    }
}

/// The selection `sels` and character tables `tables` (operands of `f`)
/// pointing at the selection bytes `c` and the characters.
fn chars(t: &mut Twin, f: u32, sels: &[u32], tables: &[u32], c: &[u8]) {
    let sel = SCRATCH + 0x600;
    for &o in sels {
        t.set_u32(t.img.word(f + o), sel);
    }
    t.write(sel, c);
    for &o in tables {
        for j in 0..0x40 {
            t.set_u32(t.img.word(f + o) + 4 * j, CHRS + 0x10 * j);
        }
    }
}

/// `magic_0162`'s placement's globals (operands of 0x100021d0, the same
/// places as `magic_0598`'s): the selection, the characters, `*G_OFF`,
/// the summon's battle index, the summon, the four flags.
fn place_globals_0162(t: &mut Twin, f: u32, off: u32, index: u32, k: u32) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    chars(
        t,
        f,
        &[0x14, 0x76, 0x145, 0x1d3, 0x213],
        &[0x33, 0x87, 0x155, 0x1dd, 0x21d],
        &[3 + k as u8, 1],
    );
    for o in [0x179u32, 0x399, 0x534] {
        t.set_u32(w(t, o), off);
    }
    for o in [0x3a2u32, 0x523, 0x53c] {
        t.set_u32(w(t, o), index);
    }
    for o in [0x36fu32, 0x3fc, 0x433, 0x55e, 0x579] {
        t.set_u32(w(t, o), CHRS + 0x99);
    }
    junk_ops(t, f, &[0x7ea, 0x7f4, 0x7fe, 0x808]);
}

/// `magic_0162` 0x100021d0 (`r34::place_pair_0162`): the angle wrapped
/// down, kept, wrapped up, on either limit, NaN; the summon's index in
/// the battle range or not (negative too), the battle off.
#[test]
fn place_pair_0162() {
    let Some(mut t) = twin(162) else { return };
    model_stubs(&mut t);
    let f = t.at(0x21d0);
    let s = SCRATCH + 0x1_0000;
    let rots = ownb::rotations(&t, f, 0xa2, 0xb4, 0xd0);
    let mut cases = vec![(0u32, 2u32), (0, 0x20), (1, 2), (0, 0xffff_ffff), (0, 0x1f)];
    while cases.len() < rots.len() {
        cases.push((0, 3));
    }
    for (k, ((off, index), rot)) in (0u32..).zip(cases.into_iter().zip(rots)) {
        t.set_u32(t.img.word(f + 0x20), s);
        junk(&mut t, s + 0x800, 0x240, k);
        junk(&mut t, s + 0x85b0, 0x40, k + 1);
        junk(&mut t, s + 0xa30, 0x600 * 20, k + 2);
        place_globals_0162(&mut t, f, off, index, k);
        t.set_u32(ROT, rot.to_bits());
        ownb::dirty_stack(&mut t, rot.to_bits(), &[]);
        ownb::reset_counters(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r34::place_pair_0162(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x400, "the placements' calls");
}

/// `magic_0162` 0x10004b60 (`r34::turn_0162`): the yaw wrapped down,
/// kept, wrapped up, on either limit and NaN; the scale task's list empty
/// and not.
#[test]
fn turn_0162() {
    let Some(mut t) = twin(162) else { return };
    model_stubs(&mut t);
    let f = t.at(0x4b60);
    let st = t.img.rel32_target(f + 0x3a9);
    let take = t.img.rel32_target(st + 0x7d);
    let (head, tail) = (t.img.word(st + 0x83), t.img.word(st + 0x9b));
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x14), s);
    let (step, hi, lo) = (kf(&t, f, 0x25), kf(&t, f, 0x2f), kf(&t, f, 0x43));
    let yaws = [
        hi - step + 0.5,
        0.25,
        lo - step - 0.5,
        f32::NAN,
        hi - step,
        lo - step,
    ];
    for (k, yaw) in (0u32..).zip(yaws) {
        junk(&mut t, s + 0x840, 0x1c0, 3 + k);
        junk(&mut t, s + 0x8460, 0x60, 7 + k);
        junk(&mut t, s + 0x85c0, 0x10, 9 + k);
        t.set_u32(s + 0x954, yaw.to_bits());
        t.set_u32(s + 0x85c8, (0.75 * k as f32 - 1.0).to_bits());
        chars(
            &mut t,
            f,
            &[0x123, 0x15d, 0x291, 0x2cc],
            &[0x12d, 0x167, 0x29c, 0x2d7],
            &[3, 0x21, 5 + k as u8],
        );
        magic_hi::pool(&mut t, take, SCRATCH + 0x5000, 3);
        magic_hi::list(&mut t, head, tail, SCRATCH + 0x5400, k % 2 == 1);
        ownb::dirty_stack(&mut t, 0x4b60 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the turn's calls");
}

/// `magic_0162` 0x10002030 (`r34::task_0162`): the first script running
/// (the points, the four motion scripts, the camera rigs' step) and done
/// (`S + 0x85ec`, the node given back).
#[test]
fn task_0162() {
    let Some(mut t) = twin(162) else { return };
    all_stubs(&mut t);
    let f = t.at(0x2030);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x7), s);
    let (s1, s2) = (t.img.word(f + 0x2e), t.img.word(f + 0x3d));
    let give = t.img.rel32_target(f + 0x151);
    let node = SCRATCH + 0x5800;
    t.set_u32(t.img.word(f + 0x26), 0x0026_1234);
    for (k, (first, second)) in (0u32..).zip([(4u8, 0u8), (0, 4), (0, 0)]) {
        let block: Vec<u8> = (0..0x30u8).map(|i| 0x51 ^ i.wrapping_mul(7)).collect();
        t.write(t.img.word(f + 0x10), &block);
        t.set_u16(t.img.word(f + 0x21), 0x1234 + k as u16);
        t.set_u8(s1 + 0x56, first);
        t.set_u8(s2 + 0x56, second);
        junk(&mut t, s + 0x800, 0x200, k);
        junk(&mut t, s + 0x85e0, 0x10, k + 1);
        for op in [0x10bu32, 0x11b, 0x125, 0x12f] {
            let ms = t.img.word(f + op);
            t.set_u32(ms + 0x14, 1);
        }
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x2030 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0162` 0x100013a0 (`r34::load_0162`, with 0x10004f70
/// `r34::start_up_0162` and its placement): the waits, the start-up with
/// the task list empty and not, the angle wrapped and kept, the summon's
/// index below 0x20 and not.
#[test]
fn load_0162() {
    let Some(mut t) = twin(162) else { return };
    model_stubs(&mut t);
    let f = t.at(0x13a0);
    let su = t.at(0x4f70);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    let pp = t.at(0x21d0);
    let lock = SCRATCH + 0x7f0;
    let pi = t.img.rel32_target(su + 0x60);
    t.set_u32(w(&t, pi, 0x04), lock);
    let take = t.img.rel32_target(su + 0xce);
    let ri = t.img.rel32_target(su + 0x10c);
    // (225, once, 227, task list, rotation, summon index)
    let cases: [(u32, u32, u32, bool, f32, u32); 6] = [
        (1, 0, 1, false, 0.0, 0),
        (0, 0, 0, false, 0.0, 0),
        (0, 0, 1, false, -10.0, 3),
        (0, 1, 2, true, 10.0, 0x21),
        (0, 0, 1, true, 3.0, 2),
        (0, 0, 1, false, 1.0, 0x20),
    ];
    for (k, (w225, once, r227, busy, rot, idx)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(ROT, rot.to_bits());
        t.set_u32(w(&t, f, 0x18), once);
        t.set_u32(lock, 0x77 + k);
        // the state block the start-up names: junk first
        let sb = w(&t, su, 0x4c);
        junk(&mut t, sb + 0x800, 0x240, k);
        junk(&mut t, sb + 0x85b0, 0x40, k + 1);
        place_globals_0162(&mut t, pp, 0, idx, k);
        junk_ops(&mut t, su, &[0x16, 0x56, 0x7f, 0xa3]);
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(w(&t, ri, 0x3e), node(0));
        t.set_u16(w(&t, ri, 0x20), 3);
        t.set_u32(w(&t, ri, 0xa6), 0);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, su, 0xd4), if busy { last } else { 0 });
        t.set_u32(w(&t, su, 0xe8), last);
        ownb::dirty_stack(&mut t, 0x13a0 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
}

/// The walk start's slots: 520 answers `SCRATCH + 0x30`, 390
/// `SCRATCH + 0x34`, 292 writes the point `V` from `SCRATCH + 0x40`, 542
/// logged; the rest as [`model_stubs`].
fn walk_stubs(t: &mut Twin) {
    model_stubs(t);
    rec(t, 520, 0, |m, _| m.r32(SCRATCH + 0x30));
    rec(t, 390, 1, |m, a| m.r32(SCRATCH + 0x34) ^ (a[0] << 24));
    rec(t, 292, 1, |m, a| {
        for i in 0..4 {
            let v = m.r32(SCRATCH + 0x40 + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x0292_0000
    });
    rec(t, 542, 2, |_, a| 0x0542_0000 ^ a[0] ^ a[1]);
}

/// A walk start's case: slot 520's answer `a`, slot 390's `r` (before the
/// selection byte), the point `V` (x, z), the actors' count for
/// `each_542` (`each` = its code), the rotation.
fn walk_case(t: &mut Twin, each: u32, (a, r): (u32, u32), (x, z): (f32, f32), n: u32, rot: f32) {
    t.set_u32(SCRATCH + 0x30, a);
    t.set_u32(SCRATCH + 0x34, r);
    for (i, v) in [x, 0.5, z, 2.0].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x40 + 4 * i as u32, v.to_bits());
    }
    // (the actors' map is the battle selection `G_SEL` in both DLLs: byte 0 stays 5)
    let map = SCRATCH + 0x600;
    t.write(map, &[5, 3, 2, 1]);
    t.set_u32(t.img.word(each + 0x1d), map);
    t.set_u32(t.img.word(each + 0x8), n);
    t.set_u32(ROT, rot.to_bits());
}

/// `magic_0391` 0x100013c0 (`r34::walk_start_0391`) and `magic_0448`
/// 0x10004cc0 (`r34::walk_start_0448`): slot 520's answer in `T1`, in
/// `T2`, 0x2050000 or none; the battle character in `T3` or not; the point
/// beyond the reach and within; no actors and three; the facing wrapped
/// both ways and kept.
#[test]
fn walk_starts() {
    for (n, rva, each_op, t1, t2, t3, sel) in [
        (
            391u16,
            0x13c0u32,
            0x22du32,
            0xb9u32,
            Some(0x104u32),
            0x151u32,
            0x130u32,
        ),
        (448, 0x4cc0, 0x13f, 0xa5, None, 0xf2, 0xd1),
    ] {
        let Some(mut t) = twin(n) else { return };
        walk_stubs(&mut t);
        let f = t.at(rva);
        let each = t.img.rel32_target(f + each_op);
        let s = SCRATCH + 0x1_0000;
        let s_op = if n == 391 { 0x18 } else { 0x8 };
        t.set_u32(t.img.word(f + s_op), s);
        chars(&mut t, f, &[sel], &[], &[5]);
        let word = |t: &Twin, op: u32, i: u32| t.u32(t.img.word(f + op) + 4 * i);
        let (t1a, t3a) = (word(&t, t1, 0), word(&t, t3, 0));
        let t2a = t2.map_or(0x0bad_0000, |o| word(&t, o, 0));
        // the r slot 390 answers for a selection byte 5
        let r3 = t3a ^ (5 << 24);
        let (k, hi, lo) = if n == 391 {
            (kf(&t, f, 0x61), kf(&t, f, 0x71), kf(&t, f, 0x85))
        } else {
            (kf(&t, f, 0x4f), kf(&t, f, 0x5e), kf(&t, f, 0x72))
        };
        let cases = [
            ((t1a, r3), (0.0f32, 0.0f32), 0u32, 0.0f32),
            ((t2a, r3), (1.0, -2.0), 3, k - hi - 1.0),
            ((t2a, 0x77), (90.0, 40.0), 3, k - lo + 1.0),
            ((0x0205_0000, r3), (3.0, 1.0), 1, 1.0),
            ((0x0c0c_0c0c, r3), (500.0, 500.0), 0, k - hi),
            ((0x0c0c_0c0c, 0x77), (0.25, 0.5), 3, k - lo),
            ((t1a, 0x77), (f32::NAN, 0.0), 1, f32::NAN),
            // later table entries; the point just beyond and within the reach
            (
                (word(&t, t1, 2), word(&t, t3, 1) ^ (5 << 24)),
                (100.5, 1.0),
                1,
                0.5,
            ),
            (
                (word(&t, t2.unwrap_or(t1), 1), word(&t, t3, 2) ^ (5 << 24)),
                (99.0, 1.0),
                2,
                0.5,
            ),
            (
                (0x0c0c_0c0c, word(&t, t3, 3) ^ (5 << 24)),
                (3.0, 101.5),
                1,
                0.5,
            ),
        ];
        for (c, (ar, xz, cnt, rot)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x240, 0xac0, c);
            walk_case(&mut t, each, ar, xz, cnt, rot);
            ownb::dirty_stack(&mut t, rva + c, &[]);
            ownb::reset_counters(&mut t);
            let port = if n == 391 {
                r34::walk_start_0391
            } else {
                r34::walk_start_0448
            };
            t.check(f, Call::default(), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(port(g, img, fr.at, fr.esp))
            });
        }
    }
}

/// `magic_0391` 0x10001210 and `magic_0448` 0x10004b10 (`r34::load_0391`,
/// `load_0448`, with the start-ups and walk starts): the waits, the
/// start-up with the task list empty and not.
#[test]
fn walk_loads() {
    for (n, rva, su_rva, mesh) in [
        (391u16, 0x1210u32, 0x3d10u32, true),
        (448, 0x4b10, 0x74e0, false),
    ] {
        let Some(mut t) = twin(n) else { return };
        walk_stubs(&mut t);
        let f = t.at(rva);
        let su = t.at(su_rva);
        let at = |o: u32| if mesh || o < 0x50 { o } else { o - 11 };
        let w = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
        let lock = SCRATCH + 0x7f0;
        let pi = t.img.rel32_target(su + 0x4c);
        t.set_u32(w(&t, pi, 0x04), lock);
        let take = t.img.rel32_target(su + at(0x9c));
        let ri = t.img.rel32_target(su + at(0xda));
        let walk = t.img.rel32_target(su + at(0x5c));
        let each_op = if mesh { 0x22d } else { 0x13f };
        let each = t.img.rel32_target(walk + each_op);
        // (225, once, 227, task list)
        let cases: [(u32, u32, u32, bool); 5] = [
            (1, 0, 1, false),
            (0, 0, 0, false),
            (0, 0, 1, false),
            (0, 1, 2, true),
            (0, 0, 1, true),
        ];
        for (k, (w225, once, r227, busy)) in (0u32..).zip(cases) {
            t.set_u32(SCRATCH + 0x10, w225);
            t.set_u32(SCRATCH + 0x1c, r227);
            t.set_u32(w(&t, f, 0x18), once);
            t.set_u32(lock, 0x77 + k);
            walk_case(&mut t, each, (0x0c0c_0c0c, 0x77), (1.0, 2.0), k % 3, 0.5);
            junk_ops(&mut t, su, &[0x16, 0x42]);
            let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
            for i in 0..3u32 {
                t.write(node(i), &[0x33; 0x40]);
                t.set_u32(node(i), node((i + 2) % 3));
                t.set_u32(node(i) + 4, node((i + 1) % 3));
            }
            t.set_u32(w(&t, ri, 0x3e), node(0));
            t.set_u16(w(&t, ri, 0x20), 3);
            t.set_u32(w(&t, ri, 0xa6), 0);
            magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
            let last = SCRATCH + 0x3400;
            t.write(last, &[0x34; 0x20]);
            t.set_u32(w(&t, su, at(0xa2)), if busy { last } else { 0 });
            t.set_u32(w(&t, su, at(0xb6)), last);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            ownb::reset_counters(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
        assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
    }
}

/// `magic_0391` 0x100016c0 and `magic_0448` 0x10004ec0
/// (`r34::circle_hit_0391`, `circle_hit_0448`): a line missing the circle,
/// touching it, crossing it in `x` and in `z` (both directions, so either
/// root is taken), and a NaN end.
#[test]
fn circle_hits() {
    use ffx_magic::Xmm;
    for (n, rva) in [(391u16, 0x16c0u32), (448, 0x4ec0)] {
        let Some(mut t) = twin(n) else { return };
        walk_stubs(&mut t);
        let f = t.at(rva);
        let r448 = f32::from_bits(t.u32(t.img.word(f + 0xd7)));
        let (out, p, q) = (SCRATCH + 0x6000, SCRATCH + 0x6100, SCRATCH + 0x6200);
        // (V x, z), P (x, z), Q (x, z), R
        let cases: [((f32, f32), (f32, f32), (f32, f32), f32); 10] = [
            ((0.0, 0.0), (-5.0, 1.0), (5.0, 1.0), 1.0),
            ((0.0, 0.0), (-5.0, 1.0), (5.0, 1.0), 0.5),
            ((0.0, 0.0), (-5.0, 1.0), (5.0, 1.5), 2.0),
            ((0.0, 0.0), (5.0, 1.5), (-5.0, 1.0), 2.0),
            ((1.0, 2.0), (1.5, -5.0), (1.25, 9.0), 3.0),
            ((1.0, 2.0), (1.25, 9.0), (1.5, -5.0), 3.0),
            ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0), 1.0),
            ((0.5, -0.5), (1e30, 1.0), (3.0, 1.0), 2.0),
            ((2.0, 3.0), (-1.0, -2.0), (4.0, 5.0), 0.75),
            ((2.0, 3.0), (4.0, 5.0), (-1.0, -2.0), 40.0),
        ];
        for (k, ((vx, vz), (px, pz), (qx, qz), r)) in (0u32..).zip(cases) {
            let r = if n == 448 && k > 0 { r448 * r } else { r };
            for (i, v) in [vx, 0.5, vz, 2.0].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x40 + 4 * i as u32, v.to_bits());
            }
            for (at, x, z) in [(p, px, pz), (q, qx, qz)] {
                for (i, v) in [x, 0.25, z, 1.0].into_iter().enumerate() {
                    t.set_u32(at + 4 * i as u32, v.to_bits());
                }
            }
            junk(&mut t, out, 0x10, k);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            let c = Call {
                ecx: out,
                edx: p,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(1.0), Xmm::f32(2.0), Xmm::f32(3.0), Xmm::f32(r)]);
            t.check(f, c, &[q], Eax::Checked, move |g, img, fr| {
                Ret::eax(if n == 391 {
                    r34::circle_hit_0391(g, img, fr)
                } else {
                    r34::circle_hit_0448(g, img, fr)
                })
            });
        }
    }
}

/// The aim's case: the walker `S + 0x7a0` (x, z), the battle character's
/// place (x, z; slot 262 for every character), the facing slot 333 gives.
fn aim_case(t: &mut Twin, s: u32, (sx, sz): (f32, f32), (cx, cz): (f32, f32), rot: f32) {
    for (i, v) in [sx, 0.75, sz, 1.0].into_iter().enumerate() {
        t.set_u32(s + 0x7a0 + 4 * i as u32, v.to_bits());
    }
    for (i, v) in [cx, 0.5, cz].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x50 + 4 * i as u32, v.to_bits());
    }
    t.set_u32(ROT, rot.to_bits());
}

/// The aims' slot 262: the place at `SCRATCH + 0x50`.
fn aim_stubs(t: &mut Twin) {
    walk_stubs(t);
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = m.r32(SCRATCH + 0x50 + 4 * i);
            m.w32(a[1 + i as usize], v);
        }
        0x262
    });
    // slot 229 / 248 with float results (no NaNs from finite inputs)
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[1] + 0x30 + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x248
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
}

/// `magic_0391` 0x10001a30 (`r34::walk_aim_0391`) and `magic_0448`
/// 0x10005230 (`r34::walk_aim_0448`): the table segment (near and far
/// end) and the turned one (both entries), the battle character all
/// around the walker (the unwrap's four ways).
#[test]
fn walk_aims() {
    for (n, rva, idx_off) in [(391u16, 0x1a30u32, 0x928u32), (448, 0x5230, 0x924)] {
        let Some(mut t) = twin(n) else { return };
        aim_stubs(&mut t);
        let f = t.at(rva);
        let s = SCRATCH + 0x1_0000;
        // the characters' tables and the caster (a second selection byte
        // that must not count)
        let (tables, caster): (&[u32], u32) = if n == 391 {
            (&[0x54, 0xd3, 0x1d4, 0x52a], 0x47c)
        } else {
            (&[0x6a, 0xde, 0x1b5, 0x4ce], 0x41f)
        };
        chars(&mut t, f, &[], tables, &[2, 7]);
        junk_ops(&mut t, f, &[caster]);
        let sel = SCRATCH + 0x600;
        let ops: &[u32] = if n == 391 {
            &[0x41, 0xbd, 0x1af, 0x514]
        } else {
            &[0x60, 0xc5, 0x194, 0x4b5]
        };
        for &o in ops {
            t.set_u32(t.img.word(f + o), sel);
        }
        let (flag_far, flag_bit) = if n == 391 {
            (0x934, 0x910)
        } else {
            (0x930, 0x90c)
        };
        let mut k = 0u32;
        for (idx, entry, far, bit) in [
            (0u32, 0u32, 0u32, 0u8),
            (1, 0, 1, 0),
            (1, 0, 1, 1),
            (0xffff_fffe, 0, 0, 0),
            (0xffff_fffe, 1, 1, 0),
        ] {
            for ang in 0..8u32 {
                let th = ang as f32 * 0.785;
                let (cx, cz) = (6.0 * th.cos(), 6.0 * th.sin());
                junk(&mut t, s + 0x700, 0x240, k);
                t.set_u32(s + idx_off, idx);
                t.set_u32(s + idx_off + 4, entry);
                t.set_u32(s + flag_far, far);
                t.set_u8(s + flag_bit, bit);
                aim_case(&mut t, s, (0.5, -0.25), (cx, cz), 0.3 * ang as f32 - 1.0);
                let e = ownb::ENTRY_ESP;
                let junk_word = if n == 391 {
                    0x90
                } else {
                    e - ((((e - 4) & !7) - 0xc8) + 0x7c)
                };
                ownb::dirty_stack(&mut t, rva + k, &[junk_word]);
                ownb::reset_counters(&mut t);
                t.check(f, Call::this(s), &[], Eax::Checked, move |g, img, fr| {
                    Ret::eax(if n == 391 {
                        r34::walk_aim_0391(g, img, fr.at, fr.esp, fr.ecx)
                    } else {
                        r34::walk_aim_0448(g, img, fr.at, fr.esp, fr.ecx)
                    })
                });
                k += 1;
            }
        }
    }
}

/// `magic_0391` 0x10002ab0 and `magic_0448` 0x100064d0
/// (`r34::walk_step_0391`, `walk_step_0448`): off, no steps left (0 and
/// below), the battle point's flag off, on with the point near and far,
/// the last step and an earlier one; the caster placed by party slot and
/// by object.
#[test]
fn walk_steps() {
    for (n, rva) in [(391u16, 0x2ab0u32), (448, 0x64d0)] {
        let Some(mut t) = twin(n) else { return };
        aim_stubs(&mut t);
        let f = t.at(rva);
        let s = SCRATCH + 0x1_0000;
        let ws = SCRATCH + 0x1_2000;
        let (on, count, w_op, flag, pm) = if n == 391 {
            (0x91cu32, 0x918u32, 0x72u32, 0xbb0u32, 0x10bu32)
        } else {
            (0x918, 0x914, 0x19d, 0xbac, 0x205)
        };
        t.set_u32(t.img.word(f + w_op), ws);
        let place = t.img.rel32_target(f + pm);
        let lim = if n == 448 {
            f32::from_bits(t.u32(t.img.word(f + 0x1fa)))
        } else {
            30.0 - f32::from_bits(t.u32(t.img.word(f + 0xfd)))
        };
        // (on, count, W flag, battle point x, place_me's battle word, me)
        let cases: [(u32, i32, u32, f32, u32, u32); 12] = [
            (0, 3, 1, 0.0, 0, 1),
            (1, 0, 1, 0.0, 0, 1),
            (1, -2, 1, 0.0, 0, 1),
            (1, 3, 0, 0.0, 0, 1),
            (1, 3, 1, 0.5, 0, 1),
            (1, 1, 1, lim + 50.0, 0, 1),
            (1, 2, 1, lim * 0.6, 0, 1),
            (1, 2, 1, lim * 0.85, 0, 1),
            (1, 2, 1, lim * 1.1, 0, 1),
            (1, 2, 1, 0.25, 1, 0x40),
            (1, 1, 0, 0.0, 0, 0x40),
            (1, 2, 1, f32::NAN, 1, 1),
        ];
        for (k, (o, c, fl, vx, battle, me)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x700, 0x240, k);
            junk(&mut t, ws + 0xb80, 0x40, k + 3);
            t.set_u32(s + on, o);
            t.set_u32(s + count, c as u32);
            t.set_u32(ws + flag, fl);
            t.set_u32(ws + 0xb88, 30.0f32.to_bits());
            for (i, v) in [1.5f32, 0.25, -2.0, 0.5, 3.0, 4.0, 0.75, 1.0]
                .into_iter()
                .enumerate()
            {
                t.set_u32(s + 0x780 + 4 * i as u32, v.to_bits());
            }
            for (i, v) in [7.0f32, 2.5, -9.0, 1.0].into_iter().enumerate() {
                t.set_u32(s + 0x7a0 + 4 * i as u32, (v + k as f32).to_bits());
            }
            for (i, v) in [vx, 0.5, 1.0, 2.0].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x40 + 4 * i as u32, v.to_bits());
            }
            for (i, v) in [0.5f32 * k as f32, 0.75, -1.5].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x50 + 4 * i as u32, v.to_bits());
            }
            t.set_u32(t.img.word(place + 0x2), battle);
            t.set_u32(t.img.word(place + 0xd), me);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            ownb::reset_counters(&mut t);
            t.check(f, Call::this(s), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(if n == 391 {
                    r34::walk_step_0391(g, img, fr.at, fr.esp, fr.ecx)
                } else {
                    r34::walk_step_0448(g, img, fr.at, fr.esp, fr.ecx)
                })
            });
        }
    }
}

/// The 0x74-byte record pool `take` searches (`sb_misc::pool_take`): `n`
/// records at `recs`, those in `busy` taken, the search from `from` (as
/// `magic_r32.rs`).
fn pool_74(t: &mut Twin, take: u32, recs: u32, n: u32, busy: &[u32], from: u32, k: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    let (p37, p61, n0f, n97, i08, pg) = (
        w(t, 0x37),
        w(t, 0x61),
        w(t, 0x0f),
        w(t, 0x97),
        w(t, 0x08),
        w(t, 0x2b),
    );
    t.set_u32(p37, recs);
    t.set_u32(p61, recs);
    for i in 0..n {
        junk(t, recs + 0x74 * i, 0x74, k + i);
        let taken = u32::from(busy.contains(&i));
        t.set_u32(recs + 0x74 * i + 0x6c, taken);
    }
    t.set_u32(n0f, n);
    t.set_u32(n97, n);
    t.set_u32(i08, from);
    t.set_u32(pg, 0);
}

/// The drops' slots: 314 / 404 random floats from their argument (ST0),
/// 12 a height, 719 a handle.
fn drop_stubs(t: &mut Twin) {
    aim_stubs(t);
    for (s, x) in [(314u32, 0.375f64), (404, -0.625)] {
        rec(t, s, 1, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            m.w32(SCRATCH + 0xa00, k + 1);
            (s << 16) ^ a[0] ^ k
        });
        t.stub_st0(s, move |m, a| {
            x * f64::from(m.r32(SCRATCH + 0xa00) % 5) + f64::from(f32::from_bits(a[0]))
        });
    }
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| 1.25 + f64::from(a[0] & 7));
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
    // slot 255 reads the point it gets
    rec(t, 255, 2, |m, a| {
        0x0255_0000
            ^ a[0]
            ^ m.r32(a[1])
            ^ m.r32(a[1] + 4).rotate_left(7)
            ^ m.r32(a[1] + 8).rotate_left(13)
    });
    for s in [11u32, 32, 33] {
        rec(t, s, 2, move |m, a| {
            (s << 16) ^ a[0] ^ m.r32(SCRATCH + 0x60)
        });
    }
}

/// `magic_0391` 0x10002bf0 / `magic_0448` 0x10006700 (`r34::drop_0391`,
/// `drop_0448`) and the curve steps 0x10002de0 / 0x100068d0
/// (`r34::curve_step_0391`, `curve_step_0448`): the pool with a free
/// record and none; the count above 0, at 0 and below, below 20 (a drop)
/// and not; the battle point's flag off and on, near and far; the caster
/// placed by party slot, standing or grounded.
#[test]
fn drops_and_curves() {
    for (n, drop_rva, curve_rva) in [(391u16, 0x2bf0u32, 0x2de0u32), (448, 0x6700, 0x68d0)] {
        let Some(mut t) = twin(n) else { return };
        drop_stubs(&mut t);
        let df = t.at(drop_rva);
        let cf = t.at(curve_rva);
        let take = t.img.rel32_target(df + if n == 391 { 0x79 } else { 0x49 });
        let recs = SCRATCH + 0x8000;
        if n == 448 {
            // the block whose `+0xc` is the handle
            t.set_u32(t.img.word(df + 0x11), SCRATCH + 0x9000);
            junk(&mut t, SCRATCH + 0x9000, 0x20, 9);
        }
        for (k, busy) in (0u32..).zip([&[][..], &[0, 1, 2][..], &[1][..]]) {
            pool_74(&mut t, take, recs, 3, busy, k % 3, k);
            t.set_u32(SCRATCH + 0xa00, k);
            ownb::dirty_stack(&mut t, drop_rva + k, &[]);
            ownb::reset_counters(&mut t);
            t.check(df, Call::default(), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(if n == 391 {
                    r34::drop_0391(g, img, fr.at, fr.esp)
                } else {
                    r34::drop_0448(g, img, fr.at, fr.esp)
                })
            });
        }
        let s = SCRATCH + 0x1_0000;
        let ws = SCRATCH + 0x1_2000;
        let at = |o: u32| {
            if n == 391 || o < 0x1c4 {
                o
            } else if o < 0x2f0 {
                o - 1
            } else {
                o - 0x10
            }
        };
        let w = |t: &Twin, o: u32| t.img.word(cf + at(o));
        t.set_u32(w(&t, 0x1c5), ws);
        let (count, total, flag) = if n == 391 {
            (0x918, 0x91c, 0xbb0)
        } else {
            (0x914, 0x918, 0xbac)
        };
        let reach = if n == 391 {
            30.0 - f32::from_bits(t.u32(t.img.word(cf + 0x2e7)))
        } else {
            f32::from_bits(t.u32(t.img.word(cf + 0x2da)))
        };
        // the battle point's x for a flat distance `reach + e` from the
        // still curve's point (0, 1): see `flat` below
        let edge = |e: f32| ((reach + e) * (reach + e) - 0.25).sqrt();
        // (count, total, flag, battle point x, battle word, me, grounded);
        // count 50 and up: a still curve at (0, y, 0.5)
        let cases: [(i32, i32, u32, f32, u32, u32, u32); 14] = [
            (50, 60, 1, edge(0.01), 0, 1, 0),
            (51, 60, 1, edge(-0.01), 0, 1, 0),
            (52, 60, 1, edge(0.2), 0, 0x20, 0),
            (53, 60, 1, edge(-0.2), 0, 0x1f, 0),
            (54, 60, 0, 1.0, 0, 0x20, 1),
            (30, 40, 0, 0.0, 0, 1, 0),
            (21, 40, 1, 0.5, 0, 1, 0),
            (20, 40, 1, reach + 60.0, 0, 1, 0),
            (5, 40, 1, 2.0, 1, 1, 0),
            (1, 8, 0, 0.0, 0, 0x40, 1),
            (0, 8, 0, 0.0, 0, 3, 0),
            (-3, 8, 1, 1.0, 1, 0x40, 1),
            (12, 7, 1, f32::NAN, 0, 0xffff_ffff, 0),
            (40, 40, 0, 0.0, 1, 2, 1),
        ];
        for (k, (c, tot, fl, vx, battle, me, ground)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x700, 0x240, k);
            junk(&mut t, ws + 0xb80, 0x40, k + 3);
            t.set_u32(s + count, c as u32);
            t.set_u32(s + total, tot as u32);
            t.set_u32(ws + flag, fl);
            t.set_u32(ws + 0xb88, 30.0f32.to_bits());
            for i in 0..8u32 {
                let v = if c >= 50 {
                    [0.0f32, 0.75, 0.5, 1.0][i as usize % 4]
                } else {
                    1.5 * i as f32 - 3.0 + 0.25 * k as f32
                };
                t.set_u32(s + 0x7b0 + 4 * i, v.to_bits());
            }
            for (i, v) in [vx, 0.5, 1.0, 2.0].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x40 + 4 * i as u32, v.to_bits());
            }
            t.set_u32(w(&t, 0x2f6), battle);
            t.set_u32(w(&t, 0x2fe), me);
            t.set_u32(w(&t, 0x326), 0x0bb0_0000 + k);
            t.set_u32(SCRATCH + 0x60, 0);
            t.stub(11, move |_, _| ground);
            pool_74(&mut t, take, recs, 3, &[], 0, k);
            ownb::dirty_stack(&mut t, curve_rva + k, &[]);
            ownb::reset_counters(&mut t);
            t.check(cf, Call::this(s), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(if n == 391 {
                    r34::curve_step_0391(g, img, fr.at, fr.esp, fr.ecx)
                } else {
                    r34::curve_step_0448(g, img, fr.at, fr.esp, fr.ecx)
                })
            });
        }
    }
}
