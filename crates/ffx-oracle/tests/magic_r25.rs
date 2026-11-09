//! Round 25's ports (`ffx_magic::r25`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, r25};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::twin;
use ownb::rec;

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Folds `v` into word `i` of `slot`'s dump row (the outputs a stub writes
/// to the stack, which the comparison skips).
fn fold(m: &mut dyn StubMem, slot: u32, i: u32, v: u32) {
    let at = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400) + 4 * i;
    let x = m.r32(at).rotate_left(5) ^ v;
    m.w32(at, x);
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// Every slot answering `first word ^ (slot << 20)` and logging its first
/// word (into `SCRATCH + 0xa00..`), as `magic_o6_fam.rs`'s `stubs`.
fn all_stubs(t: &mut Twin) {
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

/// The particle node's globals behind `particle_node_with` at `pnw` (as
/// `magic_s3.rs`'s test): the lock, the pool word and the owner block.
fn particle_globals(t: &mut Twin, pnw: u32) {
    let pn = t.img.rel32_target(pnw + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x7f00);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(t.img.word(pnw + 0x0d), owner);
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
}

/// `magic_0573` 0x100077b0 (`r25::summon_task_0573`): every state and
/// count way, the object's flag bit set and not, the sum below 1.0, at
/// it and NaN, the jump table's dead states and the states past it. The
/// bone update stops at once (its own test is `magic_r23.rs`'s).
#[test]
fn summon_task_0573() {
    let Some(mut t) = twin(573) else { return };
    all_stubs(&mut t);
    let f = t.at(0x77b0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 - 1.25);
        }
        0x531
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            let x = m.r32(a[1] + 4 * i);
            fold(m, 234, i, x);
        }
        0x234
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]) + 0.5;
            wf(m, a[0] + 4 * i, v);
        }
        0x233
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x0248_0000 ^ m.r32(a[2])
    });
    for (s, n) in [
        (325u32, 2usize),
        (228, 2),
        (239, 3),
        (213, 2),
        (211, 2),
        (255, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let pnw = t.img.rel32_target(f + 0x31f);
    particle_globals(&mut t, pnw);
    let task = SCRATCH + 0x100;
    let (tn, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let obj = SCRATCH + 0x2000;
    let list = w(&t, 0x26a);
    let plist = t.img.word(pnw + 0x26);
    let old = SCRATCH + 0x800;
    let pass = w(&t, 0xd0);
    // (state, count, the object's flags, R + 0x2c)
    let nan = f32::NAN.to_bits();
    let cases: [(u32, u32, u16, u32); 21] = [
        (0, 9, 0, 0),
        (1, 3, 0, 0),
        (1, 0, 0, 0),
        (1, u32::MAX, 0, 0),
        (5, 2, 0, 0),
        (10, 0, 0, 0),
        (11, 2, 0, 0),
        (11, 0, 0, 0),
        (11, 0, 1, 0),
        (0x15, 3, 0, 0x3f00_0000),
        (0x15, 1, 0, 0x3f4c_cccd),
        (0x15, 0, 1, 0x3f80_0000),
        (0x15, 0, 0, 0x3e00_0000),
        (0x15, 2, 0, nan),
        (0x14, 7, 1, 0),
        (0x1e, 3, 0, 0),
        (0x1e, 0, 1, 0),
        (0x2710, 5, 0, 0),
        (0x13, 1, 0, 0),
        (0x16, 0, 0, 0),
        (0x4e20, 4, 0, 0),
    ];
    for (k, (state, count, flag, sum)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x600, k);
        junk(&mut t, obj, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count);
        t.set_u32(r + 0x2c, sum);
        t.set_u32(r + 0x30, obj);
        t.set_u16(obj + 0x90, flag);
        // the vectors the slots add: moderate floats
        for i in 0..16 {
            t.set_u32(r + 0x3c + 4 * i, (0.25 * i as f32 - 1.0).to_bits());
        }
        t.set_u32(r + 0xa8, (1.5 + k as f32).to_bits());
        // the bone node's walk stops at once
        t.set_u8(pass, 0x30 + k as u8);
        t.set_u8(r + 0x10c + 0x1a, 0x30 + k as u8);
        // the nodes' parents: none
        for n in [0x2bcu32, 0x410, 0x510] {
            t.set_u32(r + n + 0x14, 0);
        }
        // the static lists: empty, or with a last node
        for (l, i) in [(list, 0u32), (plist, 1)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        junk_ops(&mut t, f, &[0x358, 0x34a]);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, prev);
        t.set_u32(tn + 8, next);
        t.write(prev, &[0x4d; 0x20]);
        t.write(next, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x77b0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}

/// `magic_0708` 0x10001f80 (`r25::draw_0708`): off, a negative time,
/// then the times around every bound (the fade from 1, the parts bound
/// up to 0x190, the quad's alpha ramps from 15 to 0xdb with slot 885
/// answering and not, the rise's random start, its later steps and its
/// end, the second rise and the clean-up past 0x172 with the busy word
/// set and not), the stop word set and not. Slot 361 writes the record's
/// `+0x10`/`+0x14` words, so the quad's divisions see non-zero values.
#[test]
fn draw_0708() {
    let Some(mut t) = twin(708) else { return };
    all_stubs(&mut t);
    let f = t.at(0x1f80);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let cell = SCRATCH + 0x240;
    for (s, n) in [
        (723u32, 1usize),
        (724, 2),
        (355, 2),
        (354, 2),
        (264, 1),
        (265, 1),
        (357, 3),
        (356, 3),
        (346, 1),
        (211, 2),
        (238, 2),
        (213, 2),
        (214, 1),
        (352, 3),
        (776, 1),
        (915, 5),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, 0.25 * i as f32 - 1.0);
        }
        0x230
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x0235_0000 ^ (a[0] & 0xfff0) ^ i);
        }
        0x235
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(3);
            m.w32(a[0] + 4 * i, v);
            fold(m, 231, i, v);
        }
        0x231
    });
    let ent = SCRATCH + 0x600;
    rec(&mut t, 941, 1, move |_, a| ent + 0x20 * (a[0] & 1));
    rec(&mut t, 361, 4, |m, a| {
        // the record's +0x10 / +0x14 words the quad divides
        let (x, y) = (m.r32(SCRATCH + 0x250), m.r32(SCRATCH + 0x254));
        m.w32(a[1] + 0x10, a[0].wrapping_mul(0x0001_3579) ^ x);
        m.w32(a[1] + 0x14, a[3].wrapping_mul(0x0002_4680) ^ y);
        // the whole draw record as the game's function sees it
        for i in 0..0x14 {
            let z = m.r32(a[1] + 4 * i);
            fold(m, 361, i % 8, z.rotate_left(i));
        }
        0x361
    });
    let part = SCRATCH + 0x700;
    rec(&mut t, 719, 3, move |_, _| part);
    rec(&mut t, 243, 2, |m, a| {
        for i in 0..8 {
            m.w32(a[0] + 4 * i, a[1] ^ (i << 8));
        }
        0x243
    });
    let lock = SCRATCH + 0x220;
    rec(&mut t, 885, 7, move |m, a| {
        let x = m.r32(a[6]) ^ a[0] ^ m.r32(lock).rotate_left(7);
        fold(m, 885, 0, x);
        m.w32(a[6], 0x0885_0000 ^ a[1]);
        m.r32(cell)
    });
    rec(&mut t, 879, 10, move |m, a| {
        let l = m.r32(lock);
        fold(m, 879, 5, l);
        for i in 0..4 {
            let x = m.r32(a[4] + 4 * i);
            fold(m, 879, i, x);
        }
        let x = m.r32(a[1]) ^ m.r32(a[6]);
        fold(m, 879, 4, x);
        0x879
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        f64::from(m.r32(magic_owna::row(307)) % 7) * 0.13 + 0.02
    });
    rec(&mut t, 712, 1, move |m, _| m.r32(cell + 4));
    let (p1, p2, tex, o, lock, stop, rec_) = (
        SCRATCH + 0x200,
        SCRATCH + 0x210,
        SCRATCH + 0x260,
        SCRATCH + 0x1000,
        SCRATCH + 0x220,
        SCRATCH + 0x230,
        SCRATCH + 0x500,
    );
    let parts = op(&t, 0x594);
    let pb = op(&t, 0x5b5);
    // (time, on, the rise's word *G_R, busy, stop, 885's answer)
    let cases: [(u32, u32, u32, u16, u32, u32); 25] = [
        (5, 0, 0, 0, 0, 1),
        (u32::MAX, 1, 0, 0, 0, 1),
        (0, 1, 0, 0, 0, 1),
        (1, 1, 0, 0, 1, 1),
        (0xe, 1, 0, 0, 0, 1),
        (0xf, 1, 0, 0, 0, 1),
        (0x40, 1, 0, 0, 0, 0),
        (0x68, 1, 0, 0, 0, 1),
        (0x69, 1, 0, 0, 0, 1),
        (0x77, 1, 0, 0, 0, 1),
        (0x78, 1, 0, 0, 0, 1),
        (0xa0, 1, 0, 0, 0, 1),
        (0xbe, 1, 0, 0, 0, 1),
        (0xc0, 1, 3, 0, 0, 1),
        (0xdb, 1, 0, 0, 0, 1),
        (0xdc, 1, 0, 0, 0, 1),
        (0x153, 1, 0, 0, 0, 1),
        (0x154, 1, 0, 0, 0, 1),
        (0x154, 1, 5, 0, 0, 1),
        (0x171, 1, 2, 0, 0, 1),
        (0x172, 1, 0, 0, 0, 1),
        (0x173, 1, 0, 0, 0, 1),
        (0x173, 1, 0, 1, 1, 1),
        (0x190, 1, 0, 0, 0, 1),
        (0x191, 1, 0, 0, 0, 1),
    ];
    for (k, (time, on, r, busy, stopv, shown)) in (0u32..).zip(cases) {
        for i in 0..16u32 {
            let p = parts + 0x80 * i;
            junk(&mut t, p, 0x80, k + i);
            t.set_u16(p + 0x1e, u16::from((i + k) % 3 == 0));
        }
        junk(&mut t, pb, 0x80, k + 20);
        t.set_u16(pb + 0x1e, (k & 1) as u16);
        junk_ops(
            &mut t,
            f,
            &[0x3d, 0x66, 0x92, 0x1da, 0x200, 0x501, 0x534, 0x67d, 0x655],
        );
        t.set_u32(op(&t, 0x1c), on);
        t.set_u32(op(&t, 0x2b), time);
        t.set_u32(op(&t, 0x4e6), r);
        t.set_u16(op(&t, 0x5eb), busy);
        t.set_u32(op(&t, 0x37), p1);
        t.set_u32(op(&t, 0x48), p2);
        junk(&mut t, p1, 0x20, k);
        t.set_u32(op(&t, 0x117), tex);
        t.set_u32(tex, 0x7e00_0000 + k);
        t.set_u32(op(&t, 0x2d5), o);
        junk(&mut t, o + 0x800, 0x40, k + 1);
        t.set_u32(op(&t, 0x32a), lock);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(op(&t, 0x693), stop);
        t.set_u32(stop, stopv);
        t.set_u32(cell, shown);
        t.set_u32(cell + 4, k & 2);
        t.set_u32(SCRATCH + 0x250, 0x0012_3456 * (k + 1));
        t.set_u32(SCRATCH + 0x254, 0x8765_4321_u32.wrapping_mul(k + 3));
        // the part's 64-bit word: the sizes vary
        t.set_u32(part, 0x1234_5678u32.rotate_left(k) ^ (k << 26));
        t.set_u32(part + 4, 0x9abc_def0u32.rotate_left(3 * k));
        // the parts record: two entries
        t.set_u32(op(&t, 0x206), rec_);
        junk(&mut t, rec_, 0x40, k + 2);
        t.set_u16(rec_ + 4, 2);
        for e in [ent, ent + 0x20] {
            junk(&mut t, e, 0x20, k + 3);
        }
        t.set_u32(op(&t, 0x578), 0x30 + k);
        t.set_u32(op(&t, 0x559), 0x31 + k);
        t.set_u32(op(&t, 0x57e), 0x2 + k);
        ownb::dirty_stack(&mut t, 0x1f80 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the draw's calls");
}

/// The 0x74-record pool of `sb_misc::pool_take` at `take`: `count`
/// records from `recs`, the ones in `busy` taken.
fn pool74(t: &mut Twin, take: u32, recs: u32, count: u32, busy: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    t.set_u32(w(t, 0x37), recs);
    for i in 0..8 {
        t.write(recs + 0x74 * i, &[0x5a; 0x74]);
        t.set_u32(recs + 0x74 * i + 0x30, 0x300 + i);
        t.set_u32(recs + 0x74 * i + 0x6c, (busy >> i) & 1);
    }
    t.set_u32(w(t, 0x0f), count);
    t.set_u32(w(t, 0x08), 0);
    t.set_u32(w(t, 0x2b), 0);
}

/// `magic_0612` 0x10008860 and `magic_0563` 0x10003f30
/// (`r25::spin_setup_0612`, one record and two): the angle wrapped up,
/// down and not, the count running out and not, a free record, the pool
/// full after the first take and full at once, a record's point above
/// the ground and below it.
#[test]
fn spin_setup_0612() {
    for (n, addr) in [(612u16, 0x8860u32), (563, 0x3f30)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        let f = t.at(addr);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        rec(&mut t, 237, 4, |m, a| {
            for i in 0..4 {
                let v = 0.5 * a[1] as f32 - 0.75 * i as f32 + (a[0] & 0xf) as f32;
                wf(m, a[3] + 4 * i, v);
            }
            0x237
        });
        rec(&mut t, 235, 1, |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
            }
            0x235
        });
        for s in [330u32, 328, 329, 231] {
            rec(&mut t, s, 3, move |m, a| {
                for i in 0..16 {
                    let v = m.r32(a[1] + 4 * i) ^ (a[2].rotate_left(i) & 0x0000_ff00);
                    m.w32(a[0] + 4 * i, v);
                    fold(m, s, i % 4, v);
                }
                s
            });
        }
        rec(&mut t, 229, 3, |m, a| {
            for i in 0..4 {
                let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
                wf(m, a[0] + 4 * i, v);
            }
            0x229
        });
        rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
        let ground = SCRATCH + 0x7f0;
        t.stub_st0(12, move |m, _| f64::from(rf(m, ground)));
        rec(&mut t, 314, 1, |m, a| {
            let k = m.r32(magic_owna::row(314));
            fold(m, 314, 0, a[0]);
            0x0314_0000 ^ k
        });
        t.stub_st0(314, |m, a| {
            let k = m.r32(magic_owna::row(314));
            f64::from(f32::from_bits(a[0])) * (f64::from(k % 5) * 0.2 + 0.1)
        });
        let take = t.img.rel32_target(f + 0x309);
        let recs = SCRATCH + 0x4000;
        let s = SCRATCH + 0x1_0000;
        let x = SCRATCH + 0x900;
        // (angle, count, free records (busy mask), the ground)
        let cases: [(f32, u32, u32, f32); 6] = [
            (0.5, 3, 0, -10.0),
            (3.2, 1, 0b10, 50.0),
            (-3.3, 2, 0b1, -10.0),
            (0.0, 1, 0xff, 0.0),
            (f32::NAN, 5, 0, 5.0),
            (6.0, 1, 0b100, -100.0),
        ];
        for (k, (angle, count, busy, gr)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0x2b00, 0xc00, k);
            for i in 0..12 {
                t.set_u32(
                    s + 0x2b40 + 4 * i,
                    (2.5 * i as f32 - 7.0 + k as f32).to_bits(),
                );
                t.set_u32(s + 0x3620 + 4 * i, (0.125 * i as f32 + 0.5).to_bits());
            }
            t.set_u32(s + 0x3640, angle.to_bits());
            t.set_u32(s + 0x3660, count);
            t.set_u32(op(&t, 0x16), s);
            t.set_u32(op(&t, 0x30), 0x00cb_0000 + k);
            junk(&mut t, x, 0x40, k + 1);
            t.set_u32(op(&t, 0x266), x);
            t.set_u32(ground, gr.to_bits());
            pool74(&mut t, take, recs, 3, busy);
            ownb::dirty_stack(&mut t, addr + k, &[]);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(r25::spin_setup_0612(g, img, fr.at, fr.esp))
            });
        }
        assert!(t.u32(ownb::LOG) > 0x100, "the set-up's calls");
    }
}

