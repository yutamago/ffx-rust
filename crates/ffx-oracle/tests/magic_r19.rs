//! Round 19's ports (`ffx_magic::r19`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{rec, row, twin};

/// Junk in the stack below the entry (the frames' locals start as it).
fn scrub(t: &mut Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

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

/// An empty list head at `l` (its child list itself).
fn empty_list(t: &mut Twin, l: u32) {
    t.write(l, &[0; 0x20]);
    t.set_u32(l + 0x10, l);
    t.set_u32(l + 0xc, l);
}

/// A task at `SCRATCH + 0x700` for `obj` (its node in a ring of three).
fn summon_task_at(t: &mut Twin, obj: u32) -> u32 {
    let (task, tn, a, b) = (
        SCRATCH + 0x700,
        SCRATCH + 0x740,
        SCRATCH + 0x7c0,
        SCRATCH + 0x7e0,
    );
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, obj);
    task
}

/// Where a test keeps an answer a stub gives.
const ANSWER_CELL: u32 = SCRATCH + 0x98;

/// The slots the summon tasks' callees make, as plain records: the model
/// slots (218 gives `count` model numbers), the float slots (12 the
/// ground, 333 an alpha), 237/262 fill vectors, 248 adds, 313 answers
/// [`ANSWER_CELL`], 941 a program table.
#[allow(clippy::too_many_lines)] // one stub per slot
fn summon_slots(t: &mut Twin, k: u32) {
    for (s, n) in [
        (413u32, 1usize),
        (215, 1),
        (14, 1),
        (211, 2),
        (228, 2),
        (222, 6),
        (36, 2),
        (257, 2),
        (296, 2),
        (38, 3),
        (541, 3),
        (538, 2),
        (213, 2),
        (35, 3),
        (34, 2),
        (547, 1),
        (276, 1),
        (219, 2),
        (331, 1),
        (353, 1),
        (271, 1),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, 2);
    rec(t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    rec(t, 558, 2, |m, a| {
        for j in 0..4 {
            m.w32(a[0] + 4 * j, 0x20 + 3 * j);
        }
        0x558
    });
    rec(t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.625);
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |_, _| 0.5 + f64::from(k));
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32 - k as f32);
        }
        0x262
    });
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 - 1.0);
        }
        0x237
    });
    rec(t, 248, 5, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 313, 1, |m, _| m.r32(ANSWER_CELL));
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    for s in [20u32, 411] {
        rec(t, s, 1, |m, _| m.r32(ANSWER_CELL));
    }
    rec(t, 217, 2, move |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 1.5 * i as f32 - 2.0 + k as f32);
        }
        0x217
    });
    rec(t, 601, 2, move |m, a| {
        wf(m, a[1], 0.375 + k as f32);
        0x601
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 234, i, v.to_bits());
        }
        0x234
    });
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, v);
            fold(m, 232, i, v.to_bits());
        }
        0x232
    });
    rec(t, 239, 3, |_, a| 0x0239_0000 ^ a[1]);
    rec(t, 30, 1, |_, a| 0x0030_0000 ^ a[0]);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
}

