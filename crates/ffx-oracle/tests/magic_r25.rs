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
        let z = m.r32(a[1] + 0x38);
        fold(m, 361, 0, z);
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
    rec(&mut t, 885, 7, move |m, a| {
        let x = m.r32(a[6]) ^ a[0];
        fold(m, 885, 0, x);
        m.w32(a[6], 0x0885_0000 ^ a[1]);
        m.r32(cell)
    });
    rec(&mut t, 879, 10, |m, a| {
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