/// `magic_0573` 0x10007ee0 (`r25::main_task_0573`): every state, each
/// count way, the flags the late states test, the static list empty and
/// not, the jump table's dead entries and the states past it. The summon
/// set-up's statics as in `magic_r23.rs`'s test.
#[test]
fn main_task_0573() {
    let Some(mut t) = twin(573) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7ee0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [
        (15u32, 0.5f64),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
        (333, -0.75),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75);
        0x601
    });
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 3.0);
        }
        0x290
    });
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 - 2.0);
        }
        0x210
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            fold(m, 325, i, x);
        }
        0x325
    });
    for (s, n) in [
        (211u32, 2usize),
        (239, 3),
        (713, 1),
        (219, 2),
        (276, 2),
        (548, 1),
        (413, 1),
        (215, 1),
        (14, 2),
        (415, 1),
        (255, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    // the summon set-up's statics (as `magic_r23.rs`'s summon_setup_0573)
    let ss = t.img.rel32_target(f + 0x7b5);
    let ssw = |t: &Twin, o: u32| t.img.word(ss + o);
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    // the particle node's globals
    let pnw = t.img.rel32_target(f + 0x87e);
    particle_globals(&mut t, pnw);
    let plist = t.img.word(pnw + 0x26);
    let list = op(&t, 0x93);
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let src = SCRATCH + 0x3000;
    // (state, count, flags byte R + 0x858, R + 0x1158 / + 0xf4c at 20000)
    let cases: [(u32, u32, u8, bool); 24] = [
        (0, 3, 0, false),
        (10, 0, 0, false),
        (11, 4, 0, false),
        (11, 0, 0, false),
        (11, u32::MAX, 0, false),
        (5, 1, 0, false),
        (0x14, 0, 0, false),
        (0x15, 2, 0, false),
        (0x15, 0, 0, false),
        (0x1e, 0, 0, false),
        (0x1f, 1, 0, false),
        (0x1f, 0, 0, false),
        (0x32, 0, 0, false),
        (0x33, 5, 0, false),
        (0x33, 0, 0, false),
        (0x34, 1, 0, false),
        (0x34, 0, 0, false),
        (0x35, 0, 1, true),
        (0x35, 0, 0, false),
        (0x35, 0, 0, true),
        (0x2710, 0, 0, true),
        (0x2711, 0, 0, true),
        (0x2711, 0, 0, false),
        (0x40, 0, 0, false),
    ];
    for (k, (state, count, flags, done)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x1800, k);
        junk(&mut t, src, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count);
        t.set_u8(r + 0x858, flags);
        let end = if done { 0x4e20 } else { 0x4e1f };
        t.set_u32(r + 0x1158, end);
        t.set_u32(r + 0xf4c, end);
        // the particle node's bit 2 (`+0xe8`) both ways
        t.set_u8(r + 0x770 + 0xe8, (k & 4) as u8);
        // the parents and the objects the callees read
        for n in [0x24u32, 0xbc, 0x1d8, 0xd54, 0x870, 0x770] {
            t.set_u32(r + n + 0x14, 0);
        }
        t.set_u32(r + 0x24 + 0x10, r + 0x24);
        t.set_u32(r + 0x110, src);
        t.set_u16(src + 0x90, (k & 1) as u16);
        t.set_u32(r + 0x78, 0x00c0_0078 + k);
        // the static lists: empty, or with a last node
        for (l, i) in [(list, 0u32), (plist, 1)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.set_u32(ssw(&t, 0x35), old);
        t.set_u32(ssw(&t, 0x50), 0x8000_0007);
        let sl = ssw(&t, 0x6a);
        t.write(sl, &[0x4e; 0x20]);
        t.set_u32(ssw(&t, 0x76), if k % 2 == 0 { sl } else { prev });
        t.set_u16(ssw(&t, 0x9c), 0x0305);
        junk_ops(&mut t, f, &[0x6c8, 0x6f6, 0x8d3, 0x8e1, 0x2cd]);
        let p = t.img.word(f + 0x635);
        t.set_u32(p, SCRATCH + 0x7e0);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x7ee0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}

/// `magic_0514` 0x10007de0 (`r25::summon_task_0514`): every state, each
/// time and count way, the model's alpha from slot 333 and from slot 601,
/// the static list empty and not, the end's flag words sharing a bit and
/// not, the jump table's dead entries and the states past it (the entry's
/// `eax` kept where the original never sets it).
#[test]
fn summon_task_0514() {
    let Some(mut t) = twin(514) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7de0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [
        (333u32, -0.75f64),
        (15, 0.5),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75 + (a[0] & 1) as f32);
        0x601
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0);
        }
        0x262
    });
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 3.0);
        }
        0x290
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 0.125 * i as f32 + 0.5);
        }
        0x268
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [
        (306u32, 3usize),
        (211, 2),
        (219, 2),
        (276, 2),
        (548, 1),
        (413, 1),
        (215, 1),
        (14, 2),
        (234, 3),
        (353, 1),
        (271, 1),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the particle node's globals (the colour set-up's particle node)
    let cs = t.img.rel32_target(f + 0x773);
    let (owner, list2) = (t.img.word(cs + 0x161), t.img.word(cs + 0x17a));
    let ob = SCRATCH + 0x7a00;
    t.set_u32(owner, ob);
    // the colour node's helper keeps slot 790's float
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let nums = SCRATCH + 0x6000;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        m.w32(nums, 0x30 + (a[0] & 1));
        m.w32(nums + 4, 0x31);
        nums
    });
    let list = op(&t, 0xef);
    let clist = op(&t, 0x400);
    let old = SCRATCH + 0x800;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let model = SCRATCH + 0x3000;
    let ring = op(&t, 0x596);
    // (state, count, time, R + 0x5a4, the end's words share 0x4e20, eax)
    let cases: [(u32, u32, u32, u8, bool); 22] = [
        (0, 3, 0x30, 1, false),
        (0, 3, 0x10, 0, false),
        (10, 0, 0x27, 0, false),
        (10, 0, 0x28, 0, false),
        (11, 0, 0x27, 0, false),
        (11, 0, 0x29, 0, false),
        (5, 1, 0, 0, false),
        (0x14, 0, 0, 0, false),
        (0x15, 2, 0, 0, false),
        (0x15, 0, 0, 0, true),
        (0x15, 0, 0, 0, false),
        (0x18, 0, 0, 0, false),
        (0x1e, 0, 0, 0, false),
        (0x1f, 1, 0, 0, false),
        (0x1f, 0, 0, 0, false),
        (0x20, 4, 0, 0, false),
        (0x20, 0, 0, 0, false),
        (0x2710, 0, 0, 0, false),
        (0x2710, 0, 0, 0, true),
        (0x2711, 0, 0, 0, false),
        (0x30, 0, 0, 0, false),
        (13, 0, 0, 0, false),
    ];
    for (k, (state, count, time, alpha, done)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x2800, k);
        junk(&mut t, model, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count);
        t.set_u32(r + 0x1c, time);
        t.set_u8(r + 0x5a4, alpha);
        let (x, y) = if done {
            (0x4e20, 0xffff)
        } else {
            (0x4e20, 0xb1df)
        };
        for (o, v) in [(0x2708u32, x), (0xc38, y), (0x968, 0xffff), (0x938, 0xffff)] {
            t.set_u32(r + o, v);
        }
        // the parents the nodes leave and the objects the callees read
        for n in [0x24u32, 0x514, 0x688, 0x7d4] {
            t.set_u32(r + n + 0x14, 0);
        }
        t.set_u32(r + 0x514 + 0x54, model);
        t.set_u32(r + 0x688 + 0x140, model);
        t.set_u32(model + 0x28, (2.0 + k as f32).to_bits());
        t.set_u16(model + 0x90, (k & 1) as u16);
        for o in [0x40u32, 0x94, 0x110] {
            t.set_u32(r + o, model);
        }
        // the static list: empty, or with a last node
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        junk(&mut t, clist, 0x20, k + 4);
        t.set_u32(clist + 0x10, clist);
        // the task ring's last
        let ring_end = SCRATCH + 0x880;
        t.write(ring_end, &[0x5d; 0x40]);
        t.set_u32(op(&t, 0x58c), ring_end);
        t.set_u32(ring, ring);
        // the colour node's words: moderate floats
        let cn = op(&t, 0x3f8);
        for i in 0..0x30 {
            t.set_u32(cn + 4 * i, (0.25 * i as f32 - 2.0 + k as f32).to_bits());
        }
        junk_ops(&mut t, f, &[0x75, 0xa7, 0x566, 0x5aa]);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        junk(&mut t, ob, 0x40, k + 6);
        t.set_u32(ob + 0x10, ob);
        junk(&mut t, list2, 0x20, k + 7);
        t.set_u32(list2 + 0x10, list2);
        ownb::dirty_stack(&mut t, 0x7de0 + k, &[]);
        let c = Call {
            eax: 0x0eaa_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}

