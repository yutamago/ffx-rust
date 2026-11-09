//! Round 28's ports (`ffx_magic::r28`) against each original. Needs
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

use ffx_magic::{Ret, r28};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{filler, rec, twin};

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// `n` floats at `at`: `a + b i`.
fn floats(t: &mut Twin, at: u32, n: u32, (a, b): (f32, f32)) {
    for i in 0..n {
        t.set_u32(at + 4 * i, (a + b * i as f32).to_bits());
    }
}

/// `magic_0563` 0x100045f0 and `magic_0612` 0x10008f20
/// (`r28::spin_step_0563`, internal): the count running and running out;
/// the task list empty and not; the ring with plenty of nodes and with
/// exactly three (the last take empties it); no, one and three target
/// bytes (and a negative count).
#[test]
fn spin_step_0563() {
    for (n, addr) in [(563u16, 0x45f0u32), (612, 0x8f20)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        filler(&mut t, 235, 1, 16);
        for s in [328u32, 231] {
            rec(&mut t, s, 3, move |m, a| {
                for i in 0..4 {
                    let v = rf(m, a[1] + 4 * i) * 0.5 + f32::from_bits(a[2] | 1).abs().min(4.0)
                        - i as f32;
                    wf(m, a[0] + 4 * i, v);
                }
                s ^ a[2]
            });
        }
        rec(&mut t, 229, 3, |m, a| {
            for i in 0..4 {
                let v = rf(m, a[2] + 4 * i) * 0.5
                    + (rf(m, a[1] + 4 * i)
                        + rf(m, a[1] + 0x10 + 4 * i)
                        + rf(m, a[1] + 0x20 + 4 * i))
                        * 0.25
                    + 0.5
                    + i as f32;
                wf(m, a[0] + 4 * i, v);
            }
            0x229
        });
        rec(&mut t, 314, 1, |m, a| {
            let k = m.r32(magic_owna::row(314));
            0x0314_0000 ^ k ^ a[0]
        });
        t.stub_st0(314, |m, a| {
            let k = m.r32(magic_owna::row(314));
            f64::from(f32::from_bits(a[0])) * (f64::from(k % 7) * 0.15 + 0.1) - 0.03
        });
        rec(&mut t, 388, 3, |_, a| 0x0388_0000 ^ a[0]);
        magic_r11::object_slots(&mut t);
        let start = t.img.rel32_target(f + 0x265);
        magic_r11::object_start_auto(&mut t, start);
        let s = SCRATCH + 0x1_0000;
        let ring = SCRATCH + 0x8000;
        let list = SCRATCH + 0x7c00;
        let bytes = SCRATCH + 0x7e00;
        // (count, listed, free nodes, target bytes)
        let cases: [(u32, bool, u16, i32); 6] = [
            (3, false, 8, 0),
            (1, false, 8, 0),
            (1, true, 3, 3),
            (1, true, 8, 1),
            (1, false, 3, -2),
            (0, true, 8, 3),
        ];
        for (k, (count, listed, free, nb)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x900, 0x3300, k);
            floats(&mut t, s + 0x3580, 12, (0.5 + k as f32, -0.75));
            floats(&mut t, s + 0x3620, 4, (2.0, 1.5 - k as f32));
            floats(&mut t, s + 0x35a0, 4, (-1.0, 0.625));
            floats(&mut t, s + 0x2b40, 4, (7.0, -2.5));
            t.set_u32(s + 0x3660, count);
            t.set_u32(op(&t, 0x13), s);
            let (cnt, head) = (op(&t, 0x26c), op(&t, 0x32c));
            magic_r11::ring(&mut t, cnt, head, free, ring);
            junk(&mut t, list, 0x40, k + 3);
            t.set_u32(op(&t, 0x38e), if listed { list } else { 0 });
            t.set_u32(op(&t, 0x3a0), list + 0x20);
            t.set_u32(op(&t, 0x4c3), nb.cast_unsigned());
            t.set_u32(op(&t, 0x4dd), bytes);
            for i in 0..8u8 {
                t.set_u8(bytes + u32::from(i), 0x31 + 7 * i + k as u8);
            }
            junk_ops(&mut t, f, &[]);
            magic_h6::scrub(&mut t);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(r28::spin_step_0563(g, img, fr.at, fr.esp))
            });
        }
        // the original side: 4 x 18 x 4 calls of slot 314, 4 of 388
        assert_eq!(t.u32(magic_owna::row(314)), 4 * 72, "slot 314's calls");
        assert_eq!(t.u32(magic_owna::row(388)), 4, "slot 388's calls");
    }
}

/// The slots [`mats_0511`] calls: 262 a point, 333 and the scale/factor
/// slots a float (333 the case's yaw from `yaw`), 210/211/235 matrices,
/// the turns 328-330 change one word each, the rest log their words.
fn mats_slots(t: &mut Twin, k: u32, yaw: u32) {
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32 - k as f32);
        }
        0x262
    });
    for s in [210u32, 211] {
        rec(t, s, 2, move |m, a| {
            for i in 0..16 {
                let v = (a[0] & 0xff) as f32 * 0.01 + i as f32 * 0.1 + s as f32 * 0.001;
                wf(m, a[1] + 4 * i, v);
            }
            s ^ a[0]
        });
    }
    filler(t, 235, 1, 16);
    for s in [328u32, 329, 330] {
        rec(t, s, 3, move |m, a| {
            let k = (m.r32(magic_owna::row(s)) + s) % 16;
            let v = rf(m, a[1] + 4 * k) + f32::from_bits(a[2]);
            wf(m, a[0] + 4 * k, v);
            s
        });
    }
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(rf(m, yaw)));
    for s in [277u32, 15, 16, 17] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            0.5 + f64::from(a[0] & 7) * 0.25 + f64::from(s) * 0.01
        });
    }
    for s in [238u32, 296, 257, 217, 255] {
        rec(t, s, 2, move |_, a| (0x5a5a_0000 + s) ^ a[0] ^ a[1]);
    }
}

