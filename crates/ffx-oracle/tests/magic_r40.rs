//! Round 40's ports (`ffx_magic::r40`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

mod magic;
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::r40;
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::{ENTRY_ESP, rec};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

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

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// Every slot answering `first word ^ (slot << 20)` and logging its first
/// word (into `SCRATCH + 0xa00..`).
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

/// The stubs of `magic_0603`'s start-up and its copies (as
/// `magic_r25.rs`'s `start_up_0603`), slot 215 logging `n215` words.
fn start_up_stubs(t: &mut Twin, n215: usize) {
    all_stubs(t);
    for (s, x) in [
        (15u32, 0.5f64),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
        (289, -0.5),
        (333, -0.75),
    ] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    rec(t, 601, 2, |m, a| {
        wf(m, a[1], 1.75);
        0x601
    });
    for (s, n) in [(232u32, 2usize), (531, 2)] {
        rec(t, s, n, move |m, a| {
            for i in 0..4 {
                wf(m, a[0] + 4 * i, 0.5 * i as f32 - 1.25 + (s & 3) as f32);
            }
            s
        });
    }
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[2] + 4 * i);
            fold(m, 248, i, x);
        }
        0x248
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            fold(m, 234, i, x);
        }
        0x234
    });
    let ids = SCRATCH + 0x7c0;
    rec(t, 390, 1, move |m, _| m.r32(ids));
    rec(t, 535, 1, move |m, _| m.r32(ids + 4));
    let lock = SCRATCH + 0x7d0;
    rec(t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
    for (s, n) in [
        (803u32, 2usize),
        (217, 2),
        (215, n215),
        (262, 4),
        (718, 1),
        (710, 4),
        (222, 4),
        (221, 4),
        (551, 3),
        (10, 2),
        (325, 2),
        (684, 1),
        (294, 1),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `magic_0150` 0x1000d240 (`r40::start_up_0150`, `magic_0603`'s start-up
/// with slot 215 early and the code after it 3 bytes earlier): every
/// model id way (4203, 4209 with slot 535 answering 0, 2 and 1, 4232,
/// 4233, 4236 and another), the static lists empty and not, the bone
/// node stopping at once, slot 294's model number 0xff and not.
#[test]
fn start_up_0150() {
    let Some(mut t) = twin(150) else { return };
    start_up_stubs(&mut t, 1);
    let f = t.at(0xd240);
    // magic_0603's operand offset in this copy
    let m = |o: u32| if o >= 0x2cd { o - 3 } else { o };
    let op = |t: &Twin, o: u32| t.img.word(f + m(o));
    let ids = SCRATCH + 0x7c0;
    let lock = SCRATCH + 0x7d0;
    let list = op(&t, 0x1c0);
    let old = SCRATCH + 0x800;
    let bone = op(&t, 0x5b3);
    let pass = op(&t, 0x55d);
    // the model start's slot 294 (through the imports' `+0x498`) and the
    // set-up's ring and root list
    let ms = t.img.rel32_target(f + 0xb3d);
    let su = t.img.rel32_target(f + 0xb42);
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
        t.set_u8(pass, 0xff);
        t.set_u8(bone + 0x1a, 0x30 + k as u8);
        junk_ops(
            &mut t,
            f,
            &[
                0x5b,
                m(0x781),
                m(0x8c7),
                m(0x75a),
                m(0x97f),
                m(0x8fc),
                m(0xa56),
            ],
        );
        junk_ops(&mut t, f, &[0x227, 0x232]);
        t.set_u32(op(&t, 0x75a), 0x1234_5677 + k);
        t.set_u32(op(&t, 0x781), (2.5 + k as f32).to_bits());
        for o in [0x97fu32, 0x8fc] {
            t.set_u32(op(&t, o), (1.5 * k as f32).to_bits());
        }
        // the model start's ring tail and slot 294's first answer (0xff
        // on odd cases)
        let tail = t.img.word(ms + 0x04);
        t.set_u32(tail, SCRATCH + 0x900);
        t.write(SCRATCH + 0x900, &[0x3a; 0x20]);
        let tail = t.img.word(su + 0x04);
        t.set_u32(tail, SCRATCH + 0x940);
        t.write(SCRATCH + 0x940, &[0x3b; 0x20]);
        let a294 = t.img.word(ms + 0xba);
        t.set_u32(a294, if k % 2 == 1 { 0xff ^ (294 << 16) } else { 7 });
        ownb::dirty_stack(&mut t, 0xd240 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
}

/// `magic_0150` 0x1000a550 (`r40::walk_ring_junk_0150`) on the test rings
/// with the never-written frame word `[ebp - 4]` set to an owner on both
/// sides (the port's own entry passes 0), and its thunk 0x1000a5d0 on the
/// rings whose walk stops at once (the only way that never reads the
/// word).
#[test]
fn walk_ring_junk_0150() {
    let Some(mut t) = twin(150) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0xa550);
    let ring = SCRATCH + 0x5000;
    let owner = SCRATCH + 0x7000;
    for (k, &(c, p, cp)) in (0u32..).zip(sa::RINGS) {
        sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
        t.write(ring, &[0x41; 0x40]);
        t.set_u32(ring, sa::VTABLE);
        t.write(owner, &[0x52; 0x300]);
        t.set_u32(owner + 0x2ec, ring);
        // ebp = ENTRY_ESP - 4: [ebp - 4]
        t.set_u32(ENTRY_ESP - 8, owner);
        t.check(f, Call::this(sa::CURSOR), &[], Eax::Checked, |g, _, fr| {
            r40::walk_ring_junk_0150(g, fr, owner)
        });
    }
    let thunk = t.at(0xa5d0);
    for &(c, p, cp) in sa::RINGS.iter().filter(|r| r.1 == r.2) {
        sa::ring(&mut t, c, p, cp, 0x30);
        t.check_entry(
            thunk,
            Call::default(),
            &[sa::CURSOR, 0x0bad_f00e],
            Eax::Checked,
        );
    }
}

/// `magic_0070` 0x1000d000 (`r40::start_up_0070`, `magic_0570`'s start-up
/// with slot 381 first and slot 377 last; `magic_r27.rs`'s
/// `start_up_0597_kind`): no targets and two; the statics' root with and
/// without a child; the lock word and the rings' last words varied.
#[test]
fn start_up_0070() {
    let Some(mut t) = twin(70) else { return };
    all_stubs(&mut t);
    let f = t.at(0xd000);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let ti = t.img.rel32_target(f + 0xc);
    let statics = t.img.rel32_target(ti + 0xe0);
    let root = t.img.word(statics + 0x10d);
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    let list = SCRATCH + 0x6000;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(list + 0x7c));
        list
    });
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, a[0].wrapping_mul(0x0101_0101) ^ i);
        }
        0x531
    });
    rec(&mut t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x18b), lock);
    rec(&mut t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ a[1] ^ (a[2] << 8)
    });
    rec(&mut t, 710, 4, |_, a| a[2] ^ 0x710);
    for (s, n) in [
        (718u32, 1usize),
        (222, 4),
        (221, 4),
        (551, 3),
        (238, 2),
        (296, 2),
        (215, 1),
        (276, 2),
        (381, 1),
        (377, 4),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let child = SCRATCH + 0x6800;
    for (k, count) in [0u32, 2, 1].into_iter().enumerate() {
        let k = k as u32;
        t.write(list, &[0x80, 0x81, 0x83, 0x84]);
        t.set_u32(list + 0x7c, count);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, if k.is_multiple_of(2) { 0 } else { child });
        t.write(child, &[0x33; 0x40]);
        t.set_u32(child + 0x10, child);
        t.set_u32(lock, 0x40 + k);
        let (ids, steps, recs) = (w(&t, 0x1e4), w(&t, 0x1fd), w(&t, 0x1e9));
        for j in 0..0x57u32 {
            let id = t.img.word(ids + 1 - j).to_le_bytes()[0];
            t.set_u32(steps + 4 * u32::from(id), 0x5eed_0000 + j + k);
            let r = recs + 0x60 - 0x30 * j;
            t.write(r, &[0xa5 ^ (j as u8), 0x5a ^ (j as u8)]);
        }
        let (r1, r2) = (SCRATCH + 0x6900 + 0x20 * k, SCRATCH + 0x6a00 + 0x20 * k);
        t.write(r1, &[0x44; 0x10]);
        t.write(r2, &[0x55; 0x10]);
        if k > 0 {
            t.set_u32(w(&t, 0x2bd), r1);
            t.set_u32(w(&t, 0x2fc), r2);
        }
        junk_ops(
            &mut t,
            f,
            &[
                0x318, 0x31e, 0x369, 0x197, 0x302, 0x358, 0x2c8, 0x308, 0x3ae, 0x2c3, 0x2a2, 0x2b0,
            ],
        );
        ownb::dirty_stack(&mut t, 0xd000 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
}

/// `r40::motion_task` on one copy: every state from 5 to 0x40 and 10000,
/// 10001 (the waits answering 0 and not, the model flag set and not, the
/// model id 0 and not), the unlink, and the states the tables send to the
/// counts.
fn motion_task_case(dll: u16, at: u32) {
    let Some(mut t) = twin(dll) else { return };
    all_stubs(&mut t);
    let f = t.at(at);
    rec(&mut t, 20, 1, |m, a| {
        m.r32(SCRATCH + 0x10) ^ (a[0] & 0xff00)
    });
    rec(&mut t, 411, 1, |m, a| {
        m.r32(SCRATCH + 0x10) ^ (a[0] & 0xff00)
    });
    for (s, n) in [(30u32, 1usize), (36, 3), (228, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let task = SCRATCH + 0x100;
    let r = SCRATCH + 0x1000;
    let obj = SCRATCH + 0x2000;
    let (node, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let states = (5u32..=0x40).chain([10000, 10001, 0x8000_0005]);
    let mut k = 0u32;
    for state in states {
        for (wait, flag) in [(0u32, 0u16), (1, 1), (0x100, 0), (0, 1)] {
            k += 1;
            junk(&mut t, r, 0x40, k);
            junk(&mut t, obj, 0x100, k + 1);
            t.set_u32(r + 0x18, state);
            t.set_u32(r + 0x1c, obj);
            t.set_u16(obj + 0x90, flag);
            t.set_u32(SCRATCH + 0x10, wait);
            t.set_u32(task, node);
            t.set_u32(task + 4, r);
            t.write(node, &[0x3c; 0x20]);
            t.set_u32(node + 4, prev);
            t.set_u32(node + 8, next);
            t.write(prev, &[0x4d; 0x20]);
            t.write(next, &[0x5e; 0x20]);
            ownb::dirty_stack(&mut t, at + k, &[]);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// `magic_0155` 0x100092e0 (`r40::motion_task_0155`).
#[test]
fn motion_task_0155() {
    motion_task_case(155, 0x92e0);
}

/// `magic_0164` 0x10009340 (`r40::motion_task_0164`).
#[test]
fn motion_task_0164() {
    motion_task_case(164, 0x9340);
}

/// `magic_0609` 0x100093d0 (`r27::motion_task_0609`, the same shape) under
/// [`motion_task_case`]'s cases.
#[test]
fn motion_task_0609_all_states() {
    motion_task_case(609, 0x93d0);
}