/// `magic_0513` 0x10007a90 (`r25::summon_task_0513`): every state, each
/// count way, the model's alpha from slot 333 and from slot 601, the
/// share below 1.0, at it and NaN, the height above the target and not,
/// the static list empty and not, the end word with and without a bit of
/// 0x4e20, the jump table's dead entries and the states past it.
#[test]
fn summon_task_0513() {
    let Some(mut t) = twin(513) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7a90);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [
        (333u32, -0.75f64),
        (12, 3.5),
        (251, 0.75),
        (15, 0.5),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75 + (a[0] & 1) as f32);
        0x601
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0);
        }
        0x262
    });
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 3.0);
        }
        0x290
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 0.125 * i as f32 + 0.5);
        }
        0x268
    });
    for (s, n) in [(234u32, 3usize), (232, 2), (233, 3)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i) ^ (s << 8);
                m.w32(a[0] + 4 * i, v);
                fold(m, s, i, v);
            }
            s
        });
    }
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            fold(m, 325, i, x);
        }
        0x325
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [
        (306u32, 3usize),
        (211, 2),
        (219, 2),
        (276, 2),
        (548, 1),
        (413, 1),
        (215, 1),
        (14, 2),
        (353, 1),
        (271, 1),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the colour node's helper keeps slot 790's float
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let nums = SCRATCH + 0x6000;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        m.w32(nums, 0x30 + (a[0] & 1));
        m.w32(nums + 4, 0x31);
        nums
    });
    let list = op(&t, 0xd5);
    // the late set-ups' particle statics: the owner and the list
    let (ln, bt) = (t.img.rel32_target(f + 0x889), t.img.rel32_target(f + 0x962));
    let ob = SCRATCH + 0x7a00;
    let pls = [t.img.word(ln + 0x220), t.img.word(bt + 0x280)];
    t.set_u32(t.img.word(ln + 0x207), ob);
    t.set_u32(t.img.word(bt + 0x267), ob);
    let clist = op(&t, 0x65e);
    let old = SCRATCH + 0x800;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let model = SCRATCH + 0x3000;
    let ring = op(&t, 0x7f0);
    let nan = f32::NAN.to_bits();
    // (state, count, flag bit of the model, R + 0x54 (the share), the
    // height against the target, the end word)
    let cases: [(u32, u32, u16, u32, (f32, f32), u32); 26] = [
        (0, 3, 1, 0, (0.0, 0.0), 0),
        (0, 3, 0, 0, (0.0, 0.0), 0),
        (10, 0, 0, 0, (0.0, 0.0), 0),
        (11, 4, 0, 0, (0.0, 0.0), 0),
        (11, 0, 0, 0, (0.0, 0.0), 0),
        (5, 1, 0, 0, (0.0, 0.0), 0),
        (0x1e, 0, 0, 0, (0.0, 0.0), 0),
        (0x1f, 0, 0, 0, (0.0, 0.0), 0),
        (0x20, 2, 0, 0, (0.0, 0.0), 0),
        (0x20, 0, 0, 0, (0.0, 0.0), 0),
        (0x14, 0, 1, 0, (0.0, 0.0), 0),
        (0x15, 0, 0, 0x3e80_0000, (0.0, 0.0), 0),
        (0x15, 0, 0, 0x3f80_0000, (0.0, 0.0), 0),
        (0x15, 0, 0, nan, (0.0, 0.0), 0),
        (0x16, 0, 0, 0, (5.0, 4.0), 0),
        (0x16, 0, 0, 0, (4.0, 5.0), 0),
        (0x16, 0, 1, 0, (4.0, 4.0), 0),
        (0x3c, 0, 0, 0, (0.0, 0.0), 0),
        (0x3d, 3, 0, 0, (0.0, 0.0), 0),
        (0x3d, 0, 0, 0, (0.0, 0.0), 0x4e20),
        (0x3d, 0, 0, 0, (0.0, 0.0), 0x1),
        (0x2710, 0, 0, 0, (0.0, 0.0), 0x20),
        (0x2710, 0, 0, 0, (0.0, 0.0), 0x101),
        (0x2711, 0, 0, 0, (0.0, 0.0), 0x20),
        (0x40, 0, 0, 0, (0.0, 0.0), 0),
        (0x32, 0, 0, 0, (0.0, 0.0), 0),
    ];
    for (k, (state, count, flag, share, (h, top), end)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x1500, k);
        junk(&mut t, model, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x50, count);
        t.set_u32(r + 0x54, share);
        t.set_u32(r + 0x58, (0.125 + 0.0625 * k as f32).to_bits());
        for (i, o) in (0u32..).zip([0x1cu32, 0x20, 0x24, 0x2c, 0x30, 0x34]) {
            t.set_u32(r + o, (0.5 * i as f32 - 1.0).to_bits());
        }
        t.set_u32(r + 0x65c, h.to_bits());
        t.set_u32(r + 0x604, top.to_bits());
        t.set_u32(r + 0xac0, end);
        t.set_u16(model + 0x90, flag);
        t.set_u32(r + 0x54c + 0x54, model);
        t.set_u32(r + 0x54c + 0x50, model);
        t.set_u16(r + 0x54c + 0x90, flag);
        for o in [0xccu32, 0x59c, 0x5a0, 0x3c, 0x78] {
            t.set_u32(r + o, model);
        }
        // the parents the nodes leave and the objects the callees read
        for n in [0x5cu32, 0x54c, 0x694, 0x13d0, 0xb00, 0xdf0, 0x10e0] {
            t.set_u32(r + n + 0x14, 0);
        }
        junk(&mut t, ob, 0x40, k + 6);
        t.set_u32(ob + 0x10, ob);
        for l in pls {
            t.set_u32(l + 0x10, l);
        }
        // the node the late set-ups walk: no children
        for n in [0x694u32, 0x5e4, 0x54c] {
            t.set_u32(r + n + 0x10, r + n);
        }
        // the static list: empty, or with a last node
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        junk(&mut t, clist, 0x20, k + 4);
        t.set_u32(clist + 0x10, clist);
        // the task ring's last
        let ring_end = SCRATCH + 0x880;
        t.write(ring_end, &[0x5d; 0x40]);
        t.set_u32(op(&t, 0x7e6), ring_end);
        t.set_u32(ring, ring);
        // the colour node's words: moderate floats
        let cn = op(&t, 0x656);
        for i in 0..0x30 {
            t.set_u32(cn + 4 * i, (0.25 * i as f32 - 2.0 + k as f32).to_bits());
        }
        junk_ops(&mut t, f, &[0x5e, 0x8d, 0x7fe, 0x91f, 0x951]);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x7a90 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}