/// `magic_0511` 0x10001610 and `magic_0602` 0x10006d00
/// (`r28::mats_0511`, internal): the yaw wrapped down, kept and up;
/// slot 296 or 257 for each of the three actors by the flag and the
/// indices (a negative global index, selector bytes above 31).
#[test]
fn mats_0511() {
    for (n, addr) in [(511u16, 0x1610u32), (602, 0x6d00)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let yaw = SCRATCH + 0x90;
        let e = SCRATCH + 0x8000;
        let sel = SCRATCH + 0x5000;
        // (yaw, flag, global index, selector bytes)
        let cases: [(f32, u32, u32, [u8; 3]); 6] = [
            (-1.0, 0, 3, [2, 1, 0x21]),
            (6.0, 1, 3, [5, 0x20, 4]),
            (9.0, 0, 0x40, [7, 0x1f, 0x3f]),
            (1.0, 0, 0x20, [3, 0x20, 0x20]),
            (2.0, 0, 0xffff_fff0, [1, 6, 3]),
            (f32::NAN, 0, 0x1f, [0, 0, 0]),
        ];
        for (k, (y, flag, idx, b)) in (0u32..).zip(cases) {
            mats_slots(&mut t, k, yaw);
            t.set_u32(yaw, y.to_bits());
            junk(&mut t, e + 0x1700, 0xc00, k);
            t.set_u32(op(&t, 0x1b), e);
            t.write(sel, &[b[0], b[1], b[2], 3]);
            t.set_u32(op(&t, 0x11), sel);
            for i in 0..0x40 {
                t.set_u32(op(&t, 0x2e) + 4 * i, 0x0c0c_0100 + i);
            }
            t.set_u32(op(&t, 0x167), flag);
            t.set_u32(op(&t, 0x33c), 0x0b0b_0000 + k);
            t.set_u32(op(&t, 0x36c), idx);
            junk_ops(&mut t, f, &[0x9ff, 0xa09, 0xa13, 0xa1d]);
            magic_h6::scrub(&mut t);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(r28::mats_0511(g, img, fr.at, fr.esp))
            });
        }
        assert_eq!(
            t.u32(magic_owna::row(257)) + t.u32(magic_owna::row(296)),
            18
        );
    }
}

/// The 0x74-record pool of `sb_misc::pool_take` at `take`: `count`
/// records from `recs`, the ones in `busy` taken.
fn pool74(t: &mut Twin, take: u32, recs: u32, count: u32, busy: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    t.set_u32(w(t, 0x37), recs);
    for i in 0..count {
        t.write(recs + 0x74 * i, &[0x5a; 0x74]);
        t.set_u32(recs + 0x74 * i + 0x30, 0x300 + i);
        t.set_u32(recs + 0x74 * i + 0x6c, (busy >> i) & 1);
    }
    t.set_u32(w(t, 0x0f), count);
    t.set_u32(w(t, 0x08), 0);
    t.set_u32(w(t, 0x2b), 0);
}

/// `magic_0563` 0x10003be0 and `magic_0612` 0x10008510
/// (`r28::sparks_0563`, cdecl `(node)`): the tick off and on the mask
/// (below and above 90), the pool with room for every spark, a few and
/// none, the floor below and above the sparks, the count reaching 96 (the
/// node given back).
#[test]
fn sparks_0563() {
    for (n, addr) in [(563u16, 0x3be0u32), (612, 0x8510)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
        let floor = SCRATCH + 0x7f0;
        t.stub_st0(12, move |m, _| f64::from(rf(m, floor)));
        filler(&mut t, 230, 2, 16);
        rec(&mut t, 229, 3, |m, a| {
            for i in 0..4 {
                let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
                wf(m, a[0] + 4 * i, v);
            }
            0x229
        });
        for s in [314u32, 404] {
            rec(&mut t, s, 1, move |m, a| {
                let k = m.r32(magic_owna::row(s));
                (s << 16) ^ k ^ a[0]
            });
            t.stub_st0(s, move |m, a| {
                let k = m.r32(magic_owna::row(s));
                f64::from(f32::from_bits(a[0])) * (f64::from(k % 7) * 0.15 + 0.1) - 0.03
            });
        }
        rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
        let take = t.img.rel32_target(f + 0x159);
        let give = t.img.rel32_target(f + 0x33d);
        let s = SCRATCH + 0x1_0000;
        let recs = SCRATCH + 0x4000;
        let node = SCRATCH + 0x3e00;
        let tex = SCRATCH + 0x900;
        // (tick, count, free records (busy mask of 24), floor)
        let cases: [(u16, u16, u32, f32); 7] = [
            (5, 3, 0, 0.0),
            (0xf, 3, 0, -40.0),
            (0xf, 0x59, 0x00f0_f0f0, 40.0),
            (0x1f, 0x5a, 0x00ff_ffff, 0.0),
            (0x20, 0x5a, 0x0012_3456, 10.0),
            (0x3f, 0x5f, 0, 5.0),
            (0x10, 0x60, 0, -5.0),
        ];
        for (k, (tick, count, busy, fl)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x2e00, 0xc00, k);
            for i in 0..0x20 {
                t.set_u32(
                    s + 0x2ec0 + 0x60 * (i % 18) + 4 * (i / 18),
                    (0.5 * i as f32 - 3.0 + k as f32).to_bits(),
                );
            }
            t.set_u32(op(&t, 0x47), s);
            t.set_u32(op(&t, 0x3d), tex);
            t.set_u32(tex + 0x24, 0x7e00_0000 + k);
            t.set_u32(op(&t, 0x56), 0x00cb_0000 + k);
            t.set_u32(floor, fl.to_bits());
            pool74(&mut t, take, recs, 24, busy);
            magic_hi::lone_node(&mut t, give, node);
            t.set_u16(node + 0x14, tick);
            t.set_u16(node + 0x16, count);
            magic_h6::scrub(&mut t);
            t.check_entry(f, Call::default(), &[node], Eax::Checked);
        }
        assert!(t.u32(magic_owna::row(404)) >= 18, "the sparks");
    }
}