/// The particle node a [`particle_node_with`] copy at `pnw` starts: its
/// list empty, its part start's lock, its owner's vectors.
fn particle_with(t: &mut Twin, pnw: u32, owner: u32) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    empty_list(t, op(t, pnw, 0x26));
    let pn = t.img.rel32_target(pnw + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    t.set_u32(op(t, pnw, 0x0d), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
}

/// `magic_0343` 0x10006ad0 (`r19::summon_task_0343`): the record's
/// `(state, +0x28, +0x2c)`, slot 313's answer, the static model's and
/// the vector's flag bits (`bits`: 1 the flag step's, 2 the vector's), a
/// current list node or none.
fn summon_0343_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n28, n2c): (u32, u32, u32),
    (answer, bits, current): (u32, u8, bool),
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    t.set_u32(ANSWER_CELL, answer);
    empty_list(t, op(t, f, 0xa8));
    for (call, owner) in [(0x247u32, 0x7a00u32), (0x55b, 0x7b00), (0x6e1, 0x7c00)] {
        let pnw = t.img.rel32_target(f + call);
        particle_with(t, pnw, SCRATCH + owner);
    }
    let enb = t.img.rel32_target(f + 0x25e);
    for o in [0x18u32, 0x1f] {
        empty_list(t, op(t, enb, o));
    }
    for o in [0x283u32, 0x2bc, 0x1b5] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    let sfs = t.img.rel32_target(f + 0x310);
    t.set_u8(op(t, sfs, 0x02), 0x40 | (bits & 1));
    let vs = t.img.rel32_target(f + 0x35a);
    t.set_u8(op(t, vs, 0x06), 0x80 | (bits >> 1 & 1));
    // the summon's model node (0x100 bytes, up to the camera block)
    let node = op(t, f, 0x5a4);
    t.write(node, &[0x22; 0x100]);
    // the camera blocks' sources: real floats; their outputs junk
    for o in [
        0x91u32, 0x9c, 0xca, 0x105, 0x140, 0x153, 0x166, 0x16e, 0x17e, 0x186, 0x393, 0x39e, 0x3c1,
        0x3fc, 0x437, 0x459, 0x46c, 0x474, 0x484, 0x48c, 0x331, 0x4d2, 0x4da,
    ] {
        t.set_u32(op(t, f, o), (0.125 * o as f32 - 20.0 + k as f32).to_bits());
    }
    junk_ops(
        t,
        f,
        &[
            0xa2, 0xb3, 0xbf, 0xd0, 0xfa, 0x10b, 0x11f, 0x129, 0x135, 0x148, 0x15b, 0x176, 0x18e,
            0x196, 0x1ca, 0x1e1, 0x1eb, 0x1f5, 0x1ff, 0x241, 0x267, 0x26e, 0x3a3, 0x3b6, 0x3c7,
            0x3f1, 0x420, 0x42c, 0x442, 0x44e, 0x461, 0x47c, 0x494, 0x49c, 0x4ea, 0x4f6, 0x516,
            0x534, 0x550, 0x578, 0x582, 0x664, 0x67b, 0x699, 0x6db, 0x701, 0x708,
        ],
    );
    let lu = t.img.rel32_target(f + 0x5e4);
    let cur = op(t, lu, 0x02);
    if current {
        let n = SCRATCH + 0x7600;
        t.write(n, &[0x33; 0x40]);
        t.set_u32(n + 4, n);
        t.set_u32(n + 8, n);
        t.set_u32(cur, n);
    } else {
        t.set_u32(cur, 0);
    }
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x28, n28), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0343` 0x10006ad0 (`r19::summon_task_0343`): every state, the
/// counts at their bounds, slot 313's two answers, both ways of the flag
/// steps, the unlink with and without a current node, states no case
/// takes.
#[test]
fn summon_task_0343() {
    let Some(mut t) = twin(343) else { return };
    let f = t.at(0x6ad0);
    for (k, (st, more)) in [
        ((0u32, 0u32, 5u32), (0u32, 0u8, false)),
        ((0, 0, 5), (0, 1, false)),
        ((10, 0, 5), (0, 0, false)),
        ((0x1e, 0, 0xa9), (0, 0, false)),
        ((0x1e, 0, 0xaa), (0, 0, false)),
        ((0x3c, 0, 0xffff_ffff), (0, 0, false)),
        ((0x3c, 0, 0), (1, 0, false)),
        ((0x3c, 0, 0), (0, 0, false)),
        ((0x32, 0x117, 3), (0, 0, false)),
        ((0x32, 0x118, 3), (0, 0, false)),
        ((0x32, 0x118, 3), (0, 2, false)),
        ((0x33, 0x149, 3), (0, 0, false)),
        ((0x33, 0x14a, 3), (0, 0, false)),
        ((0x34, 0, 0x59), (0, 0, false)),
        ((0x34, 0, 0x5a), (0, 0, false)),
        ((0x35, 0, 0x77), (0, 0, false)),
        ((0x35, 0, 0x78), (0, 0, false)),
        ((0x2710, 0, 0x1d), (0, 0, false)),
        ((0x2710, 0, 0x1e), (0, 0, false)),
        ((0x2710, 0, 0x1e), (0, 0, true)),
        ((5, 0, 0x1e), (0, 0, false)),
        ((0x31, 0, 0x1e), (0, 0, false)),
        ((0x36, 0, 0x1e), (0, 0, false)),
        ((0x3b, 0, 0x1e), (0, 0, false)),
        ((0x2711, 0, 0x1e), (0, 0, false)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0343_setup(&mut t, f, k, st, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [
        (218u32, 2u32),
        (237, 2),
        (313, 2),
        (222, 1),
        (248, 1),
        (262, 1),
    ] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// Where `summon_0202_setup` keeps the distance base slot 394 adds its
/// call count to.
const DIST_CELL: u32 = SCRATCH + 0xa0;

/// `magic_0202` 0x10006f70 (`r19::summon_task_0202`): the record's
/// `(state, +0x2c)`, the answer of slots 20/411, the flag byte, `n`
/// targets with their flags, the distance base (slot 394 answers
/// `K - 1 + (base + call) % 3`).
fn summon_0202_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n2c): (u32, u32),
    (answer, flag, n, flags, base): (u32, u8, u32, [u32; 3], u32),
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    t.set_u32(ANSWER_CELL, answer);
    let kk = f32::from_bits(t.u32(op(t, f, 0x396)));
    t.set_u32(DIST_CELL, base);
    rec(t, 394, 2, |_, _| 0x394);
    t.stub_st0(394, move |m, _| {
        let c = m.r32(row(394)) + m.r32(DIST_CELL);
        f64::from(kk) - 1.0 + f64::from(c % 3)
    });
    let list = op(t, f, 0x9f);
    empty_list(t, list);
    particle_with(t, t.img.rel32_target(f + 0x1a6), SCRATCH + 0x7a00);
    let pn = t.img.rel32_target(f + 0x5ee);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(op(t, wrapper, 0x04), SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7b00;
    t.set_u32(op(t, f, 0x5b4), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
    }
    for o in [0x6ddu32, 0x716, 0x64, 0x6f, 0x608] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u8(op(t, f, 0x56), 0x20 | flag);
    t.set_u32(op(t, f, 0x2b8), n);
    // the object: its flags, model and object, a child of the list
    let obj = op(t, f, 0x689);
    t.write(obj, &[0x26; 0x100]);
    t.set_u8(obj + 0x90, 0x10 | flag);
    for (o, v) in [(4u32, list), (8, list), (0x14, list)] {
        t.set_u32(obj + o, v);
    }
    t.set_u32(list + 0xc, obj);
    t.set_u32(list + 0x10, obj);
    t.set_u16(list + 0x18, 1);
    // the targets: actor, id, the record before them
    for i in 0..3u32 {
        let tg = op(t, f, 0x2d7) + 0x98 * i;
        t.write(tg - 0x54, &[0x31; 0x98]);
        t.set_u8(tg - 0x54 + 0x90, (i as u8) & 1);
        t.set_u32(tg, 0x0b0b_0000 + 0x10 * i + k);
        t.set_u32(tg - 4, 0x0c0c_0000 + i);
    }
    for o in [0x32bu32, 0x359, 0x4db] {
        let at = op(t, f, o);
        for j in 0..4 {
            t.set_u32(at + 4 * j, (0.625 * j as f32 + 0.25 * o as f32).to_bits());
        }
    }
    junk_ops(
        t,
        f,
        &[
            0x10b, 0x115, 0x1a0, 0x29b, 0x248, 0x252, 0x25c, 0x66b, 0x77e, 0x785, 0x769,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    for i in 0..3 {
        t.set_u32(r + 0x30 + 4 * i, flags[i as usize]);
    }
    summon_task_at(t, r)
}

/// `magic_0202` 0x10006f70 (`r19::summon_task_0202`): every state, the
/// count at its bound, both flag ways, targets flagged or not, near, at
/// and beyond the distance, none left and some left, no targets.
#[test]
fn summon_task_0202() {
    let Some(mut t) = twin(202) else { return };
    let f = t.at(0x6f70);
    for (k, (st, more)) in [
        ((0u32, 5u32), (0u32, 0u8, 2u32, [0u32; 3], 0u32)),
        ((10, 5), (0, 0, 2, [0; 3], 0)),
        ((10, 5), (0, 1, 2, [0; 3], 0)),
        ((11, 0x19), (0, 0, 2, [0x77; 3], 0)),
        ((11, 0x1a), (0, 0, 3, [0x77; 3], 0)),
        ((11, 0x1a), (0, 1, 0, [0x77; 3], 0)),
        ((12, 9), (0, 0, 3, [1, 0, 1], 0)),
        ((13, 9), (0, 0, 3, [1, 0, 1], 1)),
        ((13, 9), (0, 1, 3, [1, 1, 1], 2)),
        ((13, 9), (0, 0, 2, [0, 0, 1], 0)),
        ((13, 9), (0, 0, 0, [1, 1, 1], 0)),
        ((14, 9), (0, 0, 2, [0; 3], 0)),
        ((14, 9), (0, 1, 2, [0; 3], 0)),
        ((14, 9), (1, 0, 2, [0; 3], 0)),
        ((14, 9), (1, 1, 2, [0; 3], 0)),
        ((0x2710, 9), (0, 0, 2, [0; 3], 0)),
        ((5, 9), (0, 0, 2, [0; 3], 0)),
        ((15, 9), (0, 0, 2, [0; 3], 0)),
        ((0x270f, 9), (0, 0, 2, [0; 3], 0)),
        ((0x2711, 9), (0, 0, 2, [0; 3], 0)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0202_setup(&mut t, f, k, st, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(394u32, 6u32), (239, 3), (262, 12), (20, 2), (411, 2)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// The particle node at `pn` started: its part start's lock and pointer.
fn part_start(t: &mut Twin, pn: u32) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(op(t, wrapper, 0x04), SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
}

/// Floats at the globals the operands `ops` of `f` name (varied by `k`).
fn floats_at(t: &mut Twin, f: u32, k: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0.125 * o as f32 - 20.0 + k as f32).to_bits());
    }
}

/// `magic_0144` 0x10006780 (`r19::summon_task_0144`): the record's
/// `(state, +0x28, target)`.
fn summon_0144_setup(t: &mut Twin, f: u32, k: u32, (state, n28, target): (u32, u32, u32)) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    for o in [0x1f9u32, 0x4b7] {
        empty_list(t, op(t, f, o));
    }
    part_start(t, t.img.rel32_target(f + 0x4e2));
    let owner = SCRATCH + 0x7b00;
    t.set_u32(op(t, f, 0x46a), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
    }
    t.set_u32(op(t, f, 0x1e5), 0x0a0a_0000 + k);
    floats_at(
        t,
        f,
        k,
        &[
            0x49, 0xc0, 0xe5, 0xed, 0x128, 0x151, 0x159, 0x16c, 0x17c, 0x184, 0x197, 0x1a7, 0x1af,
        ],
    );
    junk_ops(
        t,
        f,
        &[
            0xc5, 0xd1, 0xdd, 0x11d, 0x12e, 0x149, 0x161, 0x174, 0x18c, 0x19f, 0x1b7, 0x242, 0x2a3,
            0x328, 0x349, 0x353, 0x35d, 0x465, 0x4a6, 0x563, 0x59b, 0x5f4, 0x6d1, 0x75e, 0x83b,
            0x93a,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x28, n28), (0x24, target)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0144` 0x10006780 (`r19::summon_task_0144`): every state, each