/// `magic_0603` 0x1000c840 (`r25::summon_task_0603`): every step (0 to 4
/// running on), the static list empty and not, the steps past the table.
#[test]
fn summon_task_0603() {
    let Some(mut t) = twin(603) else { return };
    all_stubs(&mut t);
    let f = t.at(0xc840);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [
        (17u32, 1.5f64),
        (277, 2.0),
        (333, -0.75),
        (15, 0.5),
        (16, 0.75),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 324, 0, |m, _| {
        let n = m.r32(SCRATCH + 0x7c0);
        m.w32(SCRATCH + 0x7c0, n + 1);
        0x1234 + 97 * n
    });
    for (s, n) in [
        (306u32, 3usize),
        (325, 2),
        (211, 2),
        (219, 2),
        (276, 2),
        (548, 1),
        (413, 1),
        (215, 1),
        (14, 2),
        (290, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let list = op(&t, 0x7b);
    let old = SCRATCH + 0x800;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let models = [op(&t, 0x262), op(&t, 0x279)];
    // the scene's ring: its last a block of its own
    let sc = t.img.rel32_target(f + 0x22d);
    let ring_last = t.img.word(sc + 0x38);
    let rl = SCRATCH + 0x880;
    let cases: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 0, 0xff];
    for (k, step) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x900, k);
        t.set_u8(r + 0x1c, step);
        t.set_u32(r + 0x18, (0.5 * k as f32).to_bits());
        for n in [0x370u32, 0x39c, 0xec, 0xc0, 0x118, 0x70, 0x600] {
            t.set_u32(r + n + 0x14, 0);
        }
        for m in models {
            junk(&mut t, m, 0x100, k + 5);
            t.set_u32(m + 0x14, 0);
            t.set_u16(m + 0x90, (k & 1) as u16);
        }
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        t.set_u32(op(&t, 0xd2), (3.0 + k as f32).to_bits());
        t.write(rl, &[0x5d; 0x40]);
        t.set_u32(ring_last, rl);
        let sw = |t: &Twin, o: u32| t.img.word(sc + o);
        let node = SCRATCH + 0x600;
        t.write(node, &[0x44; 0x20]);
        for o in [0x47u32, 0x7e, 0x8d] {
            t.set_u32(sw(&t, o), node);
        }
        let root = sw(&t, 0xdc);
        t.write(SCRATCH + 0x640, &[0x45; 0x20]);
        t.set_u32(
            sw(&t, 0xcc),
            if k % 2 == 0 { root } else { SCRATCH + 0x640 },
        );
        t.set_u16(sw(&t, 0xf0), 3);
        let parent = sw(&t, 0x2a8);
        t.set_u32(parent + 0xc, parent);
        t.set_u32(parent + 0x10, parent);
        junk_ops(&mut t, f, &[0x246, 0x250]);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0xc840 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x40, "the task's calls");
}