/// A slot that writes `n` words at argument `out` from its other pointer
/// arguments' words (each call other values) and logs its first `words`.
fn mixer(t: &mut Twin, slot: u32, words: usize, out: usize, n: u32) {
    rec(t, slot, words, move |m, a| {
        let k = m.r32(magic_owna::row(slot));
        let mut acc = [0u32; 16];
        for (j, &p) in a[..words].iter().enumerate() {
            let mem = (SCRATCH..SCRATCH + 0x4_0000).contains(&p)
                || (magic::STACK_TOP - 0x2000..magic::STACK_TOP).contains(&p);
            if j != out && mem {
                for i in 0..n as usize {
                    acc[i] = acc[i].rotate_left(3) ^ m.r32(p + 4 * i as u32);
                }
            }
        }
        for i in 0..n {
            let v = 0.5 + (acc[i as usize] & 0xff) as f32 * 0.01 + (slot + k + i) as f32 * 0.125;
            wf(m, a[out] + 4 * i, v);
        }
        slot ^ k
    });
}

/// `magic_0434` 0x10003fc0 and `magic_0610` 0x10009e60
/// (`r28::actor_draw_0434`, cdecl `(obj)`): the actor drawn by name
/// (named parts off and on) and by parts.
#[test]
fn actor_draw_0434() {
    for (n, addr) in [(434u16, 0x3fc0u32), (610, 0x9e60)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(addr);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        filler(&mut t, 235, 1, 16);
        for s in [330u32, 328, 329] {
            rec(&mut t, s, 3, move |m, a| {
                let k = (m.r32(magic_owna::row(s)) + s) % 16;
                let v = rf(m, a[1] + 4 * k) + f32::from_bits(a[2]);
                wf(m, a[0] + 4 * k, v);
                s
            });
        }
        mixer(&mut t, 231, 3, 0, 16);
        mixer(&mut t, 230, 2, 0, 16);
        mixer(&mut t, 229, 3, 0, 4);
        mixer(&mut t, 232, 2, 0, 3);
        rec(&mut t, 325, 2, |m, a| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x325
        });
        rec(&mut t, 930, 2, |m, a| {
            let row = SCRATCH + 0x2_0000 + 0x40 * 930;
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(row + 4 * i, v);
            }
            0x930
        });
        for (s, k) in [
            (833u32, 3usize),
            (847, 2),
            (843, 2),
            (842, 3),
            (929, 0),
            (913, 3),
            (278, 3),
        ] {
            rec(&mut t, s, k, move |_, a| {
                (0x5a5a_0000 + s) ^ a[..k].first().copied().unwrap_or(0)
            });
        }
        t.stub(941, |_, a| SCRATCH + 0x3c00 + (a[0] & 0xf0));
        let named = t.img.rel32_target(f + 0x32e);
        t.set_u16(t.img.word(named + 0x47), 1);
        let nrec = t.img.word(named + 0x53);
        t.set_u32(nrec - 6, 0x0a0a_0006);
        t.set_u16(nrec, 2);
        let map = SCRATCH + 0x700;
        t.write(map, &[0, 2, 1, 3]);
        t.set_u32(op(&t, 0x42), map);
        let index = t.img.rel32_target(f + 0x1b);
        t.set_u32(t.img.word(index + 0x15), map);
        t.set_u32(op(&t, 0x21), 4);
        let actors = op(&t, 0x4d);
        let s = SCRATCH + 0x8000;
        t.set_u32(op(&t, 0xca), s);
        for (k, model) in [(1u32, 0x10fcu16), (2, 0x2000), (3, 0x1100)] {
            let actor = SCRATCH + 0x1000 + 0x1000 * k;
            junk(&mut t, actor, 0x900, k);
            t.set_u16(actor, model);
            let d = actor + 0x800;
            t.set_u32(actor + 0x1c0, d);
            t.set_u16(d + 6, 0);
            t.set_u32(actors + 4 * k, actor);
        }
        // (the object, named parts on; an object no actor holds makes the
        // original read the model at 0: not run)
        for (k, (obj, on)) in [(1u32, 0), (2, 0), (3, 1), (1, 1)].into_iter().enumerate() {
            let k = k as u32;
            junk(&mut t, s, 0x1000, k + 7);
            for i in 0..4 {
                t.set_u32(
                    s + 0xc08 + 0xc8 * i + 0xb4,
                    (i as i32 * 37 - 50 + k as i32).cast_unsigned(),
                );
            }
            let (ga, gb) = (op(&t, 0x355), op(&t, 0x373));
            junk(&mut t, ga, 0x40, k + 4);
            junk(&mut t, gb, 0x40, k + 5);
            for j in 1..4u32 {
                t.set_u32(SCRATCH + 0x1000 + 0x1000 * j + 0x830, 0x0c0c_0000 + j);
            }
            let objw = 0x0b0b_0000 + obj;
            if obj < 4 {
                let actor = SCRATCH + 0x1000 + 0x1000 * u32::from([0u8, 2, 1, 3][obj as usize]);
                t.set_u32(actor + 0x830, objw);
            }
            t.set_u32(t.img.word(named + 0x15), on);
            // the vector's never-written words (junk reads): 0 on both sides
            magic_h6::scrub(&mut t);
            for o in [0x88u32, 0x80, 0x7c] {
                t.set_u32(magic::STACK_TOP - 0x100 - o, 0);
            }
            t.check_entry(f, Call::default(), &[objw], Eax::Checked);
        }
        assert!(t.u32(magic_owna::row(847)) >= 4, "the draws");
    }
}

