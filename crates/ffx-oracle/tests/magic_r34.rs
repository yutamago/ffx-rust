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

/// `magic_0391` 0x10002050 and `magic_0448` 0x10005800
/// (`r34::walk_plan_0391`, `walk_plan_0448`): an aim set (the aim runs);
/// the step before the last and at the end; a next character (both
/// yaw orders, close and far apart) and none (at the first step: slot
/// 324); the reach off, tiny (every test beyond), huge (none) and in
/// between, so each turn of the move and the circle crossing run.
#[test]
fn walk_plans() {
    for (n, rva) in [(391u16, 0x2050u32), (448, 0x5800)] {
        let Some(mut t) = twin(n) else { return };
        aim_stubs(&mut t);
        for (s, x) in [(251u32, 0.75f64), (19, 1.25)] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
            t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
        }
        let f = t.at(rva);
        let s = SCRATCH + 0x1_0000;
        let st = |o: u32| s + if n == 448 { o - 4 } else { o };
        let at = |o: u32| -> u32 {
            if n == 391 {
                return o;
            }
            let runs = [
                (0x5f1u32, 7u32),
                (0x60e, 0xa),
                (0x68d, 0xe),
                (0x852, 0x15),
                (0x86b, 0x18),
                (0x89c, 0x1c),
            ];
            let d = runs
                .iter()
                .rev()
                .find(|&&(b, _)| b <= o)
                .map_or(0, |&(_, d)| d);
            o - d
        };
        // the selection table and the characters
        let sel = SCRATCH + 0x600;
        t.write(sel, &[3, 5, 7, 9, 11, 13, 15, 17]);
        for o in [0xb1u32, 0x1b5] {
            t.set_u32(t.img.word(f + at(o)), sel);
        }
        for o in [0xbcu32, 0x1c0] {
            for j in 0..0x20 {
                t.set_u32(t.img.word(f + at(o)) + 4 * j, CHRS + 0x10 * j);
            }
        }
        junk_ops(&mut t, f, &[at(0x913)]);
        let rs: [f32; 10] = [0.01, 1.0e6, 3.0, 8.0, 20.0, 0.5, 1.5, 2.5, 5.0, 12.0];
        // (aim index, i, n, bytes, enable, R index, flag, slot 324, V x)
        let mut cases: Vec<(u32, i32, i32, [u8; 3], u32, usize, u32, u32, f32)> = vec![
            (0, 0, 3, [1, 2, 3], 0, 0, 0, 0, 0.0),
            (u32::MAX, 0, 3, [1, 2, 3], 0, 0, 0, 0, 0.0),
            (u32::MAX, 1, 3, [1, 2, 3], 1, 0, 1, 0, 5.0),
            (u32::MAX, 0, 3, [1, 1, 3], 1, 1, 0, 1, 5.0),
            (u32::MAX, 0, 1, [4, 2, 3], 1, 2, 1, 0xfe, -3.0),
            (u32::MAX, 2, 3, [1, 2, 6], 1, 3, 0, 0, 12.0),
            (u32::MAX, 3, 3, [1, 2, 6], 1, 4, 1, 0, 30.0),
            (u32::MAX, 0, 2, [6, 1, 0], 1, 0, 1, 0, 40.0),
            (u32::MAX, 0, 2, [1, 6, 0], 1, 2, 0, 0, 2.0),
            (u32::MAX, 1, 4, [0, 2, 7], 1, 3, 0, 0, -9.0),
            (u32::MAX, 1, 4, [0, 7, 2], 1, 4, 1, 0, 9.0),
            (u32::MAX, 0, 4, [5, 3, 0], 1, 5, 0, 0, 1.0),
            (u32::MAX, 0, 4, [3, 5, 0], 1, 5, 1, 0, 25.0),
            (u32::MAX, 2, 4, [0, 0, 2], 0, 2, 0, 0, 1.0),
            (u32::MAX, 0, 4, [2, 2, 2], 1, 3, 1, 2, 6.0),
            (u32::MAX, 0, 4, [7, 0, 1], 1, 2, 0, 3, 3.5),
        ];
        // the reach's tests near their edges: more reaches and points
        for r in 6..10usize {
            for (j, vx) in [-4.0f32, -1.5, 0.75, 2.25, 6.5].into_iter().enumerate() {
                let fl = u32::try_from(j % 2).unwrap_or(0);
                cases.push((u32::MAX, 1, 4, [0, 2, 7], 1, r, fl, 0, vx));
            }
        }
        for (k, (aim, i, cnt, bytes, en, ri, fl, s324, vx)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x700, 0x260, k);
            t.set_u32(st(0x928), aim);
            t.set_u32(st(0x910), i as u32);
            t.set_u32(st(0x914), cnt as u32);
            let off = u32::try_from(i.max(0)).unwrap_or(0);
            t.write(st(0x93c) + off, &bytes);
            t.set_u32(st(0x930), en);
            t.set_u32(st(0x90c), fl);
            t.set_u32(s + 0x908, rs[ri].to_bits());
            for (j, v) in [0.5f32, 0.25, -1.5, 1.0].into_iter().enumerate() {
                t.set_u32(s + 0x7a0 + 4 * j as u32, (v + 0.125 * k as f32).to_bits());
            }
            for (j, v) in [vx, 0.5, 1.0 - 0.5 * k as f32, 2.0].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x40 + 4 * j as u32, v.to_bits());
            }
            for (j, v) in [2.0f32 + 0.5 * k as f32, 0.75, -3.0 + k as f32]
                .into_iter()
                .enumerate()
            {
                t.set_u32(SCRATCH + 0x50 + 4 * j as u32, v.to_bits());
            }
            t.stub(324, move |_, _| s324);
            t.set_u32(ROT, (0.3 * k as f32 - 1.0).to_bits());
            let e = ownb::ENTRY_ESP;
            let e1 = ((e - 4) & !7) - 0xe8;
            let q_w = e - (e1 + 0x5c);
            // the aim's own never-written frame word (`walk_aim_*`)
            let ea = e1 - 4;
            let aim_w = if n == 391 {
                e - (ea - 4 - 0x8c)
            } else {
                e - ((((ea - 4) & !7) - 0xc8) + 0x7c)
            };
            ownb::dirty_stack(&mut t, rva + k, &[q_w, aim_w]);
            ownb::reset_counters(&mut t);
            t.check(f, Call::this(s), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(if n == 391 {
                    r34::walk_plan_0391(g, img, fr.at, fr.esp, fr.ecx)
                } else {
                    r34::walk_plan_0448(g, img, fr.at, fr.esp, fr.ecx)
                })
            });
        }
    }
}