/// `magic_0603` 0x1000ccf0 (`r25::start_up_0603`): every model id way
/// (4203, 4209 with slot 535 answering 0, 2 and 1, 4232, 4233, 4236 and
/// another), the static list empty and not, the bone node stopping at
/// once.
#[test]
fn start_up_0603() {
    let Some(mut t) = twin(603) else { return };
    all_stubs(&mut t);
    let f = t.at(0xccf0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [
        (15u32, 0.5f64),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
        (289, -0.5),
        (333, -0.75),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75);
        0x601
    });
    for (s, n) in [(232u32, 2usize), (531, 2)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..4 {
                wf(m, a[0] + 4 * i, 0.5 * i as f32 - 1.25 + (s & 3) as f32);
            }
            s
        });
    }
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[2] + 4 * i);
            fold(m, 248, i, x);
        }
        0x248
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            fold(m, 234, i, x);
        }
        0x234
    });
    let ids = SCRATCH + 0x7c0;
    rec(&mut t, 390, 1, move |m, _| m.r32(ids));
    rec(&mut t, 535, 1, move |m, _| m.r32(ids + 4));
    let lock = SCRATCH + 0x7d0;
    rec(&mut t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
    for (s, n) in [
        (803u32, 2usize),
        (217, 2),
        (215, 1),
        (262, 4),
        (718, 1),
        (710, 4),
        (222, 4),
        (221, 4),
        (551, 3),
        (10, 2),
        (325, 2),
        (684, 1),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let list = op(&t, 0x1c0);
    let old = SCRATCH + 0x800;
    let bone = op(&t, 0x5b3);
    let pass = op(&t, 0x55d);
    // (model id, slot 535's answer)
    let cases: [(u32, u32); 9] = [
        (4203, 0),
        (4209, 0),
        (4209, 2),
        (4209, 1),
        (4232, 0),
        (4233, 0),
        (4236, 0),
        (4300, 0),
        (4100, 0),
    ];
    for (k, (id, ans)) in (0u32..).zip(cases) {
        t.set_u32(ids, id);
        t.set_u32(ids + 4, ans);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(op(&t, 0xa86), lock);
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        // the bone node's walk stops at once (the start-up sets its pass
        // halfword to 0xff)
        t.set_u8(pass, 0xff);
        t.set_u8(bone + 0x1a, 0x30 + k as u8);
        junk_ops(&mut t, f, &[0x5b, 0x781, 0x8c7, 0x75a, 0x97f, 0x8fc]);
        t.set_u32(op(&t, 0x781), (2.5 + k as f32).to_bits());
        for o in [0x97fu32, 0x8fc] {
            t.set_u32(op(&t, o), (1.5 * k as f32).to_bits());
        }
        ownb::dirty_stack(&mut t, 0xccf0 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
}

/// `magic_0603` 0x1000b9e0 (`r25::camera_task_0603`): every state (0, 4,
/// 5 with the aim flag 1, 2 and 0, 6, 100 with each model id and slot
/// 535's answers, 101), slot 520's three answers, the ramps' time below,
/// between and past each bound (NaN once) with the move flag set and not,
/// and states outside the table and past it.
#[test]
fn camera_task_0603() {
    let Some(mut t) = twin(603) else { return };
    all_stubs(&mut t);
    let f = t.at(0xb9e0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        f64::from(m.r32(magic_owna::row(307)) % 7) * 0.13 + 0.02
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[0] + 4 * i, 0.5 * i as f32 - 1.25 + (a[1] & 7) as f32);
        }
        0x232
    });
    for (s, n) in [(233u32, 3usize), (248, 3)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..4 {
                let x = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
                fold(m, s, i, x);
            }
            s
        });
    }
    let cell = SCRATCH + 0x7c0;
    rec(&mut t, 520, 0, move |m, _| m.r32(cell));
    rec(&mut t, 535, 1, move |m, _| m.r32(cell + 4));
    for (s, n) in [(286u32, 2usize), (211, 2), (325, 2), (285, 2), (250, 2)] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let list = op(&t, 0x111);
    let old = SCRATCH + 0x800;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let tm_at = op(&t, 0x18);
    // (state, *G_AIM, slot 520, model id, slot 535, time, move flag)
    let cases: [(u32, u32, u32, u32, u32, f32, u32); 24] = [
        (0, 0, 0x0247_0000, 0, 0, 0.0, 0),
        (4, 2, 0x024a_0000, 0, 0, 0.0, 0),
        (5, 1, 0, 0x1088, 0, 0.0, 0),
        (5, 0, 0x0123_0000, 0, 0, 0.0, 0),
        (5, 2, 0x0247_0000, 0, 0, 0.0, 0),
        (6, 0, 0, 0, 0, 1.0, 1),
        (6, 0, 0, 0, 0, 40.0, 1),
        (6, 0, 0, 0, 0, 40.0, 0),
        (6, 0, 0, 0, 0, 70.0, 0),
        (6, 0, 0, 0, 0, 100.0, 0),
        (6, 0, 0, 0, 0, 1000.0, 1),
        (6, 0, 0, 0, 0, f32::NAN, 0),
        (0x64, 0, 0, 0x1088, 0, 0.0, 0),
        (0x64, 0, 0, 0x1089, 0, 0.0, 0),
        (0x64, 0, 0, 0x108c, 0, 0.0, 0),
        (0x64, 0, 0, 0x1090, 0, 0.0, 0),
        (0x64, 0, 0, 0x1090, 2, 0.0, 0),
        (0x64, 0, 0, 0x1090, 1, 100.0, 1),
        (0x65, 0, 0, 0, 0, 1.0, 0),
        (0x65, 0, 0, 0, 0, 1000.0, 1),
        (0x65, 0, 0, 0, 0, 55.0, 0),
        (3, 0, 0, 0, 0, 0.0, 0),
        (0x50, 0, 0, 0, 0, 0.0, 0),
        (0x70, 0, 0, 0, 0, 0.0, 0),
    ];
    for (k, (state, aim, kind, id, ans, time, flag)) in (0u32..).zip(cases) {
        t.set_u32(op(&t, 0x26), state);
        t.set_u32(op(&t, 0xc9), aim);
        t.set_u32(cell, kind);
        t.set_u32(cell + 4, ans);
        t.set_u32(op(&t, 0x5f2), id);
        t.set_u32(tm_at, time.to_bits());
        t.set_u32(op(&t, 0x419), flag);
        t.set_u32(op(&t, 0x9a1), flag);
        // moderate floats in the camera words
        for o in [
            0x1b1u32, 0x1db, 0x1ef, 0x35c, 0x364, 0x388, 0x390, 0x3c4, 0x49d, 0x4a5, 0x4eb, 0x529,
            0x546, 0x54e, 0x577,
        ] {
            t.set_u32(
                op(&t, o),
                (0.75 + 0.5 * k as f32 + (o & 0xf) as f32).to_bits(),
            );
        }
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        let leave = op(&t, 0xb1d);
        junk(&mut t, leave, 0x20, k + 4);
        t.set_u32(leave + 0x14, 0);
        let cam = op(&t, 0x57);
        junk(&mut t, cam, 0x300, k + 5);
        // the camera tree's parent: an empty child ring
        let par = op(&t, 0x5f);
        t.set_u32(par + 0xc, par);
        t.set_u32(par + 0x10, par);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0xb9e0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}