/// Vector stubs with real maths for the segment tests: 325 copies, 234
/// subtracts, 266 crosses, 233 scales, 394 dots (ST0), 229 multiplies by
/// a row-major matrix, 230 and 327 copy matrices.
fn vector_slots(t: &mut Twin) {
    let v = |m: &dyn StubMem, at: u32| [0, 4, 8, 12].map(|o| rf(m, at + o));
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(t, 234, 3, move |m, a| {
        let (p, q) = (v(m, a[1]), v(m, a[2]));
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, p[i] - q[i]);
        }
        0x234
    });
    rec(t, 266, 3, move |m, a| {
        let (p, q) = (v(m, a[1]), v(m, a[2]));
        let c = [
            p[1] * q[2] - p[2] * q[1],
            p[2] * q[0] - p[0] * q[2],
            p[0] * q[1] - p[1] * q[0],
        ];
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, c[i]);
        }
        0x266
    });
    rec(t, 233, 3, move |m, a| {
        let p = v(m, a[1]);
        let s = f32::from_bits(a[2]);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, p[i] * s);
        }
        0x233
    });
    rec(t, 394, 2, |_, a| 0x0394_0000 ^ a[0]);
    t.stub_st0(394, move |m, a| {
        let (p, q) = (v(m, a[0]), v(m, a[1]));
        f64::from(p[0] * q[0] + p[1] * q[1] + p[2] * q[2])
    });
    rec(t, 229, 3, move |m, a| {
        magic_owna::dump(m, 229, a[2], 4);
        let p = v(m, a[2]);
        for i in 0..4u32 {
            let mut s = 0.0f32;
            for j in 0..4u32 {
                s += rf(m, a[1] + 16 * j + 4 * i) * p[j as usize];
            }
            wf(m, a[0] + 4 * i, s);
        }
        0x229
    });
    for s in [230u32, 327] {
        rec(t, s, 2, move |m, a| {
            for i in 0..16 {
                let x = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, x);
            }
            s
        });
    }
}

/// A part's model at `d`: the groups (`(count, triangles)` each, kind
/// 0x10; the last ends with `end` as the next kind byte) and the vertices.
fn part_model(t: &mut Twin, d: u32, groups: &[&[[u16; 3]]], end: u8, verts: &[[f32; 3]]) {
    junk(t, d, 0x600, 9);
    t.set_u32(d + 4, 0x40);
    t.set_u32(d + 8, 0x400);
    let mut at = d + 0x40;
    for grp in groups {
        t.set_u8(at + 1, 0x10);
        t.set_u16(at + 2, grp.len() as u16);
        at += 0x20;
        for tri in *grp {
            for (i, &x) in tri.iter().enumerate() {
                t.set_u16(at + 2 * i as u32, x);
            }
            at += 0x1c;
        }
    }
    t.set_u8(at + 1, end);
    for (i, p) in verts.iter().enumerate() {
        for (j, &x) in p.iter().enumerate() {
            t.set_u32(d + 0x400 + 16 * i as u32 + 4 * j as u32, x.to_bits());
        }
        t.set_u32(d + 0x400 + 16 * i as u32 + 12, 1.0f32.to_bits());
    }
}

/// `magic_0150` 0x10007690 and `magic_0603` (`r28::ray_hit_0150`,
/// internal `(ecx list, edx P0; P1, out, far)`): one and two parts,
/// groups ending the part (0xff) and all (another kind), an empty group,
/// triangles hit, missed and behind, both best-`t` directions.
#[test]
fn ray_hit_0150() {
    for n in [150u16, 603] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(0x7690);
        vector_slots(&mut t);
        let r = SCRATCH + 0x7000;
        rec(&mut t, 264, 1, move |_, _| r);
        rec(&mut t, 265, 1, |_, a| 0x0265_0000 ^ a[0]);
        // 941: models (ids below 0x100) and matrices (an identity with a
        // shift in its scale row)
        rec(&mut t, 941, 1, |m, a| {
            if a[0] & 0x100 == 0 {
                SCRATCH + 0x8000 + 0x800 * (a[0] & 3)
            } else {
                let at = SCRATCH + 0x9800 + 0x40 * (a[0] & 3);
                for i in 0..16u32 {
                    let x = if i % 5 == 0 { 1.0f32 } else { 0.0 };
                    wf(m, at + 4 * i, x);
                }
                wf(m, at + 0x34, (a[0] & 3) as f32 * 0.5);
                at
            }
        });
        let verts = [
            [-1.0f32, 0.0, -1.0],
            [1.0, 0.0, -1.0],
            [0.0, 0.0, 1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [0.0, 1.0, 1.0],
            [9.0, 0.0, 9.0],
            [11.0, 0.0, 9.0],
            [10.0, 0.0, 11.0],
            [-1.0, -2.0, -1.0],
            [1.0, -2.0, -1.0],
            [0.0, -2.0, 1.0],
        ];
        part_model(
            &mut t,
            SCRATCH + 0x8000,
            &[&[[0, 1, 2], [6, 7, 8]], &[], &[[3, 4, 5]]],
            0xff,
            &verts,
        );
        part_model(
            &mut t,
            SCRATCH + 0x8800,
            &[&[[9, 10, 11], [2, 1, 0]]],
            0x11,
            &verts,
        );
        part_model(
            &mut t,
            SCRATCH + 0x9000,
            &[&[[3, 4, 5]], &[[0, 1, 2]]],
            0xff,
            &verts,
        );
        let list = SCRATCH + 0x600;
        let (p0, p1, out) = (SCRATCH + 0x700, SCRATCH + 0x710, SCRATCH + 0x720);
        // (parts, models of the entries, P0.y, P1.y, far)
        let cases: [(u8, [u32; 3], f32, f32, u32); 6] = [
            (1, [0, 1, 2], 5.0, -5.0, 1),
            (1, [0, 1, 2], 5.0, -5.0, 0),
            (2, [2, 0, 1], -4.0, 6.0, 1),
            (3, [0, 2, 1], 3.0, -3.0, 0),
            (3, [1, 0, 2], 0.5, -0.5, 1),
            (0, [0, 1, 2], 5.0, -5.0, 1),
        ];
        for (k, (parts, models, y0, y1, far)) in (0u32..).zip(cases) {
            junk(&mut t, r, 0xe0, k);
            junk(&mut t, list, 0x40, k + 1);
            t.set_u8(list + 4, parts);
            for (i, &md) in models.iter().enumerate() {
                t.set_u32(list + 0x10 + 16 * i as u32, md);
                t.set_u32(list + 0x14 + 16 * i as u32, 0x100 + md);
            }
            for (at, y) in [(p0, y0), (p1, y1)] {
                for (j, x) in [0.25f32, y, 0.125, 7.0].into_iter().enumerate() {
                    t.set_u32(at + 4 * j as u32, x.to_bits());
                }
            }
            junk(&mut t, out, 0x10, k + 2);
            magic_h6::scrub(&mut t);
            let c = Call {
                ecx: list,
                edx: p0,
                ..Call::default()
            };
            t.check(f, c, &[p1, out, far], Eax::Checked, |g, img, fr| {
                r28::ray_hit_0150(g, img, fr)
            });
        }
        assert!(t.u32(magic_owna::row(229)) > 12, "hits");
    }
}