/// The walk plan's set-up for a start's case `k` (`plan` = the plan's
/// code): no aim, the step `i` of 3 with a next character, the reach and
/// the battle point by `k`.
fn plan_env(t: &mut Twin, n: u16, plan: u32, s: u32, i: i32, k: u32) {
    let st = |o: u32| s + if n == 448 { o - 4 } else { o };
    let at = |o: u32| -> u32 {
        if n == 391 {
            return o;
        }
        let runs = [
            (0x5f1u32, 7u32),
            (0x60e, 0xa),
            (0x68d, 0xe),
            (0x852, 0x15),
            (0x86b, 0x18),
            (0x89c, 0x1c),
        ];
        let d = runs
            .iter()
            .rev()
            .find(|&&(b, _)| b <= o)
            .map_or(0, |&(_, d)| d);
        o - d
    };
    let sel = SCRATCH + 0x600;
    t.write(sel, &[3, 5, 7, 9, 11, 13, 15, 17]);
    for o in [0xb1u32, 0x1b5] {
        t.set_u32(t.img.word(plan + at(o)), sel);
    }
    for o in [0xbcu32, 0x1c0] {
        for j in 0..0x20 {
            t.set_u32(t.img.word(plan + at(o)) + 4 * j, CHRS + 0x10 * j);
        }
    }
    junk(t, s + 0x700, 0x260, k);
    t.set_u32(st(0x928), u32::MAX);
    t.set_u32(st(0x910), i as u32);
    t.set_u32(st(0x914), 3);
    t.write(st(0x93c), &[1, 4, 2, 6]);
    t.set_u32(st(0x930), k % 2);
    t.set_u32(st(0x90c), k / 2 % 2);
    t.set_u32(s + 0x908, [0.5f32, 30.0, 4.0][k as usize % 3].to_bits());
    for (j, v) in [0.5f32, 0.25, -1.5, 1.0].into_iter().enumerate() {
        t.set_u32(s + 0x7a0 + 4 * j as u32, (v + 0.375 * k as f32).to_bits());
    }
    for (j, v) in [3.0f32 - k as f32, 0.5, 1.0, 2.0].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x40 + 4 * j as u32, v.to_bits());
    }
    for (j, v) in [2.0f32 + 0.5 * k as f32, 0.75, -3.0 + k as f32]
        .into_iter()
        .enumerate()
    {
        t.set_u32(SCRATCH + 0x50 + 4 * j as u32, v.to_bits());
    }
}