/// wait at its bound (the count is read after the entry's increment),
/// states the byte table sends to the default and states past it.
#[test]
fn summon_task_0144() {
    let Some(mut t) = twin(144) else { return };
    let f = t.at(0x6780);
    for (k, st) in [
        (0u32, 5u32, 1u32),
        (10, 5, 2),
        (11, 0x12, 0),
        (11, 0x13, 1),
        (20, 0x13, 2),
        (21, 0x17, 0),
        (21, 0x18, 1),
        (30, 0x18, 2),
        (31, 0x1c, 0),
        (31, 0x1d, 0),
        (40, 0x1d, 0),
        (41, 0x17, 0),
        (41, 0x18, 0),
        (42, 0x3f, 0),
        (42, 0x40, 0),
        (0x2710, 3, 0),
        (5, 3, 0),
        (12, 3, 0),
        (0x2a, 3, 0),
        (0x2b, 3, 0),
        (0x2711, 3, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0144_setup(&mut t, f, k, st);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(719u32, 2u32), (941, 6), (218, 1)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// `magic_0207` 0x10006b00 (`r19::summon_task_0207`): the record's
/// `(state, +0x2c)` and slot 411's answer.
fn summon_0207_setup(t: &mut Twin, f: u32, k: u32, (state, n2c): (u32, u32), answer: u32) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    rec(t, 701, 11, |_, a| 0x0701_0000 ^ a[4]);
    rec(t, 213, 2, |_, a| 0x0213_0000 ^ a[0]);
    t.set_u32(ANSWER_CELL, answer);
    empty_list(t, op(t, f, 0xdc));
    particle_with(t, t.img.rel32_target(f + 0x40e), SCRATCH + 0x7a00);
    let ena = t.img.rel32_target(f + 0x195);
    for o in [0x18u32, 0x1f] {
        empty_list(t, op(t, ena, o));
    }
    for o in [0xc8u32, 0x101, 0x44, 0x21b, 0x228, 0x96e] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u8(op(t, f, 0x73a), 0x5c + k as u8);
    floats_at(
        t,
        f,
        k,
        &[
            0x4a6, 0x4b1, 0x4df, 0x51a, 0x555, 0x568, 0x57b, 0x583, 0x593, 0x59b, 0x7ed, 0x7f8,
            0x81b, 0x848, 0x883, 0x8be, 0x8db, 0x8e3, 0x8f3, 0x8fb,
        ],
    );
    junk_ops(
        t,
        f,
        &[
            0x158, 0x15f, 0x1b1, 0x1b8, 0x245, 0x256, 0x26a, 0x27e, 0x2d1, 0x31b, 0x336, 0x35d,
            0x3a7, 0x3b8, 0x4b7, 0x4d4, 0x50f, 0x53e, 0x54a, 0x55d, 0x570, 0x58b, 0x5a3, 0x5ab,
            0x5ed, 0x5f7, 0x639, 0x67d, 0x687, 0x6bf, 0x749, 0x7a3, 0x7fd, 0x810, 0x83d, 0x878,
            0x8a7, 0x8b3, 0x8d0, 0x8eb, 0x903, 0x90b,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0207` 0x10006b00 (`r19::summon_task_0207`): every state, the
/// counts at their bounds, slot 411's two answers (the waits that run the
/// next state at once), states the byte table sends to the default and
/// states past it.
#[test]
fn summon_task_0207() {
    let Some(mut t) = twin(207) else { return };
    let f = t.at(0x6b00);
    for (k, (st, answer)) in [
        ((0u32, 5u32), 0u32),
        ((10, 5), 0),
        ((11, 5), 0),
        ((11, 5), 1),
        ((20, 5), 0),
        ((21, 2), 0),
        ((21, 3), 0),
        ((22, 5), 0),
        ((22, 5), 1),
        ((23, 3), 0),
        ((23, 4), 0),
        ((24, 0xf), 0),
        ((24, 0x10), 0),
        ((25, 5), 0),
        ((25, 5), 1),
        ((30, 5), 0),
        ((31, 5), 0),
        ((31, 5), 1),
        ((32, 5), 0),
        ((32, 5), 1),
        ((0x2710, 5), 0),
        ((5, 5), 0),
        ((0x20, 5), 0),
        ((0x21, 5), 0),
        ((0x2711, 5), 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0207_setup(&mut t, f, k, st, answer);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(701u32, 3u32), (411, 10), (239, 1), (213, 2), (215, 4)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// Where `summon_0106_setup` keeps how many target bytes slot 288 gives.
const IDS_CELL: u32 = SCRATCH + 0xa4;

/// `magic_0106` 0x10006c30 (`r19::summon_task_0106`): the record's
/// `(state, +0x2c)`, the flag `R + 0x74`, the kept ids `R + 0x30` (count
/// `R + 0x70`) and how many bytes slot 288 gives.
fn summon_0106_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n2c): (u32, u32),
    (flag, ids, n288): (u32, u16, u32),
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    for (s, n) in [(296u32, 2usize), (878, 1), (684, 1)] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0) ^ 1
        });
    }
    rec(t, 845, 0, |_, _| 0x845);
    rec(t, 531, 2, move |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.75 * i as f32 + a[0] as f32 - k as f32);
        }
        0x531
    });
    let bytes = SCRATCH + 0x320;
    t.write(bytes, &[3, 9, 4, 11, 2, 0]);
    t.set_u32(IDS_CELL, n288);
    rec(t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(IDS_CELL));
        bytes
    });
    for o in [0x5au32, 0x109] {
        empty_list(t, op(t, f, o));
    }
    for (call, o) in [(0x3b3u32, 0x374u32), (0x650, 0x611), (0x79f, 0x753)] {
        part_start(t, t.img.rel32_target(f + call));
        let owner = SCRATCH + 0x7b00;
        t.set_u32(op(t, f, o), owner);
        for j in 0..0x40 {
            t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
        }
    }
    let lk = t.img.rel32_target(f + 0x199);
    t.set_u32(op(t, lk, 0x1), SCRATCH + 0x7f20);
    t.set_u32(SCRATCH + 0x7f20, 0x0123_4567);
    let ce = t.img.rel32_target(f + 0x98);
    t.set_u32(op(t, ce, 0x48), 2);
    t.write(op(t, ce, 0x55), &[5, 7]);
    // the targets: 3 and 4 are known
    t.set_u32(op(t, f, 0x4fe), 2);
    for (j, id) in [(0u32, 3u32), (1, 4)] {
        t.set_u32(op(t, f, 0x50c) + 0x98 * j, id);
    }
    t.set_u32(op(t, f, 0x46), 0x0a0a_0000 + k);
    t.set_u32(op(t, f, 0x213), 0x0b0b_0000 + k);
    floats_at(
        t,
        f,
        k,
        &[
            0x24, 0x239, 0x261, 0x29c, 0x2d7, 0x2fb, 0x30e, 0x31e, 0x326, 0x339, 0x3da, 0x40a,
            0x445, 0x476, 0x489, 0x49c, 0x4a4, 0x4b4, 0x4bc, 0x7a7,
        ],
    );
    junk_ops(
        t,
        f,
        &[
            0x23e, 0x256, 0x291, 0x2c0, 0x2cc, 0x2f0, 0x303, 0x316, 0x32e, 0x341, 0x3e0, 0x3e9,
            0x3f3, 0x3ff, 0x410, 0x43a, 0x45f, 0x46b, 0x47e, 0x491, 0x4ac, 0x4c4, 0x4cc, 0x596,
            0x5aa, 0x699, 0x6a0, 0x700, 0x707, 0x759, 0x760, 0xe4, 0xf0, 0x7da,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x80, k);
    for (o, v) in [(0x18u32, state), (0x2c, n2c), (0x74, flag)] {
        t.set_u32(r + o, v);
    }
    t.set_u16(r + 0x70, ids);
    summon_task_at(t, r)
}

/// `magic_0106` 0x10006c30 (`r19::summon_task_0106`): every state, the
/// counts at their bounds, ids known and new, none, the flag's two ways,
/// states the byte table sends to the default and states past it.
#[test]
fn summon_task_0106() {
    let Some(mut t) = twin(106) else { return };
    let f = t.at(0x6c30);
    for (k, (st, more)) in [
        ((0u32, 5u32), (1u32, 0u16, 0u32)),
        ((10, 5), (1, 0, 0)),
        ((20, 5), (0, 0, 0)),
        ((21, 0x79), (0, 0, 0)),
        ((21, 0x7a), (0, 0, 0)),
        ((30, 5), (0, 0, 0)),
        ((31, 0x59), (0, 0, 0)),
        ((31, 0x5a), (0, 0, 0)),
        ((40, 5), (0, 0, 0)),
        ((41, 9), (0, 0, 5)),
        ((41, 0xa), (0, 3, 5)),
        ((41, 0xa), (0, 3, 0)),
        ((42, 1), (1, 2, 0)),
        ((42, 0x27), (1, 2, 0)),
        ((42, 0x28), (1, 3, 0)),
        ((42, 0x28), (0, 0, 0)),
        ((50, 5), (1, 2, 0)),
        ((51, 0xe), (1, 2, 0)),
        ((51, 0xf), (1, 3, 0)),
        ((51, 0xf), (0, 0, 0)),
        ((52, 0xe), (1, 2, 0)),
        ((52, 0xf), (1, 2, 0)),
        ((0x2710, 5), (0, 0, 0)),
        ((0x2710, 5), (1, 0, 0)),
        ((5, 5), (1, 0, 0)),
        ((0x34, 5), (0, 0, 0)),
        ((0x35, 5), (1, 0, 0)),
        ((0x2711, 5), (0, 0, 0)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0106_setup(&mut t, f, k, st, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [
        (878u32, 2u32),
        (541, 4),
        (296, 5),
        (845, 6),
        (288, 2),
        (239, 1),
    ] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// What `r19::three_nodes_0128` (`magic_0128`, at `f`) needs: the part
/// set-up, the object start, `n` scaled actors, slot 333's angle per
/// actor (the yaw above `HI`, inside and below `LO`), the records, the
/// free ring and the task list.
fn three_nodes_0128_setup(t: &mut Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let pi = t.img.rel32_target(f + 0x3c);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    magic_r11::object_slots(t);
    magic_r11::object_start_setup(t, t.img.rel32_target(f + 0xfa), (0x80, 0x9b));
    magic_r11::scaled_actors(t, f, n, (0x41, 0x6f, 0x87));
    let fl = |t: &Twin, o: u32| f64::from(f32::from_bits(t.u32(op(t, f, o))));
    let (k, hi, lo) = (fl(t, 0x18c), fl(t, 0x19b), fl(t, 0x1af));
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, a| match a[0] % 3 {
        0 => k - hi - 0.5,
        1 => k - f64::midpoint(hi, lo),
        _ => k - lo + 0.5,
    });
    t.write(op(t, f, 0x1f9), &[0x5c; 0x34 * 4]);
    magic_r11::nodes_setup(t, f + 0xe4, free, listed);
}

/// `magic_0128` 0x10001460 (`r19::three_nodes_0128`, internal) and its
/// load step 0x10001300 (`r19::load_0128`): no, one, two and three
/// actors (scales below, inside and above the clamp; yaws wrapped down,
/// kept and wrapped up), three and five free nodes, the task list empty
/// and not; the load step's waits and its start.
#[test]
fn three_nodes_0128() {
    let Some(mut t) = twin(128) else { return };
    let f = t.at(0x1460);
    for (n, free, listed) in [
        (0u32, 3u16, false),
        (1, 5, true),
        (3, 3, true),
        (2, 5, false),
    ] {
        three_nodes_0128_setup(&mut t, f, n, free, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r19::three_nodes_0128,
        );
    }
    let load = t.at(0x1300);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        three_nodes_0128_setup(&mut t, f, 2, 5, true);
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x27u32, 0x47] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x62, 0x6c]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// The matrix and vector slots the start-ups' turns use: 235 (an
/// identity varied by its call), 328/329 (one word plus the angle), 229
/// (`M[i] * 0.5 + v[i]`), 248 (`a + b`, folded into a dump row).
fn matrix_slots(t: &mut Twin) {
    rec(t, 235, 1, |m, a| {
        let k = m.r32(row(235)) as f32;
        for i in 0..16u32 {
            let v = if i % 5 == 0 {
                1.0
            } else {
                0.125 * (i as f32) - k * 0.25
            };
            wf(m, a[0] + 4 * i, v);
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let k = m.r32(row(s)) % 16;
            let v = rf(m, a[1] + 4 * k) + f32::from_bits(a[2]);
            wf(m, a[0] + 4 * k, v);
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
}

/// Where a copy of `r19::start_up_0285` keeps what its test sets: the
/// tail's shift, the object start's `rel32` and its pool call, the
/// selector and actor table, the words it stores (junk first).
struct Layout0285 {
    d: u32,
    start: (u32, u32),
    sel: u32,
    chrs: u32,
    emitters: u32,
    junk: &'static [u32],
}

const L0285: Layout0285 = Layout0285 {
    d: 0,
    start: (0x1a4, 0x9d),
    sel: 0x218,
    chrs: 0x2c4,
    emitters: 0x351,
    junk: &[
        0x4b, 0x1b1, 0x1be, 0x1cb, 0x1d8, 0x1f3, 0x203, 0x213, 0x220, 0x22e, 0x238, 0x242, 0x24c,
        0x26a, 0x274, 0x28a, 0x2b2, 0x2ba, 0x2fb, 0x30b, 0x329,
    ],
};

const L0287: Layout0285 = Layout0285 {
    d: 0xc,
    start: (0x1a2, 0x9c),
    sel: 0x227,
    chrs: 0x2b8,
    emitters: 0x352,
    junk: &[
        0x4b, 0x1af, 0x1bc, 0x1c9, 0x1d6, 0x202, 0x212, 0x222, 0x22f, 0x245, 0x24f, 0x259, 0x263,
        0x281, 0x28b, 0x2a9, 0x23f, 0x2a1, 0x2fc, 0x30c, 0x32a,
    ],
};

/// What `r19::start_up_0285` (`magic_0285`/`0286`/`0287`, at `f`, laid out
/// as `l`) needs: the part set-up, the slots (333's turn, 12's ground,
/// 314/404's random floats, 268's bone), the object start, the free ring
/// (eight nodes: three emitters, two task nodes and the live node take
/// from it), the task list (empty or not), the actors, the model's two
/// flags.
fn start_up_0285_setup(
    t: &mut Twin,
    f: u32,
    l: &Layout0285,
    k: u32,
    (flag_a, flag_b, listed): (u32, u32, bool),
) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let d = |o: u32| o + l.d;
    let pi = t.img.rel32_target(f + 0x50);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    magic_r11::object_slots(t);
    matrix_slots(t);
    for (s, n) in [(215u32, 1usize), (413, 1), (710, 4), (711, 3)] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 268, 3, move |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 0.375 * i as f32 - 1.5 + k as f32);
        }
        0x268
    });
    for (s, v) in [(333u32, 0.4f64), (12, 2.25), (314, 0.75), (404, 3.5)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            v + f64::from(m.r32(row(s)) % 5) * 1.75 - f64::from(a[0] & 3)
        });
    }
    let start = t.img.rel32_target(f + l.start.0);
    magic_r11::object_start_setup(t, start, (0x84, l.start.1));
    magic_r11::ring(
        t,
        op(t, f, d(0x5c8)),
        op(t, f, d(0x5dc)),
        8,
        SCRATCH + 0x8000,
    );
    t.write(SCRATCH + 0x600, &[0x44; 0x20]);
    t.set_u32(op(t, f, d(0x63e)), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, f, d(0x651)), SCRATCH + 0x600);
    let sel = SCRATCH + 0x180;
    t.set_u32(op(t, f, l.sel), sel);
    t.write(sel, &[3, 5, 7, 9]);
    for i in 0..0x10 {
        t.set_u32(op(t, f, l.chrs) + 4 * i, 0x00c0_0000 + i + k);
    }
    for o in [0x5bu32, 0xc2, d(0x54f)] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u32(op(t, f, d(0x549)), flag_a);
    t.set_u32(op(t, f, d(0x589)), flag_b);
    junk_ops(t, f, l.junk);
    // the emitters' records
    junk(t, op(t, f, l.emitters) - 0x250, 0x288 * 2, k);
    junk_ops(t, f, &[0x554, 0x594, 0x5a4, 0x5ae, 0x5b8, 0x5c1].map(d));
}

/// `magic_0285` 0x10004490, `magic_0286` 0x10001340 (`r19::start_up_0285`)
/// and `magic_0287` 0x10004460 (`r19::start_up_0287`), internal, and
/// their load steps (0x10004360, 0x10001210, 0x10004330): both model
/// slots, the flag's halfword both ways, the task list empty and not; the
/// load steps' waits and their start.
#[test]
fn start_up_0285() {
    type Port = fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> ffx_magic::Ret;
    for (dll, rva, load, l, port) in [
        (
            285u16,
            0x4490u32,
            0x4360u32,
            &L0285,
            ffx_magic::r19::start_up_0285 as Port,
        ),
        (286, 0x1340, 0x1210, &L0285, ffx_magic::r19::start_up_0285),
        (287, 0x4460, 0x4330, &L0287, ffx_magic::r19::start_up_0287),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (k, more) in [
            (0u32, 0u32, false),
            (1, 1, true),
            (0, 1, true),
            (1, 0, false),
        ]
        .into_iter()
        .enumerate()
        {
            start_up_0285_setup(&mut t, f, l, k as u32, more);
            scrub(&mut t);
            t.check(f, Call::default(), &[], Eax::Checked, port);
        }
        let load = t.at(load);
        for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
            start_up_0285_setup(&mut t, f, l, 3, (0, 1, true));
            rec(&mut t, 225, 1, move |_, _| busy);
            rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
            rec(&mut t, 227, 1, move |_, _| ready);
            let op = |t: &Twin, o: u32| t.img.word(load + o);
            let at = op(&t, 0x18);
            t.set_u32(at, loaded);
            for o in [0x26u32, 0x46] {
                let at = op(&t, o);
                t.set_u32(at, 0x0b0b_0000 + o);
            }
            junk_ops(&mut t, load, &[0x61, 0x6b]);
            scrub(&mut t);
            t.check_entry(load, Call::default(), &[], Eax::Checked);
        }
        assert!(t.u32(row(229)) >= 9, "slot 229: {} calls", t.u32(row(229)));
    }
}

/// What `r19::start_0082` (`magic_0082` 0x10006bf0) needs: the slots
/// (390 the kind, 534 the second answer), the summon list empty or not,
/// the camera block's list, the particle part start, the kind byte.
fn start_0082_setup(t: &mut Twin, f: u32, k: u32, (kind, second, filled): (u32, u32, bool)) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    rec(t, 390, 1, move |_, a| kind ^ (a[0] & 0x1_0000));
    rec(t, 534, 1, move |_, a| second ^ (a[0] & 0x1_0000));
    let list = op(t, f, 0x1b);
    empty_list(t, list);
    empty_list(t, op(t, f, 0x41d));
    if filled {
        let other = SCRATCH + 0x640;
        t.write(other, &[0x66; 0x20]);
        t.set_u32(list + 0x10, other);
    }
    // the camera block's chain: its next's `+4` takes the block
    let chain = SCRATCH + 0x680;
    t.write(chain, &[0x31; 0x20]);
    t.set_u32(op(t, f, 0x7f), chain);
    part_start(t, t.img.rel32_target(f + 0x43d));
    let owner = SCRATCH + 0x7b00;
    t.set_u32(op(t, f, 0x40c), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
    }
    for o in [0x9u32, 0x38] {
        t.set_u8(op(t, f, o), 0x21 + k as u8);
    }
    t.set_u32(op(t, f, 0x144), 0x0a0a_0000 + k);
    junk_ops(
        t,
        f,
        &[
            0x84, 0xb9, 0xc2, 0xd7, 0x24d, 0x282, 0x28b, 0x2a0, 0x481, 0x488, 0x4cd, 0x4d4,
        ],
    );
}

/// `magic_0082` 0x10006bf0 (`r19::start_0082`, internal) and the node
/// start 0x10007170 that runs it (`r9_summon::nodes_0082`): the kind in
/// the table's 0 and 1 rows and past it, slot 534's two answers, the
/// summon list empty and not.
#[test]
fn start_0082() {
    let Some(mut t) = twin(82) else { return };
    let f = t.at(0x6bf0);
    for (k, more) in [
        (0x1071u32, 0u32, false),
        (0x1072, 0, true),
        (0x1072, 1, false),
        (0x2000, 0, true),
        (0x2000, 1, true),
        (0x1081, 0, false),
        (0x108c, 1, false),
        (0x108d, 0, true),
    ]
    .into_iter()
    .enumerate()
    {
        start_0082_setup(&mut t, f, k as u32, more);
        scrub(&mut t);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r19::start_0082,
        );
    }
    assert!(t.u32(row(534)) >= 4, "slot 534: {} calls", t.u32(row(534)));
    // the node start: two actors, the lock word clear and set
    let at = t.at(0x7170);
    let lock = SCRATCH + 0x1f0;
    for (actor, lk) in [(3u32, 0u32), (0x21, 7)] {
        start_0082_setup(&mut t, f, actor, (0x1072, 1, actor == 3));
        for (slot, k) in [
            (718u32, 1usize),
            (710, 4),
            (222, 4),
            (221, 4),
            (551, 3),
            (168, 1),
            (531, 2),
        ] {
            rec(&mut t, slot, k, move |_, a| (0x0700_0000 + slot) ^ a[0]);
        }
        rec(&mut t, 711, 3, move |m, a| {
            m.w32(SCRATCH + 0x1f4, m.r32(lock));
            0x0711_0000 ^ a[0]
        });
        t.stub_st0(289, |_, _| -1.25);
        let lock_at = t.img.word(at + 0x2b2);
        t.set_u32(lock_at, lock);
        t.set_u32(lock, lk);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::default(), &[actor], Eax::Checked);
    }
}

/// What `r19::start_up_0249` (`magic_0249`, at `f`) needs: the pools and
/// the part set-up, the slots, the object start, the actors (`n`), the
/// task list (empty or not).
fn start_up_0249_setup(t: &mut Twin, f: u32, k: u32, (n, listed): (u32, bool)) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    magic_r11::pools(t, f, 1549);
    magic_r11::object_slots(t);
    for (s, nw) in [
        (215u32, 1usize),
        (413, 1),
        (855, 2),
        (357, 3),
        (356, 3),
        (362, 4),
        (6, 2),
        (548, 1),
    ] {
        rec(t, s, nw, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, a| {
        0.75 + f64::from(k) * 2.5 - f64::from(a[0] & 3)
    });
    magic_r11::object_start_auto(t, t.img.rel32_target(f + 0x10b));
    let la = t.img.rel32_target(f + 0x4d);
    for o in [0x34u32, 0x73] {
        t.set_u32(op(t, la, o), n);
    }
    t.write(op(t, la, 0x4a), &[3, 5, 7, 9]);
    for o in [0x87u32, 0x9a, 0x333, 0x398, 0x3d6, 0x3e3] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.write(SCRATCH + 0x600, &[0x44; 0x20]);
    for (first, last) in [(0x4dd, 0x4f4), (0x5c8, 0x5df)] {
        t.set_u32(op(t, f, first), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(op(t, f, last), SCRATCH + 0x600);
    }
    let ts = t.img.rel32_target(f + 0x41f);
    let ct = t.img.rel32_target(f + 0x462);
    for (fun, a, b) in [(ts, 0x75u32, 0x8cu32), (ct, 0x2a, 0x41)] {
        t.set_u32(op(t, fun, a), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(op(t, fun, b), SCRATCH + 0x600);
    }
    junk_ops(
        t,
        f,
        &[
            0x12, 0x1c, 0x26, 0x30, 0x3a, 0x44, 0x6b, 0x118, 0x125, 0x132, 0x147, 0x170, 0x182,
            0x18f, 0x19c, 0x1b1, 0x1dd, 0x221, 0x236, 0x243, 0x250, 0x271, 0x290, 0x2a2, 0x2b7,
            0x2c4, 0x2d1, 0x2f4, 0x311, 0x323, 0x33b, 0x348, 0x355, 0x36d, 0x385, 0x407, 0x416,
            0x425, 0x42f, 0x450, 0x4b2, 0x49e,
        ],
    );
}

/// `magic_0249` 0x100013e0 (`r19::start_up_0249`, internal) and its load
/// step 0x100012b0: no, one and two actors, the task list empty and not;
/// the load step's waits and its start.
#[test]
fn start_up_0249() {
    let Some(mut t) = twin(249) else { return };
    let f = t.at(0x13e0);
    for (k, more) in [(0u32, false), (1, true), (2, false), (2, true)]
        .into_iter()
        .enumerate()
    {
        start_up_0249_setup(&mut t, f, k as u32, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r19::start_up_0249,
        );
    }
    let load = t.at(0x12b0);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0249_setup(&mut t, f, 3, (2, true));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        scrub(&mut t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r19::start_up_0444` (`magic_0444`, at `f`) needs: the part
/// set-up, the slots (324 the deal's draws, 404 and 333 floats, 390 the
/// kinds), the hosted start's pool and lists, the actors (`n`; the map and
/// characters), the ring (eight nodes), the task list (empty or not), the
/// model's flags.
fn start_up_0444_setup(t: &mut Twin, f: u32, k: u32, (n, listed, flags): (u32, bool, u32)) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    magic_r11::pools(t, f, 1514);
    magic_r11::object_slots(t);
    for (s, nw) in [(215u32, 1usize), (413, 1), (16, 1), (364, 1)] {
        rec(t, s, nw, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 324, 0, move |m, _| m.r32(row(324)).wrapping_mul(7) ^ k);
    deal_kinds(t, t.img.rel32_target(f + 0x4e));
    for (s, v) in [
        (404u32, 0.3f64),
        (333, 0.9),
        (12, 1.75),
        (16, 1.25),
        (364, 0.5),
    ] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            v + f64::from(m.r32(row(s)) % 5) * 0.875 - f64::from(a[0] & 3) + f64::from(k)
        });
    }
    // slot 390: kinds by actor, one listed in the table
    let listed_kind = t.u32(op(t, f, 0x12f));
    rec(
        t,
        390,
        1,
        move |_, a| if a[0] & 1 == 0 { listed_kind } else { 0x5555 },
    );
    let host = t.img.rel32_target(f + 0x173);
    magic_r11::object_start_setup(t, host, (0x80, 0x9b));
    t.write(SCRATCH + 0x640, &[0x55; 0x20]);
    for o in [0xa6u32, 0xb9] {
        t.set_u32(op(t, host, o), if listed { SCRATCH + 0x640 } else { 0 });
    }
    t.set_u32(op(t, host, 0xc8), 5);
    // the actors: the map's bytes and the characters
    let map = SCRATCH + 0x180;
    t.set_u32(op(t, f, 0x117), map);
    t.write(map, &[2, 4, 6, 1, 3, 5]);
    let ap = t.img.rel32_target(f + 0x180);
    t.set_u32(op(t, ap, 0x26), map);
    for i in 0..0x10 {
        t.set_u32(op(t, ap, 0x31) + 4 * i, 0x00c0_0000 + i + k);
    }
    for o in [0x59u32, 0x110] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u32(op(t, ap, 0x1f), 0x0a0a_0110 + k);
    t.set_u32(op(t, f, 0xb6), n);
    t.set_u32(op(t, f, 0x240), flags & 1);
    t.set_u32(op(t, f, 0x27d), flags >> 1);
    magic_r11::ring(t, op(t, f, 0x2bc), op(t, f, 0x34e), 8, SCRATCH + 0x8000);
    t.write(SCRATCH + 0x600, &[0x44; 0x20]);
    t.set_u32(op(t, f, 0x3a9), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, f, 0x3c4), SCRATCH + 0x600);
    junk(t, op(t, f, 0xc1) - 0xd4, 0x130 * 3, k);
    junk(t, op(t, f, 0xc6) - 0x14, 0x34 * 3, k + 1);
    junk_ops(
        t,
        f,
        &[
            0x29, 0x33, 0x41, 0x246, 0x251, 0x257, 0x261, 0x288, 0x28e, 0x298, 0x2a2, 0x2ac, 0x2b5,
            0x2c2, 0x2cc, 0x312, 0x31c, 0x3e8, 0x3f2, 0x455, 0x45b, 0x514, 0x526,
        ],
    );
}

/// `magic_0444` 0x10002a10 (`r19::start_up_0444`, internal) and its load
/// step 0x100028e0: no, one and three objects (kinds listed and not, the
/// caster's byte among the dealt), the hosted list and the task list
/// empty and not, the model's flags; the load step's waits and its start.
#[test]
fn start_up_0444() {
    let Some(mut t) = twin(444) else { return };
    let f = t.at(0x2a10);
    for (k, more) in [
        (0u32, false, 0u32),
        (1, true, 3),
        (3, false, 1),
        (3, true, 2),
    ]
    .into_iter()
    .enumerate()
    {
        start_up_0444_setup(&mut t, f, k as u32, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r19::start_up_0444,
        );
    }
    let load = t.at(0x28e0);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0444_setup(&mut t, f, 2, (2, true, 1));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        scrub(&mut t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(390)) >= 5, "slot 390: {} calls", t.u32(row(390)));
}