/// `magic_0150` 0x100070e0 and its thunk 0x10007bf0, `magic_0603`'s copies
/// (`r28::strip_task_0150`, `ecx` = the handle; the thunk `(handle)`): the
/// pass seen (junk `eax`) and new (two children visited), `*G_C` set at
/// either test, no points to make, the hit kinds (near, far; hit and
/// missed), another kind, the ten-point cap and the spacing test, the
/// strip's two pieces with alphas above the top, faded and clamped.
#[test]
fn strip_task_0150() {
    for n in [150u16, 603] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(0x70e0);
        let thunk = t.at(0x7bf0);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let k = |t: &Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        vector_slots(&mut t);
        // 248 adds, 232 halves (the direction) and sets `*G_C` from a cell
        // (the second test), 15 and 277 make the scale 1
        rec(&mut t, 248, 3, |m, a| {
            for i in 0..3 {
                let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
                wf(m, a[0] + 4 * i, v);
            }
            0x248
        });
        let (gc, gc_later) = (op(&t, 0x9c), SCRATCH + 0x7f8);
        rec(&mut t, 232, 2, move |m, a| {
            for i in 0..3 {
                let v = rf(m, a[1] + 4 * i) * 0.5;
                wf(m, a[0] + 4 * i, v);
            }
            let c = m.r32(gc_later);
            m.w32(gc, c);
            0x232
        });
        let (k1, k2) = (k(&t, 0x182), k(&t, 0x1b6));
        rec(&mut t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
        t.stub_st0(277, move |_, _| f64::from(k1) * 2.0);
        rec(&mut t, 15, 1, |_, a| 0x0015_0000 ^ a[0]);
        t.stub_st0(15, move |_, _| f64::from(1.0 / k2));
        rec(&mut t, 210, 2, |m, a| {
            for i in 0..16u32 {
                wf(m, a[1] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
            }
            0x210
        });
        let r = SCRATCH + 0x7000;
        rec(&mut t, 264, 1, move |_, _| r);
        rec(&mut t, 265, 1, |_, a| 0x0265_0000 ^ a[0]);
        rec(&mut t, 941, 1, |m, a| {
            if a[0] & 0x100 == 0 {
                SCRATCH + 0x8000
            } else {
                let at = SCRATCH + 0x9800;
                for i in 0..16u32 {
                    wf(m, at + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
                }
                at
            }
        });
        let verts = [[-1.0f32, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 0.0, 1.0]];
        part_model(&mut t, SCRATCH + 0x8000, &[&[[0, 1, 2]]], 0xff, &verts);
        let list = SCRATCH + 0x600;
        t.write(list, &[0; 0x20]);
        t.set_u8(list + 4, 1);
        t.set_u32(list + 0x14, 0x100);
        // the children's callbacks: slot 1001
        t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
        rec(&mut t, 1001, 1, |m, a| {
            let k = m.r32(magic_owna::row(1001));
            0x1001_0000 ^ k ^ m.r32(a[0])
        });
        let (h, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
        let (b0, b1, kids) = (SCRATCH + 0x1400, SCRATCH + 0x1600, SCRATCH + 0x1800);
        let (actor, data) = (SCRATCH + 0x1a00, SCRATCH + 0x3000);
        let ky = k(&t, 0xfe);
        // (pass seen, *G_C at the 1st / 2nd test, points to make, kind,
        // the second bone's point, count, f8 junk, e4)
        let cases: [(bool, (u32, u32), u16, u32, [f32; 3], u16, f32); 11] = [
            (true, (0, 0), 1, 0x1071, [0.1, 1.0, 0.1], 0, 100.0),
            (false, (1, 0), 1, 0x1071, [0.1, 1.0, 0.1], 0, 100.0),
            (false, (0, 0), 0, 0x1071, [0.1, 1.0, 0.1], 0, 100.0),
            (false, (0, 0), 1, 0x1071, [0.1, 1.0, 0.1], 0, 100.0),
            (false, (0, 0), 1, 0x1089, [0.1, 1.0, 0.1], 3, 100.0),
            (false, (0, 0), 1, 0x108c, [50.0, 1.0, 50.0], 2, 100.0),
            (false, (0, 0), 1, 0x1088, [50.0, 1.0, 50.0], 9, 100.0),
            (false, (0, 0), 1, 0x2000, [0.1, 1.0, 0.1], 4, 0.01),
            (false, (0, 1), 1, 0x1072, [0.1, 1.0, 0.1], 10, 100.0),
            (false, (0, 0), 2, 0x1000, [0.1, 1.0, 0.1], 1, 100.0),
            (false, (0, 0), 1, 0x108d, [0.1, 1.0, 0.1], 5, 1.0e9),
        ];
        let mut hits = 0;
        for (kk, (seen, (c1, c2), make, kind, p, count, e4)) in (0u32..).zip(cases) {
            junk(&mut t, node, 0x200, kk);
            t.set_u32(h, node);
            t.set_u8(h + 4, 0x7e);
            t.set_u8(h + 5, 3 + kk as u8);
            t.set_u8(node + 0x1a, if seen { 3 + kk as u8 } else { 1 });
            // two children (vtables at `kids + 0x40`), then the node
            for (i, c) in [kids, kids + 0x20].into_iter().enumerate() {
                junk(&mut t, c, 0x20, kk + i as u32);
                t.set_u32(c, kids + 0x40);
                t.set_u32(c + 8, if i == 0 { kids + 0x20 } else { node });
            }
            t.set_u32(kids + 0x40 + 0x1c, magic::TRAPS + 16 * 1001);
            t.set_u32(node + 0x10, kids);
            // the bones: their walks see their pass (nothing moves); B0's
            // 0x9c an identity matrix, B1's 0xcc the point
            for b in [b0, b1] {
                junk(&mut t, b, 0x100, kk + 5);
                t.set_u8(b + 0x1a, 3 + kk as u8);
            }
            for i in 0..16u32 {
                t.set_u32(
                    b0 + 0x9c + 4 * i,
                    if i % 5 == 0 { 1.0f32 } else { 0.0 }.to_bits(),
                );
            }
            for (i, &x) in p.iter().enumerate() {
                t.set_u32(b1 + 0xcc + 4 * i as u32, x.to_bits());
            }
            t.set_u32(b1 + 0xd8, 1.0f32.to_bits());
            t.set_u32(node + 0xdc, b0);
            t.set_u32(node + 0xe0, b1);
            // (`+0x9c` and `+0x4db` name the same global: the second test
            // sees what slot 232 stored)
            t.set_u32(op(&t, 0x9c), c1);
            t.set_u32(gc_later, c2);
            t.set_u16(node + 0xee, make);
            t.set_u32(node + 0xf4, kind);
            t.set_u32(node + 0xf0, actor);
            t.set_u32(actor + 0x54, 0x00a0_0000 + kk);
            t.set_u32(node + 0xfc, list);
            t.set_u16(node + 0xec, count);
            t.set_u32(node + 0xe4, (e4 / ky).to_bits());
            t.set_u32(node + 0xe8, (1.5 + kk as f32).to_bits());
            t.set_u32(op(&t, 0x3e2), (2.5 - kk as f32).to_bits());
            // the ring and the strip (`N` is both): 8 slots of 16 bytes
            t.set_u32(node + 0x1c, data);
            t.set_u16(node + 0x20, 8);
            t.set_u16(node + 0x22, 0x10);
            t.set_u16(node + 0x24, (kk % 8) as u16);
            t.set_u16(node + 0x26, if kk % 2 == 0 { 5 } else { 8 });
            t.set_u16(node + 0x28, (kk * 3 % 8) as u16);
            for i in 0..8u32 {
                junk(&mut t, data + 0x10 * i, 0x10, kk + i);
                let a = [0.5f32, 0.01, 2.0, 0.3, -0.5, 0.05, 1.0, 0.9][((i + kk) % 8) as usize];
                t.set_u32(data + 0x10 * i + 0xc, a.to_bits());
            }
            magic_h6::scrub(&mut t);
            // the pass seen, or `*G_C` at once: a bone's `eax` (its cookie)
            let eax = if seen || c1 != 0 {
                Eax::Junk
            } else {
                Eax::Checked
            };
            let at = if kk % 2 == 0 { f } else { thunk };
            if at == f {
                t.check(f, Call::this(h), &[], eax, |g, img, fr| {
                    r28::strip_task_0150(g, img, fr)
                });
            } else {
                t.check_entry(thunk, Call::default(), &[h], eax);
            }
            let hit_kind = matches!(kind, 0x1071 | 0x1088 | 0x1089 | 0x108c);
            hits +=
                usize::from(t.u32(node + 0xf8) == 1 && hit_kind && make != 0 && c1 == 0 && !seen);
        }
        assert!(hits >= 2, "hits: {hits}");
        assert!(t.u32(magic_owna::row(1001)) >= 8, "the children");
    }
}

/// `magic_0150` 0x10006210 and `magic_0603` (`r28::strip_mesh_0150`,
/// `ecx` = the request): cursors at several alignments, colours in range,
/// above it (wrapping bytes) and NaN, widths of either sign.
#[test]
fn strip_mesh_0150() {
    for n in [150u16, 603] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(0x6210);
        vector_slots(&mut t);
        rec(&mut t, 248, 3, |m, a| {
            for i in 0..3 {
                let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
                wf(m, a[0] + 4 * i, v);
            }
            0x248
        });
        rec(&mut t, 232, 2, |m, a| {
            for i in 0..3 {
                let v = rf(m, a[1] + 4 * i) * 0.5;
                wf(m, a[0] + 4 * i, v);
            }
            0x232
        });
        let c = SCRATCH + 0x100;
        let (mtx, cols, buf) = (SCRATCH + 0x200, SCRATCH + 0x300, SCRATCH + 0x1000);
        for (k, (off, wd, col)) in [
            (0u32, 1.5f32, 0.5f32),
            (3, -2.0, 0.99),
            (0x11, 0.25, 1.5),
            (0x2c, 4.0, f32::NAN),
            (7, 0.0, -0.25),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            junk(&mut t, buf, 0x300, k);
            junk(&mut t, c, 0x40, k + 1);
            t.set_u32(c, buf + off);
            for i in 0..16u32 {
                let x = if i % 5 == 0 { 1.0f32 } else { 0.125 * i as f32 };
                t.set_u32(mtx + 4 * i, x.to_bits());
            }
            t.set_u32(c + 0xc, mtx);
            for (j, o) in [0x14u32, 0x18, 0x1c].into_iter().enumerate() {
                let at = cols + 0x10 * j as u32;
                t.set_u32(c + o, at);
                for i in 0..4u32 {
                    let x = if i == 1 {
                        col
                    } else {
                        0.2 * (i + j as u32 + k) as f32
                    };
                    t.set_u32(at + 4 * i, x.to_bits());
                }
            }
            t.set_u32(c + 0x20, wd.to_bits());
            for (i, x) in [1.0f32, 2.0, -3.0, 4.5, -1.0, 0.5].into_iter().enumerate() {
                t.set_u32(c + 0x24 + 4 * i as u32, (x + k as f32).to_bits());
            }
            magic_h6::scrub(&mut t);
            t.check(f, Call::this(c), &[], Eax::Checked, r28::strip_mesh_0150);
        }
    }
}

/// `magic_0150` 0x100069a0 and `magic_0603` (`r28::strip_draw_0150`, cdecl
/// `(obj)`): the size at 0, negative and NaN (the short way), positive
/// with slot 773 true and false; the flag word restored each time.
#[test]
fn strip_draw_0150() {
    for n in [150u16, 603] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(0x69a0);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        vector_slots(&mut t);
        rec(&mut t, 248, 3, |m, a| {
            for i in 0..3 {
                let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
                wf(m, a[0] + 4 * i, v);
            }
            0x248
        });
        rec(&mut t, 232, 2, |m, a| {
            for i in 0..3 {
                let v = rf(m, a[1] + 4 * i) * 0.5;
                wf(m, a[0] + 4 * i, v);
            }
            0x232
        });
        mixer(&mut t, 231, 3, 0, 16);
        filler(&mut t, 235, 1, 16);
        let cell = SCRATCH + 0x7f0;
        // each game call logs the flag word it sees
        for (s, k) in [(311u32, 1usize), (779, 2), (865, 3), (776, 1), (777, 1)] {
            rec(&mut t, s, k, move |m, a| {
                (0x5a5a_0000 + s) ^ a[0] ^ m.r32(cell).rotate_left(8)
            });
        }
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
        t.stub_edx(772, |_, a| 0x0077_2000 ^ a[1]);
        let answer = SCRATCH + 0x7f4;
        rec(&mut t, 773, 2, move |m, _| m.r32(answer));
        // the packet calls dump the request and the record
        for (s, k) in [(819u32, 4usize), (820, 5)] {
            rec(&mut t, s, k, move |m, a| {
                magic_owna::dump(m, s, a[2], 16);
                magic_owna::dump(m, s + 0x100, a[3], 16);
                (s << 16) ^ m.r32(cell)
            });
        }
        rec(&mut t, 307, 0, |_, _| 0x307);
        t.stub_st0(307, |m, _| f64::from(m.r32(SCRATCH + 0x7f8) & 0xff) * 0.01);
        let (obj, strip, data) = (SCRATCH + 0x2000, SCRATCH + 0x2c00, SCRATCH + 0x3000);
        let (b0, b1) = (SCRATCH + 0x2400, SCRATCH + 0x2600);
        let (gx, gy) = (SCRATCH + 0x7e0, op(&t, 0x331));
        for (k, (s, yes)) in [
            (0.0f32, 0u32),
            (-1.0, 0),
            (f32::NAN, 1),
            (0.75, 1),
            (1.5, 0),
            (0.25, 7),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            junk(&mut t, obj + 0x900, 0x40, k);
            t.set_u32(obj + 0x930, s.to_bits());
            t.set_u32(obj + 0x920, strip);
            t.set_u32(obj + 0x924, b1);
            t.set_u32(obj + 0x91c, b0);
            junk(&mut t, b0, 0x100, k + 1);
            junk(&mut t, b1, 0x100, k + 2);
            for (i, x) in [0.5f32, -1.0, 2.0, 1.0].into_iter().enumerate() {
                t.set_u32(b1 + 0xcc + 4 * i as u32, (x + k as f32).to_bits());
            }
            // the strip: 8 points of 16 bytes, a window of 5 from `k`
            junk(&mut t, strip, 0x40, k + 3);
            t.set_u32(strip + 0x1c, data);
            t.set_u16(strip + 0x20, 8);
            t.set_u16(strip + 0x22, 0x10);
            t.set_u16(strip + 0x26, 5);
            t.set_u16(strip + 0x28, (k * 3 % 8) as u16);
            for i in 0..32u32 {
                t.set_u32(data + 4 * i, (0.25 * i as f32 - 3.0).to_bits());
            }
            let p = op(&t, 0x56);
            t.set_u32(p, cell);
            t.set_u32(cell, 0x0c0c_0000 + k);
            t.set_u32(answer, yes);
            t.set_u32(SCRATCH + 0x7f8, 0x40 + 7 * k);
            t.set_u32(op(&t, 0x30c), gx);
            t.set_u32(gx, 0x0e0e_0000 + k);
            t.set_u32(gy, 0x0f0f_0000 + k);
            magic_h6::scrub(&mut t);
            // the request's and the record's never-written words: 0
            let z = ((magic::STACK_TOP - 0x100 - 4) & !0xf) - 0x1f0;
            for o in [0x30u32, 0x5c, 0x74] {
                t.set_u32(z + o, 0);
            }
            t.set_u16(z + 0x6a, 0);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
        assert!(
            t.u32(magic_owna::row(820)) >= 1 && t.u32(magic_owna::row(819)) >= 1,
            "both packets"
        );
    }
}

/// `magic_0150` 0x10004c90 and `magic_0603` (`r28::ribbon_draw_0150`,
/// `ecx` = the object, `edx` = the strip): windows in one piece and
/// wrapped into two (the second empty or not), points whose `w` passes the
/// record limit or not, the `+0x944` matrix or none, slot 773 true and
/// false, and a buffer whose records stay empty.
#[test]
fn ribbon_draw_0150() {
    for n in [150u16, 603] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(0x4c90);
        let k = |t: &Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        rec(&mut t, 325, 2, |m, a| {
            for i in 0..4 {
                let x = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, x);
            }
            0x325
        });
        mixer(&mut t, 231, 3, 0, 16);
        mixer(&mut t, 230, 2, 0, 16);
        let cell = SCRATCH + 0x7f0;
        for (s, k) in [(865u32, 3usize), (776, 1), (777, 1)] {
            rec(&mut t, s, k, move |m, a| {
                (0x5a5a_0000 + s) ^ a[0] ^ m.r32(cell).rotate_left(8)
            });
        }
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
        t.stub_edx(772, |_, a| 0x0077_2000 ^ a[1]);
        let answer = SCRATCH + 0x7f4;
        rec(&mut t, 773, 2, move |m, _| m.r32(answer));
        for (s, k) in [(819u32, 4usize), (820, 5)] {
            rec(&mut t, s, k, move |m, a| {
                magic_owna::dump(m, s, a[2], 16);
                magic_owna::dump(m, s + 0x100, a[3], 16);
                (s << 16) ^ m.r32(cell)
            });
        }
        let top = k(&t, 0x531);
        let (obj, strip, data) = (SCRATCH + 0x2000, SCRATCH + 0x2c00, SCRATCH + 0x3000);
        let (b0, b1) = (SCRATCH + 0x2400, SCRATCH + 0x2600);
        // (window, start, matrix flag, 773, the `w`s pass the limit (bits))
        let cases: [(u16, u16, u32, u32, u32); 6] = [
            (5, 0, 0, 1, 0),
            (5, 6, 1, 0, 0b0101_0101),
            (8, 3, 1, 1, 0b1000_0001),
            (6, 2, 0, 0, 0),
            (3, 6, 0, 1, 0xff),
            (4, 5, 1, 0, 0b10),
        ];
        for (kk, (window, start, mflag, yes, high)) in (0u32..).zip(cases) {
            junk(&mut t, obj + 0x900, 0x60, kk);
            t.set_u32(obj + 0x920, strip);
            t.set_u32(obj + 0x924, b1);
            t.set_u32(obj + 0x91c, b0);
            t.set_u32(obj + 0x934, (0.5 + 0.25 * kk as f32).to_bits());
            t.set_u32(obj + 0x944, mflag);
            junk(&mut t, b0, 0x100, kk + 1);
            junk(&mut t, b1, 0x100, kk + 2);
            junk(&mut t, strip, 0x40, kk + 3);
            t.set_u32(strip + 0x1c, data);
            t.set_u16(strip + 0x20, 8);
            t.set_u16(strip + 0x22, 0x10);
            t.set_u16(strip + 0x26, window);
            t.set_u16(strip + 0x28, start);
            for i in 0..8u32 {
                let p = data + 0x10 * i;
                let x = (i * i) as f32 * 0.5 - kk as f32;
                for (j, v) in [x, 1.0 + 0.25 * i as f32, 2.0 - 0.75 * i as f32]
                    .into_iter()
                    .enumerate()
                {
                    t.set_u32(p + 4 * j as u32, v.to_bits());
                }
                let w = if (high >> i) & 1 != 0 {
                    top + 1.0
                } else {
                    top - 0.5 * i as f32
                };
                t.set_u32(p + 0xc, w.to_bits());
            }
            let pf = op(&t, 0xad9);
            t.set_u32(pf, cell);
            t.set_u32(cell, 0x0c0c_0000 + kk);
            t.set_u32(answer, yes);
            let gx = SCRATCH + 0x7e0;
            t.set_u32(op(&t, 0x9b6), gx);
            t.set_u32(gx, 0x0e0e_0000 + kk);
            let gy = op(&t, 0xaae);
            t.set_u32(gy, 0x0f0f_0000 + kk);
            magic_h6::scrub(&mut t);
            let z = ((magic::STACK_TOP - 0x100 - 4) & !0xf) - 0x240;
            for o in [0x50u32, 0x54, 0x58, 0x74] {
                t.set_u32(z + o, 0);
            }
            t.write(z + 0x69, &[0; 3]);
            let c = Call {
                ecx: obj,
                edx: strip,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, r28::ribbon_draw_0150);
        }
        assert!(
            t.u32(magic_owna::row(820)) >= 1 && t.u32(magic_owna::row(819)) >= 1,
            "both packets"
        );
    }
}