/// `magic_0391` 0x100034c0 / 0x100031d0 and `magic_0448` 0x10006f90 /
/// 0x10006c80 (`r34::turn_start_*`, `spark_start_*`): the plan's cases
/// (the target yaw below the facing and not, both unwrap ways), in a
/// battle and not, the old flag set and not; the spark object's pool with
/// a free node.
#[test]
fn walk_turn_starts() {
    use ffx_magic::sl_host;
    for (n, plan_rva, turn_rva, spark_rva) in [
        (391u16, 0x2050u32, 0x34c0u32, 0x31d0u32),
        (448, 0x5800, 0x6f90, 0x6c80),
    ] {
        let Some(mut t) = twin(n) else { return };
        aim_stubs(&mut t);
        for (s, x) in [(251u32, 0.75f64), (19, 1.25), (314, 0.0625)] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
            t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
        }
        rec(&mut t, 268, 3, |_, a| 0x0268_0000 ^ a[1]);
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
        rec(&mut t, 306, 3, |_, a| 0x0306_0000 ^ a[2]);
        for s in [215u32, 413] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        }
        let plan = t.at(plan_rva);
        let s = SCRATCH + 0x1_0000;
        let (tf, sf) = (t.at(turn_rva), t.at(spark_rva));
        // the hosted object's environment (as `magic_sl_b6.rs`)
        let ho = t.img.rel32_target(sf + 0x15a);
        let l = if n == 391 {
            &sl_host::L0391
        } else {
            &sl_host::L0448
        };
        let hw = |t: &Twin, o: u32| t.img.word(ho + o);
        let sp = t.img.rel32_target(ho + l.calls.0);
        t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(sp + 0x29), SCRATCH + 0x200);
        t.set_u32(SCRATCH + 0x200, 0x0000_0077);
        rec(&mut t, 714, 5, |_, a| a[2] ^ 0x714);
        let table = SCRATCH + 0x400;
        for i in 0..4 {
            t.set_u32(table + 4 * i, 0x0900_0000 + i);
        }
        rec(&mut t, 941, 1, move |_, _| table);
        t.set_u32(hw(&t, l.host.0), SCRATCH + 0x300);
        let host = SCRATCH + 0x340;
        t.set_u32(hw(&t, l.host.1), host);
        t.set_u32(host + 0x10, 0x00d0_0010);
        let pa = t.img.rel32_target(ho + l.calls.2);
        let nodes = t.img.word(pa + 0x27) - 8;
        for k in 0..6u32 {
            for (f, spark) in [(tf, false), (sf, true)] {
                plan_env(&mut t, n, plan, s, 1 + (k % 2) as i32, k);
                junk_ops(
                    &mut t,
                    f,
                    if spark {
                        &[0x98, 0xab, 0xb0, 0xb5, 0xe1, 0xe7, 0xf1, 0xfb, 0x105, 0x10e]
                    } else {
                        &[0x85, 0x8a, 0x8f, 0x9b, 0xc5, 0xcb, 0xd5, 0xdf, 0xe9, 0xf2]
                    },
                );
                // the caster's facing all around (both unwrap ways)
                t.set_u32(
                    ROT,
                    [-3.0f32, 3.0, 2.5, -2.5, 1.0, -1.0][k as usize].to_bits(),
                );
                let battle = if spark { [0xa5u32, 0xd6] } else { [0x7f, 0xba] };
                for o in battle {
                    t.set_u32(t.img.word(f + o), k / 3);
                }
                if spark {
                    for i in 0..8 {
                        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                        t.set_u32(nodes + 12 * i + 8, 0);
                    }
                    t.set_u16(t.img.word(pa + 0x04), 3);
                    t.set_u32(hw(&t, l.list.0), 0);
                    t.set_u32(hw(&t, l.list.1), 0);
                    t.set_u32(hw(&t, l.list.2), 5);
                    t.write(SCRATCH + 0x600, &[3, 5, 7, 9, 11, 13, 15, 17]);
                    for j in 0..0x20 {
                        t.set_u32(t.img.word(f + 0x18b) + 4 * j, CHRS + 0x10 * j);
                    }
                    t.set_u32(t.img.word(f + 0x180), SCRATCH + 0x600);
                    t.set_u32(t.img.word(f + 0x1eb), SCRATCH + 0x600);
                }
                let e = ownb::ENTRY_ESP;
                // the plan's Q.w (its frame below the start's)
                let pe = if spark { e - 0x18 } else { e - 0xc };
                let q_w = e - ((((pe - 4) & !7) - 0xe8) + 0x5c);
                ownb::dirty_stack(&mut t, f + k, &[q_w]);
                ownb::reset_counters(&mut t);
                t.check(f, Call::this(s), &[], Eax::Checked, move |g, img, fr| {
                    Ret::eax(match (n, spark) {
                        (391, false) => r34::turn_start_0391(g, img, fr.at, fr.esp, fr.ecx),
                        (391, true) => r34::spark_start_0391(g, img, fr.at, fr.esp, fr.ecx),
                        (_, false) => r34::turn_start_0448(g, img, fr.at, fr.esp, fr.ecx),
                        _ => r34::spark_start_0448(g, img, fr.at, fr.esp, fr.ecx),
                    })
                });
            }
        }
    }
}