/// What `r19::start_up_0413` (`magic_0413`, at `f`) needs: as
/// [`start_up_0444_setup`] with the kind check's actors and lists and the
/// object start's pool.
fn start_up_0413_setup(t: &mut Twin, f: u32, k: u32, (n, listed, flags): (u32, bool, u32)) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    magic_r11::pools(t, f, 1700);
    magic_r11::object_slots(t);
    for (s, nw) in [(215u32, 1usize), (413, 1), (534, 1)] {
        rec(t, s, nw, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 324, 0, move |m, _| m.r32(row(324)).wrapping_mul(7) ^ k);
    deal_kinds(t, t.img.rel32_target(f + 0x56));
    for (s, v) in [(404u32, 0.3f64), (333, 0.9), (12, 1.75)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            v + f64::from(m.r32(row(s)) % 5) * 0.875 - f64::from(a[0] & 3) + f64::from(k)
        });
    }
    // the kind check: the map, slot 390 a listed kind for every other actor
    let kc = t.img.rel32_target(f + 0x130);
    let map = SCRATCH + 0x180;
    t.set_u32(op(t, kc, 0x10), map);
    t.write(map, &[2, 4, 6, 1, 3, 5]);
    let first = t.u32(op(t, kc, 0x30));
    rec(
        t,
        390,
        1,
        move |_, a| if a[0] & 1 == 0 { first } else { 0x7777_7777 },
    );
    magic_r11::object_start_auto(t, t.img.rel32_target(f + 0x151));
    for o in [0x166u32, 0x1ed, 0x257] {
        t.set_u32(op(t, f, o), map);
    }
    for i in 0..0x10 {
        t.set_u32(op(t, f, 0x171) + 4 * i, 0x00c0_0000 + 2 * i + k);
    }
    for o in [0x61u32, 0x244] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u32(op(t, kc, 0x09), 0x0a0a_0061 + k);
    t.set_u32(op(t, f, 0xd4), n);
    t.set_u32(op(t, f, 0x2e6), flags & 1);
    t.set_u32(op(t, f, 0x323), flags >> 1);
    magic_r11::ring(t, op(t, f, 0x362), op(t, f, 0x41c), 8, SCRATCH + 0x8000);
    t.write(SCRATCH + 0x600, &[0x44; 0x20]);
    t.set_u32(op(t, f, 0x477), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, f, 0x492), SCRATCH + 0x600);
    junk(t, op(t, f, 0xdf) - 0xd4, 0x130 * 3, k);
    junk(t, op(t, f, 0xe4) - 0x14, 0x34 * 3, k + 1);
    junk_ops(
        t,
        f,
        &[
            0x27, 0x31, 0x3b, 0x49, 0xbd, 0xc7, 0x2ec, 0x2f7, 0x2fd, 0x307, 0x32e, 0x334, 0x33e,
            0x348, 0x352, 0x35b, 0x368, 0x372, 0x3e0, 0x3ea, 0x4b6, 0x4c0, 0x519, 0x51f,
        ],
    );
}