/// `magic_0585` 0x10008bc0 (`r25::start_up_0585`): no targets and two,
/// the ring's old last varied, the lock word varied.
#[test]
fn start_up_0585() {
    let Some(mut t) = twin(585) else { return };
    all_stubs(&mut t);
    let f = t.at(0x8bc0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    let list = SCRATCH + 0x6000;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(list + 0x7c));
        list
    });
    rec(&mut t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
    let lock = SCRATCH + 0x7f0;
    rec(&mut t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [(718u32, 1usize), (710, 4), (222, 4), (221, 4), (708, 1)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let old = SCRATCH + 0x6800;
    for (k, count) in [0u32, 2, 1].into_iter().enumerate() {
        let k = k as u32;
        t.write(list, &[0x80, 0x81, 0x83, 0x84]);
        t.set_u32(list + 0x7c, count);
        t.set_u32(w(&t, 0xd4), lock);
        t.set_u32(lock, 0x40 + k);
        t.write(old, &[0x33; 0x40]);
        t.set_u32(w(&t, 0x1f3), if k == 1 { w(&t, 0x21f) } else { old });
        junk_ops(&mut t, f, &[0x230, 0xdd, 0x1fb]);
        ownb::dirty_stack(&mut t, 0x8bc0 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
}

/// `magic_0585` 0x10007e30 (`r25::glow_task_0585`): every state, the
/// levels below, at and past their bounds (NaN once), the count's two
/// ways, the wait running and out, and states past the table.
#[test]
fn glow_task_0585() {
    let Some(mut t) = twin(585) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7e30);
    for (s, n) in [(233u32, 3usize), (248, 3), (232, 2)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..4 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + 0.25 * i as f32 + (s & 3) as f32;
                wf(m, a[0] + 4 * i, v);
                fold(m, s, i, v.to_bits());
            }
            s
        });
    }
    for (s, n) in [(589u32, 2usize), (590, 2)] {
        rec(&mut t, s, n, move |m, a| {
            let x = m.r32(a[1]) ^ m.r32(a[1] + 8);
            fold(m, s, a[0] & 3, x);
            (s << 16) ^ a[0]
        });
    }
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1000;
    let nan = f32::NAN;
    // (state, level b0, level b4, count d8, wait d4)
    let cases: [(u32, f32, f32, u32, u32); 20] = [
        (0, 0.1, 0.0, 5, 3),
        (1, 0.5, 0.0, 1, 3),
        (1, 0.99, 0.0, 1, 3),
        (1, nan, 0.0, 1, 3),
        (2, 0.5, 0.0, 1, 3),
        (2, 0.01, 0.0, 1, 3),
        (2, 0.01, 0.0, 0, 3),
        (0x32, 0.2, 0.2, 0, 3),
        (0x33, 0.9, 0.99, 0, 3),
        (0x33, 0.5, 0.5, 0, 3),
        (10, 0.5, 0.5, 0, 0),
        (11, 0.5, 0.5, 0, 2),
        (11, 1.0, 1.0, 0, 0),
        (0x2710, 0.5, 0.5, 0, 0),
        (0x2710, 0.01, 0.01, 0, 0),
        (0x2710, 0.5, nan, 0, 0),
        (5, 0.5, 0.25, 0, 7),
        (0x40, 0.5, 0.25, 0, 7),
        (0x4e20, 0.5, 0.25, 0, 7),
        (0x3000, 0.5, 0.25, 0, 7),
    ];
    for (k, (state, b0, b4, n, wait)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x100, k);
        for i in 0..0x24 {
            t.set_u32(r + 4 * i, (0.25 * i as f32 - 2.0 + k as f32).to_bits());
        }
        t.set_u32(r + 0xd0, state);
        t.set_u32(r + 0xb0, b0.to_bits());
        t.set_u32(r + 0xb4, b4.to_bits());
        t.set_u32(r + 0xd8, n);
        t.set_u32(r + 0xd4, wait);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x7e30 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0585` 0x10008450 (`r25::summon_task_0585`): every state, each
/// count way, the party lists empty and with members, slot 411 answering
/// and not, the static list empty and not, states past the table.
#[test]
fn summon_task_0585() {
    let Some(mut t) = twin(585) else { return };
    all_stubs(&mut t);
    let f = t.at(0x8450);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [
        (15u32, 0.5f64),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
        (333, -0.75),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0);
        }
        0x262
    });
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 3.0);
        }
        0x290
    });
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75);
        0x601
    });
    let cell = SCRATCH + 0x7c0;
    let members = SCRATCH + 0x7d0;
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(cell));
        members + (a[0] & 1)
    });
    rec(&mut t, 411, 1, move |m, a| m.r32(cell + 4) ^ (a[0] & 0x100));
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [
        (215u32, 1usize),
        (399, 2),
        (296, 2),
        (228, 2),
        (709, 1),
        (219, 2),
        (276, 2),
        (548, 1),
        (413, 1),
        (14, 2),
        (325, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let list = op(&t, 0x55);
    let old = SCRATCH + 0x800;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let model = SCRATCH + 0x3000;
    // (state, count, party size, slot 411)
    let cases: [(u32, u32, u32, u32); 22] = [
        (0, 3, 2, 0),
        (0, 3, 0, 0),
        (0x28, 5, 0, 0),
        (0x29, 2, 0, 0),
        (0x29, 0, 0, 0),
        (0x2c, 0, 0, 0),
        (0x2f, 0, 0, 0),
        (0x30, 1, 0, 0),
        (0x30, 0, 0, 0),
        (10, 2, 0, 0),
        (10, 0, 3, 0),
        (11, 1, 0, 0),
        (11, 0, 0, 0),
        (12, 4, 0, 0),
        (12, 0, 2, 1),
        (12, 0, 0, 0),
        (0x2710, 0, 2, 1),
        (0x2710, 0, 0, 0),
        (5, 0, 0, 0),
        (0x31, 0, 0, 0),
        (0x2711, 0, 0, 0),
        (0x4e20, 0, 0, 0),
    ];
    for (k, (state, count, size, ans)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x1900, k);
        junk(&mut t, model, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count);
        t.set_u16(model + 0x90, (k & 1) as u16);
        t.set_u32(r + 0x24 + 0x54, model);
        t.set_u32(r + 0x24 + 0x50, model);
        t.set_u32(r + 0x24 + 0x14, if k % 3 == 0 { 0 } else { old });
        t.set_u32(cell, size);
        t.set_u32(cell + 4, ans);
        t.write(members, &[0x01, 0x02, 0x03, 0x04]);
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        junk_ops(&mut t, f, &[0x42, 0x4ec, 0x567]);
        // the DLL's task ring: its last a block of its own
        let ss = t.img.rel32_target(f + 0x2f8);
        let ring_end = SCRATCH + 0x880;
        t.write(ring_end, &[0x5d; 0x40]);
        t.set_u32(t.img.word(ss + 0x1d), ring_end);
        // the light's node list: empty
        let lo = t.img.rel32_target(f + 0x488);
        let ll = t.img.word(lo + 0x6a);
        t.set_u32(ll + 0x10, ll);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x8450 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}