/// `magic_0391` 0x10003630 and `magic_0448` 0x100061b0
/// (`r34::walk_reset_0391`, `walk_reset_0448`): the first step (the plan
/// and the motion block, in a battle and not) and a later one; the
/// distance giving a few steps, the 300 cap, NaN.
#[test]
fn walk_resets() {
    for (n, plan_rva, rva) in [(391u16, 0x2050u32, 0x3630u32), (448, 0x5800, 0x61b0)] {
        let Some(mut t) = twin(n) else { return };
        aim_stubs(&mut t);
        for (s, x) in [(251u32, 0.75f64), (19, 1.25)] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
            t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
        }
        for s in [215u32, 413] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        }
        let plan = t.at(plan_rva);
        let f = t.at(rva);
        let s = SCRATCH + 0x1_0000;
        let (i_at, motion): (u32, &[u32]) = if n == 391 {
            (0x910, &[0xa7, 0xac, 0xb8, 0xc2, 0xec, 0xf2, 0xfc, 0x106])
        } else {
            (0x90c, &[0xa4, 0xa9, 0xb5, 0xbf, 0xe9, 0xef, 0xf9, 0x103])
        };
        let battle: [u32; 2] = if n == 391 { [0xa1, 0xe1] } else { [0x9e, 0xde] };
        // (first step, battle, the battle character's x and z)
        let cases: [(bool, u32, f32, f32); 6] = [
            (true, 0, 3.0, 4.0),
            (true, 1, -20.0, 7.5),
            (false, 0, 1.0e6, 0.0),
            (false, 1, 0.25, -0.5),
            (true, 0, f32::NAN, 1.0),
            (false, 0, 60.0, 80.0),
        ];
        for (k, (first, war, x, z)) in (0u32..).zip(cases) {
            plan_env(&mut t, n, plan, s, 1, k);
            t.set_u32(s + i_at, u32::from(!first) * 2);
            junk_ops(&mut t, f, motion);
            for o in battle {
                t.set_u32(t.img.word(f + o), war);
            }
            for (j, v) in [x, 0.75 + k as f32, z].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x50 + 4 * j as u32, v.to_bits());
            }
            let e = ownb::ENTRY_ESP;
            let frame = if n == 391 { 0x88 } else { 0x20 };
            let pe = ((e - 4) & !7) - frame - 4;
            let q_w = e - ((((pe - 4) & !7) - 0xe8) + 0x5c);
            ownb::dirty_stack(&mut t, rva + k, &[q_w]);
            ownb::reset_counters(&mut t);
            t.check(f, Call::this(s), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(if n == 391 {
                    r34::walk_reset_0391(g, img, fr.at, fr.esp, fr.ecx)
                } else {
                    r34::walk_reset_0448(g, img, fr.at, fr.esp, fr.ecx)
                })
            });
        }
    }
}