/// `magic_0413` 0x10002cb0 (`r19::start_up_0413`, internal) and its load
/// step 0x10002b80: no, one and three objects (kinds listed and not, the
/// caster among the dealt), the task list empty and not, the model's
/// flags; the load step's waits and its start.
#[test]
fn start_up_0413() {
    let Some(mut t) = twin(413) else { return };
    let f = t.at(0x2cb0);
    for (k, more) in [
        (0u32, false, 0u32),
        (1, true, 3),
        (3, false, 1),
        (3, true, 2),
    ]
    .into_iter()
    .enumerate()
    {
        start_up_0413_setup(&mut t, f, k as u32, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r19::start_up_0413,
        );
    }
    let load = t.at(0x2b80);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0413_setup(&mut t, f, 2, (2, true, 1));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        scrub(&mut t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(390)) >= 5, "slot 390: {} calls", t.u32(row(390)));
}

/// The deal at `deal` with three kinds of two each (the DLL's own data
/// has one kind, which clears the output: every dealt byte 0).
fn deal_kinds(t: &mut Twin, deal: u32) {
    let op = |t: &Twin, o: u32| t.img.word(deal + o);
    t.set_u32(op(t, 0x17), 3);
    for i in 0..3 {
        t.set_u32(op(t, 0x46) + 0x1c * i, 2);
    }
}