/// `magic_0391` 0x10003990 and `magic_0448` 0x10007100
/// (`r34::walk_task_0391`, `walk_task_0448`): every state (and one past
/// the table), near and far, the curve done with steps left and not, the
/// handle call at step 23; the turn's easing count above 0 and not.
#[test]
fn walk_tasks() {
    use ffx_magic::sl_host;
    for (n, rva) in [(391u16, 0x3990u32), (448, 0x7100)] {
        let Some(mut t) = twin(n) else { return };
        drop_stubs(&mut t);
        for (s, x) in [(251u32, 0.75f64), (19, 1.25)] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
            t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
        }
        rec(&mut t, 268, 3, |_, a| 0x0268_0000 ^ a[1]);
        rec(&mut t, 306, 3, |_, a| 0x0306_0000 ^ a[2]);
        rec(&mut t, 393, 3, |_, a| 0x0393_0000 ^ a[1] ^ a[2]);
        rec(&mut t, 213, 2, |_, a| 0x0213_0000 ^ a[1]);
        rec(&mut t, 34, 2, |_, a| 0x0034_0000 ^ a[1]);
        for s in [215u32, 413] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        }
        let f = t.at(rva);
        let s = SCRATCH + 0x1_0000;
        // `*G_S` is the steps' and curves' `W` too: `S = W + 0x280`
        let ws = s - 0x280;
        t.set_u32(t.img.word(f + 0x1f), ws);
        let (plan_rva, step_rva, curve_rva, drop_rva, spark_rva) = if n == 391 {
            (0x2050u32, 0x2ab0u32, 0x2de0u32, 0x2bf0u32, 0x31d0u32)
        } else {
            (0x5800, 0x64d0, 0x68d0, 0x6700, 0x6c80)
        };
        let plan = t.at(plan_rva);
        let (step, curve, drop, spark) = (
            t.at(step_rva),
            t.at(curve_rva),
            t.at(drop_rva),
            t.at(spark_rva),
        );
        // the walk state block `W` of the steps and curves
        let (w_step, w_curve) = if n == 391 {
            (0x72u32, 0x1c5u32)
        } else {
            (0x19d, 0x1c4)
        };
        t.set_u32(t.img.word(step + w_step), ws);
        t.set_u32(t.img.word(curve + w_curve), ws);
        if n == 448 {
            t.set_u32(t.img.word(drop + 0x11), SCRATCH + 0x9000);
        }
        let take = t
            .img
            .rel32_target(drop + if n == 391 { 0x79 } else { 0x49 });
        // the hosted object (as `walk_turn_starts`)
        let ho = t.img.rel32_target(spark + 0x15a);
        let l = if n == 391 {
            &sl_host::L0391
        } else {
            &sl_host::L0448
        };
        let hw = |t: &Twin, o: u32| t.img.word(ho + o);
        let sp = t.img.rel32_target(ho + l.calls.0);
        t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(sp + 0x29), SCRATCH + 0x200);
        rec(&mut t, 714, 5, |_, a| a[2] ^ 0x714);
        rec(&mut t, 941, 1, move |_, _| SCRATCH + 0x400);
        t.set_u32(hw(&t, l.host.0), SCRATCH + 0x300);
        t.set_u32(hw(&t, l.host.1), SCRATCH + 0x340);
        let pa = t.img.rel32_target(ho + l.calls.2);
        let nodes = t.img.word(pa + 0x27) - 8;
        let st = |o: u32| s + if n == 448 { o - 4 } else { o };
        let node = SCRATCH + 0x5800;
        // (state, i, count, curve steps left, easing count, caster x)
        let cases: [(u32, i32, i32, i32, i32, f32); 14] = [
            (0, 0, 3, 5, 0, 0.0),
            (0, 1, 3, 5, 4, 0.0),
            (1, 1, 3, 5, 0, 0.0),
            (1, 1, 3, 0, 2, 900.0),
            (1, 1, 3, 5, 0, 900.0),
            (2, 1, 3, 1, 0, 0.0),
            (2, 3, 3, 1, 3, 0.0),
            (2, 1, 3, 0x18, 0, 0.0),
            (2, 1, 3, 30, 1, 0.0),
            (3, 0, 3, 5, 0, 0.0),
            (4, 1, 3, 5, 6, 0.0),
            (4, 1, 3, 0, 0, 900.0),
            (4, 1, 3, 5, 0, 900.0),
            (7, 1, 3, 5, 5, 0.0),
        ];
        for (k, (state, i, cnt, left, ease, cx)) in (0u32..).zip(cases) {
            // 0448: after a step the plan's never-written `Q.w` holds the
            // step's own frame word (the original reads it, the port 0:
            // brief 2026-10-24), so no start right after a step here (the
            // starts are tested on their own in `walk_turn_starts`)
            let (left, cx) = if n == 448 && (state == 1 || state == 4) {
                (left.max(5), 900.0)
            } else {
                (left, cx)
            };
            plan_env(&mut t, n, plan, s, i, k);
            t.set_u32(st(0x910), i as u32);
            t.set_u32(st(0x914), cnt as u32);
            t.set_u32(st(0x924), state);
            t.set_u32(st(0x920), ease as u32);
            // the step and curve counts (`S + 0x918`/`+0x91c` in 0391)
            let (c_at, tot_at) = if n == 391 {
                (0x918, 0x91c)
            } else {
                (0x914, 0x918)
            };
            t.set_u32(s + c_at, left as u32);
            t.set_u32(s + tot_at, 40);
            for (j, v) in [cx, 0.75, -1.0 + 0.5 * k as f32].into_iter().enumerate() {
                t.set_u32(SCRATCH + 0x50 + 4 * j as u32, v.to_bits());
            }
            pool_74(&mut t, take, SCRATCH + 0x8000, 3, &[], 0, k);
            for j in 0..8 {
                t.set_u32(nodes + 12 * j, 0x0dd0_0000 + j);
                t.set_u32(nodes + 12 * j + 4, 0x0ee0_0000 + j);
                t.set_u32(nodes + 12 * j + 8, 0);
            }
            t.set_u16(t.img.word(pa + 0x04), 3);
            t.set_u32(hw(&t, l.list.0), 0);
            t.set_u32(hw(&t, l.list.1), 0);
            t.write(node, &[0x3c; 0x60]);
            let e = ownb::ENTRY_ESP;
            // the plan's Q.w under each caller (the task's internal callees
            // run at E1 - 4)
            let e1 = ((e - 4) & !7) - 0x30;
            let ce = e1 - 4;
            let reset_frame = if n == 391 { 0x88 } else { 0x20 };
            let zero: Vec<u32> = [ce - 0x18, ce - 0xc, ((ce - 4) & !7) - reset_frame - 4]
                .into_iter()
                .map(|pe| e - ((((pe - 4) & !7) - 0xe8) + 0x5c))
                .collect();
            ownb::dirty_stack(&mut t, rva + k, &zero);
            ownb::reset_counters(&mut t);
            t.check_entry(f, Call::default(), &[node], Eax::Checked);
        }
    }
}

/// `magic_0284` 0x10001bd0 (`r34::main_task_0284`): the motion, the
/// script and the objects each not done (the objects' answer returned),
/// and all done: the node moved on, the grid and the state tables
/// registered, slot 713 over 0, 1 and 3 records.
#[test]
fn main_task_0284() {
    let Some(mut t) = twin(284) else { return };
    model_stubs(&mut t);
    rec(&mut t, 713, 1, |m, a| 0x0713_0000 ^ a[0] ^ m.r32(a[0]));
    rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[0] ^ a[1]);
    let f = t.at(0x1bd0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let ms = t.img.rel32_target(f + 0x1b);
    let ro = t.img.rel32_target(f + 0x90);
    let list = t.img.word(ro + 0x7c);
    let obj_p = w(&t, 0x6a);
    let (node, nblock, objn, obj) = (
        SCRATCH + 0x9f00,
        SCRATCH + 0x9f80,
        SCRATCH + 0xa800,
        SCRATCH + 0xa000,
    );
    let recs = SCRATCH + 0xb000;
    let xp = SCRATCH + 0xc000;
    // the grid's and the handles' registration (as `magic_sa_misc.rs`)
    let grid = t.img.rel32_target(f + 0xbf);
    let r8 = t.img.rel32_target(f + 0xd5);
    let lock = SCRATCH + 0x800;
    t.set_u32(t.img.word(grid + 0x53), lock);
    t.set_u32(t.img.word(r8 + 0x21), lock);
    t.set_u32(t.img.word(grid + 0x05), 2);
    let grecs = SCRATCH + 0xd000;
    t.set_u32(t.img.word(grid + 0x0c), grecs);
    for k in 0..4 {
        t.set_u32(grecs + 0x68 * k, 0x0300_0000 + k);
    }
    rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(&mut t, 779, 4, |m, a| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        a[0]
    });
    // the two state tables' blocks
    let mut blk = SCRATCH + 0xe000;
    for (a, b) in [(0xc4u32, 0xde), (0xe5, 0x15f)] {
        let mut at = w(&t, a);
        while at < w(&t, b) {
            t.set_u32(at, blk);
            junk(&mut t, blk, 0x290, at);
            blk += 0x300;
            at += 4;
        }
    }
    // (motion done, script flag 0x8000, objects busy, records)
    let cases: [(u32, u16, bool, i32); 6] = [
        (0, 0x8000, false, 1),
        (1, 0, false, 1),
        (1, 0x8000, true, 1),
        (1, 0x8000, false, 0),
        (1, 0x8000, false, 1),
        (1, 0xc000, false, 3),
    ];
    for (k, (done, flag, busy, nrec)) in (0u32..).zip(cases) {
        t.set_u32(t.img.word(ms + 0x02), done);
        t.set_u16(t.img.word(ms + 0x11), 5);
        junk(&mut t, obj_p + 0x10, 0x24, k);
        t.set_u32(obj_p + 0x10, 0);
        t.set_u32(obj_p + 0x2c, 3);
        t.set_u16(obj_p + 0x30, flag);
        t.set_u32(objn, 0);
        t.set_u32(objn + 4, 0);
        t.set_u32(objn + 8, obj);
        t.set_u16(obj + 0x124, 0x8000);
        t.set_u32(obj + 0x114, 5);
        t.set_u32(list + 8, if busy { objn } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { objn } else { 0 });
        junk(&mut t, node, 0x20, k);
        t.set_u32(node + 8, nblock);
        junk(&mut t, nblock, 0x20, k + 1);
        t.set_u32(w(&t, 0x167), recs);
        junk(&mut t, recs, 0x140 * 3, k + 2);
        t.set_u32(w(&t, 0x16f), nrec as u32);
        t.set_u32(w(&t, 0x181), xp);
        t.set_u32(xp, 0x5a5a_0000 + k);
        junk_ops(&mut t, f, &[0x4e, 0x5c, 0x7a, 0x80]);
        t.set_u32(lock, 0x5555 + k);
        ownb::dirty_stack(&mut t, 0x1bd0 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0434` 0x100097f0 (`r34::check_targets_0434`) and its object
/// host start 0x100010a0 (`l5_objhost::start`): slot 390 answering
/// 0x3008 and not, `*G_P` set and 0; no targets, a target with a listed
/// kind, one slot 580 rejects, all accepted.
#[test]
fn check_targets_0434() {
    let Some(mut t) = twin(434) else { return };
    let at = t.at(0x97f0);
    let st = t.at(0x10a0);
    for (s, n) in [
        (4u32, 1usize),
        (27, 3),
        (523, 2),
        (556, 1),
        (555, 2),
        (331, 3),
        (720, 2),
        (718, 3),
        (222, 7),
        (221, 11),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let cell = SCRATCH + 0x10;
    rec(&mut t, 580, 1, move |m, a| u32::from(m.r32(cell) == a[0]));
    rec(&mut t, 390, 1, |m, a| m.r32(SCRATCH + 0x14) ^ a[0]);
    let bytes = SCRATCH + 0x2000;
    let count = SCRATCH + 0x20;
    rec(&mut t, 168, 1, |_, _| SCRATCH + 0x1000);
    rec(&mut t, 218, 2, move |m, a| {
        let n = m.r32(count);
        m.w32(a[1], n);
        bytes
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 + a[0]);
    rec(&mut t, 413, 1, |_, a| 0x0413_0000 + a[0]);
    rec(&mut t, 291, 5, |_, a| 0x10 + a[1]);
    rec(&mut t, 367, 8, |_, a| 0x0367_0000 + a[1]);
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.1);
    let list = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x93), list);
    t.write(list, &[1, 2, 3, 4]);
    let objs = t.img.word(at + 0x9e);
    let listed = t.u32(t.img.word(at + 0xa9) + 8);
    let who = t.img.word(at + 0x14);
    let payloads = t.img.word(st + 0x81);
    let blk = SCRATCH + 0x4_0000;
    // (targets, listed kind, rejected id, slot 390's code, *G_P set)
    let cases: [(i32, Option<u32>, u32, bool, bool); 6] = [
        (0, None, 0, true, false),
        (4, Some(3), 0, false, true),
        (4, None, 2, true, true),
        (4, None, 0, false, false),
        (1, None, 0, true, true),
        (-2, None, 0, false, true),
    ];
    for (k, (n, kind, reject, code, p_set)) in (0u32..).zip(cases) {
        for whole in [false, true] {
            t.set_u32(who, 0x0abc_0000 + k);
            t.set_u32(
                SCRATCH + 0x14,
                if code { 0x3008 ^ (0x0abc_0000 + k) } else { 7 },
            );
            t.set_u32(t.img.word(at + 0x48), if p_set { blk } else { 0 });
            t.set_u32(t.img.word(at + 0x76), n.cast_unsigned());
            // (every id the start's byte list can name has an object)
            for id in 0..16u32 {
                let o = SCRATCH + 0x400 + 0x10 * id;
                t.set_u32(objs + 4 * id, o);
                t.set_u32(o, 0x5a5a_fff0 + id);
                let kd = if kind == Some(id) {
                    listed
                } else {
                    0xfff0 + id
                };
                t.set_u16(o, u16::try_from(kd & 0xffff).unwrap_or(0));
            }
            t.set_u32(t.img.word(at + 0x93), list);
            t.set_u32(cell, reject);
            if whole {
                t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
                t.set_u32(count, k * 2);
                for (i, b) in [3u8, 1, 4, 1, 5, 9, 2, 6].iter().enumerate() {
                    t.set_u8(bytes + i as u32, *b);
                }
                let c = Call {
                    ecx: 0x55,
                    ..Call::default()
                };
                t.check_entry(st, c, &[0x00ab_c000 + k], Eax::Checked);
            } else {
                t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
                    Ret::eax(r34::check_targets_0434(g, img, fr))
                });
            }
        }
    }
}
