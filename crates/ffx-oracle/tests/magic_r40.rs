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

use ffx_magic::{Ret, r40};
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
/// word). Both are unreachable in the game (no object carries the
/// `EgoTailSpr` vtable 0x100115ac; `r40::walk_ring_junk_0150`).
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

const GROUP: u32 = SCRATCH + 0x1000;
const OWNER: u32 = SCRATCH + 0x2000;
const PARENT: u32 = SCRATCH + 0x2400;
const MODEL: u32 = SCRATCH + 0x5000;
const SRC: u32 = SCRATCH + 0x5800;
const PLOG: u32 = SCRATCH + 0x8000;
/// The DLL's pointer globals' targets.
const PTRS: u32 = SCRATCH + 0x9000;

/// Each slot logs its first `n` words (and what a stack pointer argument
/// points at) in its own row and answers `ret` (`magic_sb_ppp.rs`'s
/// stubs); slot 712 says done when the word before the part says so.
fn ppp_stubs(t: &mut Twin, lock: u32) {
    for (k, (s, n, ret)) in [
        (325u32, 2usize, 0x325u32),
        (230, 2, 0x230),
        (233, 3, 0x233),
        (668, 3, 0x668),
        (231, 3, 0x231),
        (713, 1, 0x713),
        (160, 3, 0x160),
    ]
    .into_iter()
    .enumerate()
    {
        let row = PLOG + 0x80 * k as u32;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x7c).wrapping_add(1);
            m.w32(row + 0x7c, c);
            let at = row + 0x40 * (c & 1);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(at + 4 * i, *w);
                if *w >= magic::STACK_TOP - magic::STACK_LEN && *w < magic::STACK_TOP {
                    for j in 0..4 {
                        m.w32(at + 0x10 + 4 * j, m.r32(*w + 4 * j));
                    }
                }
            }
            ret
        });
    }
    t.stub(712, move |m, a| {
        m.w32(PLOG + 0x400, a[0]);
        m.w32(PLOG + 0x404, a[1]);
        m.w32(PLOG + 0x408, m.r32(lock));
        m.w32(PTRS + 0x20, m.r32(PTRS + 0x20) ^ 0x0712_0000);
        m.r32(a[0] - 4)
    });
}

/// `magic_0203`'s node for [`run_part_0203`].
#[allow(clippy::too_many_arguments)]
fn node_0203(t: &mut Twin, flags: u32, delay: u32, f4: u16, done: u32, func: u32, k: u32) {
    t.write(PLOG, &[0; 0x480]);
    junk(t, OWNER, 0x120, k);
    t.set_u32(GROUP, OWNER);
    t.set_u8(GROUP + 4, 0);
    t.set_u8(GROUP + 5, 3);
    t.set_u8(OWNER + 0x1a, 9);
    t.set_u32(OWNER + 0x10, OWNER);
    t.set_u32(OWNER + 0x14, PARENT);
    t.set_u32(OWNER + 4, PARENT);
    t.set_u32(OWNER + 8, PARENT);
    t.write(PARENT, &[0x47; 0x20]);
    t.set_u32(PARENT + 0xc, OWNER);
    t.set_u32(PARENT + 0x10, OWNER);
    t.set_u16(PARENT + 0x18, 1);
    for i in 0..4 {
        t.set_u32(OWNER + 0x1c + 4 * i, (1.5f32 + i as f32).to_bits());
        t.set_u32(
            OWNER + 0x4c + 4 * i,
            (200.0f32 - 30.0 * i as f32 + k as f32).to_bits(),
        );
        t.set_u32(SRC + 0x1c + 4 * i, (0.25f32 + 0.3 * i as f32).to_bits());
    }
    t.set_u32(OWNER + 0x58, (-7.5e9f32).to_bits());
    t.set_u32(OWNER + 0x5c, MODEL);
    t.set_u32(OWNER + 0x60, 0x6060 + k);
    t.set_u32(OWNER + 0x6c, done);
    t.set_u32(OWNER + 0x108, SRC);
    t.set_u32(OWNER + 0xf0, func);
    t.set_u16(OWNER + 0xf4, f4);
    t.set_u32(OWNER + 0xf8, flags);
    t.set_u32(OWNER + 0xfc, delay);
    t.set_u32(OWNER + 0x104, 0x0104_0000 + k);
    t.set_u32(MODEL + 0x138, 0.5f32.to_bits());
}

/// `magic_0203` 0x10004630 (`r40::jmp_run_part_0203` over
/// `run_part_0203` 0x100042a0): the walk stopping at once and running;
/// flags 0, 1, 2, 5, 0x11; the delay positive (paused and not), 0 and
/// negative; the countdown at 0 and not; pppRunPart done and not; the
/// user function set and not.
#[test]
fn run_part_0203() {
    let Some(mut t) = twin(203) else { return };
    let thunk = t.at(0x4630);
    let f = t.at(0x42a0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let pause = PTRS;
    let lock = PTRS + 0x10;
    for (op, v) in [
        (0x75u32, pause),
        (0x134, PTRS + 0x100),
        (0x178, PTRS + 0x200),
        (0x185, PTRS + 0x600),
        (0x296, PTRS + 0x20),
        (0x2a2, PTRS + 0x30),
        (0x2ac, lock),
        (0x25e, PTRS + 0x300),
        (0x269, PTRS + 0x400),
        (0x275, PTRS + 0x500),
        (0x27d, PTRS + 0x700),
        (0x288, PTRS + 0x780),
        (0x301, PTRS + 0x40),
    ] {
        let g = w(&t, op);
        t.set_u32(g, v);
    }
    t.set_u32(w(&t, 0x29b), 0x29b_29b);
    t.set_u32(PTRS + 0x40, 0x0301_0301);
    ppp_stubs(&mut t, lock);
    let mut k = 0u32;
    for (flags, delay) in [
        (0u32, 0u32),
        (1, 0),
        (2, 0),
        (3, 5),
        (0, 1),
        (1, 0x8000_0000),
        (0x11, 0),
        (5, 0),
    ] {
        for paused in [0u32, 1] {
            for f4 in [0u16, 3] {
                for done in [0u32, 1] {
                    for func in [0u32, 0x00ab_cdef] {
                        k += 1;
                        node_0203(&mut t, flags, delay, f4, done, func, k);
                        if k.is_multiple_of(7) {
                            // the walk stops at once
                            t.set_u8(OWNER + 0x1a, 3);
                        }
                        t.set_u32(pause, paused);
                        t.set_u32(lock, 0x42);
                        let c = Call {
                            eax: 0x77,
                            ..Call::default()
                        };
                        t.check_entry(thunk, c, &[GROUP, 0x0bad_f00e], Eax::Checked);
                    }
                }
            }
        }
    }
}

/// `magic_0186` 0x10003f40 (`r40::jmp_path_run_0186` over
/// `path_run_0186` 0x10003ce0): the test rings (stopping at once and
/// walking), every kind (0..4 and past the table; kinds 3 and 4 with the
/// model's bit 0 set and not), the distance below, at and past the
/// target's `x` (NaN once).
#[test]
fn path_run_0186() {
    let Some(mut t) = twin(186) else { return };
    sa::handler_stubs(&mut t);
    let thunk = t.at(0x3f40);
    let model = SCRATCH + 0x6000;
    let target = SCRATCH + 0x7c00;
    // the target point every getter writes (P.x read as the limit)
    let put = move |m: &mut dyn StubMem, at: u32| {
        for i in 0..4 {
            let x = m.r32(target + 4 * i);
            m.w32(at + 4 * i, x);
        }
    };
    rec(&mut t, 262, 4, move |m, a| {
        for i in 0..3 {
            let x = m.r32(target + 4 * i);
            m.w32(a[1 + i as usize], x);
        }
        0x262
    });
    rec(&mut t, 29, 2, move |m, a| {
        put(m, a[1]);
        0x29
    });
    rec(&mut t, 268, 3, move |m, a| {
        put(m, a[2]);
        0x268
    });
    for s in [366u32, 665] {
        rec(&mut t, s, 2, move |m, a| {
            put(m, a[1]);
            s
        });
    }
    // 234: a = b - c; 668: a = b * c; 248: a = b + c (all four lanes)
    for (s, op) in [(234u32, 0u8), (668, 1), (248, 2)] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..4 {
                let b = f32::from_bits(m.r32(a[1] + 4 * i));
                let c = f32::from_bits(m.r32(a[2] + 4 * i));
                let x = match op {
                    0 => b - c,
                    1 => b * c,
                    _ => b + c,
                };
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            s
        });
    }
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    let mut k = 0u32;
    for &(c, p, cp) in sa::RINGS {
        for kind in 0u8..6 {
            for (bit, lim) in [(0u8, 1.0f32), (1, 40.0), (0, 2.5), (1, f32::NAN)] {
                k += 1;
                sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
                let n = sa::node(0);
                t.write(model, &[0x33; 0xa0]);
                t.set_u32(model, sa::VTABLE);
                t.set_u8(model + 0x90, bit);
                t.set_u32(model + 0x50, 0x0050_0000 + k);
                t.set_u32(model + 0x54, 0x0054_0000 + k);
                for (i, x) in [3.0f32, -1.5, 0.75, 1.0].into_iter().enumerate() {
                    t.set_u32(n + 0x1c + 4 * i as u32, (x + k as f32).to_bits());
                    t.set_u32(n + 0x2c + 4 * i as u32, (x * 2.0).to_bits());
                }
                t.set_u32(n + 0x48, 2.0f32.to_bits());
                t.set_u32(n + 0x4c, 0.125f32.to_bits());
                t.set_u32(n + 0x50, 0.25f32.to_bits());
                t.set_u32(n + 0x54, model);
                t.set_u32(n + 0x58, 0x58 + k);
                t.set_u8(n + 0x5c, kind);
                t.set_u8(n + 0x5d, 0);
                for (i, x) in [lim, 4.0 + k as f32, -2.0, 1.0].into_iter().enumerate() {
                    t.set_u32(target + 4 * i as u32, x.to_bits());
                }
                t.check_entry(
                    thunk,
                    Call::default(),
                    &[sa::CURSOR, 0x0bad_f00e],
                    Eax::Checked,
                );
            }
        }
    }
}

/// The actors' positions slot 217 gives in `magic_0179`'s tests (four
/// floats each, by the actor word's low two bits).
const POS_0179: u32 = SCRATCH + 0x7a00;

/// The stubs of `magic_0179`'s tests: every slot logs its first word
/// ([`all_stubs`]); slot 217 `(actor, out)` writes the actor's position,
/// 234 `a = b - c`, 229 `a = b + c` (four lanes), 307 a "random" float in
/// ST0, 240 and 666 their answers from `SCRATCH + 0x7f40..`, 941 a program
/// table, 714 (pppStartPart) logs its words and the lock.
fn stubs_0179(t: &mut Twin) {
    all_stubs(t);
    rec(t, 217, 2, |m, a| {
        let at = POS_0179 + 0x10 * (a[0] & 3);
        for i in 0..4 {
            let x = m.r32(at + 4 * i);
            m.w32(a[1] + 4 * i, x);
        }
        0x217
    });
    for (s, op) in [(234u32, 0u8), (229, 1), (248, 1)] {
        rec(t, s, 3, move |m, a| {
            for i in 0..4 {
                let b = f32::from_bits(m.r32(a[1] + 4 * i));
                let c = f32::from_bits(m.r32(a[2] + 4 * i));
                let x = if op == 0 { b - c } else { b + c };
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            s
        });
    }
    // 235 sets the matrix (the frame's words are each side's own junk)
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + 0x1111 * i);
        }
        0x235
    });
    for (s, n) in [(330u32, 3usize), (329, 3)] {
        rec(t, s, n, move |m, a| {
            let x = m.r32(a[0]) ^ a.get(2).copied().unwrap_or(0);
            m.w32(a[0], x.rotate_left(3));
            s
        });
    }
    rec(t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x7f30).wrapping_add(1);
        m.w32(SCRATCH + 0x7f30, k);
        f64::from(k % 7) * 0.375 - 0.5
    });
    rec(t, 240, 1, |m, _| m.r32(SCRATCH + 0x7f40));
    rec(t, 666, 2, |_, a| a[1] ^ 0x666);
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, _| 0x714);
    rec(t, 324, 0, |m, _| m.r32(SCRATCH + 0x7f40) ^ 0x3240);
    for (s, n) in [(325u32, 2usize), (696, 2), (213, 2), (239, 3), (388, 3)] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0x3240)
        });
    }
}

/// The positions of four actors (`k` varies them; `quad` puts actor 1
/// around actor 0 in each quadrant and on the axes).
fn positions_0179(t: &mut Twin, k: u32, quad: u32) {
    let (dx, dz) = [
        (3.0f32, 2.0f32),
        (-3.0, 2.0),
        (-3.0, -2.0),
        (3.0, -2.0),
        (0.0, 2.0),
        (0.0, -2.0),
        (2.0, 0.0),
        (-2.0, 0.0),
        (0.0, 0.0),
    ][quad as usize % 9];
    let base = [1.5f32 + k as f32, -0.25, 4.0, 1.0];
    for i in 0..4u32 {
        t.set_u32(POS_0179 + 4 * i, base[i as usize].to_bits());
    }
    let other = [
        base[0] + dx,
        base[1] + 1.0 + k as f32 * 0.5,
        base[2] + dz,
        1.0,
    ];
    for j in 1..4u32 {
        for i in 0..4u32 {
            let x = other[i as usize] + (j - 1) as f32 * 0.125;
            t.set_u32(POS_0179 + 0x10 * j + 4 * i, x.to_bits());
        }
    }
}

/// The particle node's globals in `magic_0179` (its pointer global and
/// the start-part wrapper's lock).
fn particle_globals_0179(t: &mut Twin) {
    let pn = t.at(0x34d0);
    let p = t.img.word(pn + 0x45);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    t.set_u32(p, SCRATCH + 0x7f10);
}

/// `magic_0179`'s static lists empty (each list's first and last child
/// itself, no children): the operands of the step at 0x100069c0 and the
/// task at 0x10006cc0 that name them.
fn lists_0179(t: &mut Twin) {
    let step = t.at(0x69c0);
    let task = t.at(0x6cc0);
    for (f, o) in [(step, 0xacu32), (step, 0xe4), (step, 0x11f), (task, 0x5d)] {
        let l = t.img.word(f + o);
        t.set_u32(l + 0xc, l);
        t.set_u32(l + 0x10, l);
        // a count with both bytes set (the halfword reads)
        t.set_u16(l + 0x18, 0x0305);
    }
    // the parts' owner block the particle nodes read
    // the ring's flag word (bit 0 cleared) junk
    let flag = t.img.word(t.at(0x6b60) + 0x45);
    t.set_u32(flag, 0x5a5a_5a5b);
    // the ring's last (the object start's operand)
    let ring = t.img.word(t.at(0x6b60) + 0x29);
    t.set_u32(ring, SCRATCH + 0x7d00);
    t.write(SCRATCH + 0x7d00, &[0x2d; 0x20]);
    let owner = t.img.word(task + 0x4a);
    t.set_u32(owner, SCRATCH + 0x7e00);
    t.write(SCRATCH + 0x7e00, &[0x2e; 0x40]);
}

/// `magic_0179` 0x10006540 (`r40::yaw_0179`): the second actor in every
/// quadrant, on each axis and on the first.
#[test]
fn yaw_0179() {
    let Some(mut t) = twin(179) else { return };
    stubs_0179(&mut t);
    let f = t.at(0x6540);
    for quad in 0..9 {
        positions_0179(&mut t, quad, quad);
        let c = Call {
            ecx: 0,
            edx: 1,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Junk, |g, img, fr| {
            let y = r40::yaw_0179(g, img, fr.at, fr.esp, fr.ecx, fr.edx);
            Ret::xmm0(ffx_magic::Xmm::f32(y), 0)
        });
    }
}

/// `magic_0179` 0x10006020 (`r40::throw_setup_0179`): targets in every
/// direction, near and far, above and below; the first speed too low
/// (several tries), none enough (the loop runs out), both roots
/// positive; `high` 0 and 1; the parent's list empty and not.
#[test]
fn throw_setup_0179() {
    let Some(mut t) = twin(179) else { return };
    stubs_0179(&mut t);
    let f = t.at(0x6020);
    let rec = SCRATCH + 0x6000;
    let (node, parent, kid, target) = (
        SCRATCH + 0x6100,
        SCRATCH + 0x6300,
        SCRATCH + 0x6500,
        SCRATCH + 0x6600,
    );
    let mut k = 0u32;
    for quad in 0..9u32 {
        for (dist, dy, v) in [
            (1.0f32, 0.5f32, 15.0f32),
            (8.0, -3.0, 2.0),
            (40.0, 10.0, 0.5),
            (3.0, 1.0, 9.0),
        ] {
            for high in [0u32, 1] {
                k += 1;
                junk(&mut t, node, 0x40, k);
                junk(&mut t, parent, 0x40, k + 1);
                junk(&mut t, kid, 0x20, k + 2);
                t.set_u32(
                    parent + 0x10,
                    if k.is_multiple_of(2) { parent } else { kid },
                );
                for (i, x) in [2.0f32, 0.5, -1.0].into_iter().enumerate() {
                    t.set_u32(parent + 0x1c + 4 * i as u32, x.to_bits());
                }
                let (sx, sz) = [
                    (1.0f32, 1.0f32),
                    (-1.0, 1.0),
                    (-1.0, -1.0),
                    (1.0, -1.0),
                    (0.0, 1.0),
                    (0.0, -1.0),
                    (1.0, 0.0),
                    (-1.0, 0.0),
                    (0.0, 0.0),
                ][quad as usize];
                for (i, x) in [2.0 + sx * dist, 0.5 + dy, -1.0 + sz * dist * 0.5]
                    .into_iter()
                    .enumerate()
                {
                    t.set_u32(target + 4 * i as u32, x.to_bits());
                }
                for (i, x) in [node, parent, target, v.to_bits(), 0.05f32.to_bits(), high]
                    .into_iter()
                    .enumerate()
                {
                    t.set_u32(rec + 4 * i as u32, x);
                }
                t.check(f, Call::this(rec), &[], Eax::Checked, |g, img, fr| {
                    Ret::eax(r40::throw_setup_0179(g, img, fr.at, fr.esp, fr.ecx))
                });
            }
        }
    }
}

/// `magic_0179` 0x10006cc0 (`r40::throw_task_0179`, with
/// `object_start_0179` and the set-up): the timer below and past the
/// mark, every object index (kind 1 from the seventh), the unlink after
/// the eighth.
#[test]
fn throw_task_0179() {
    let Some(mut t) = twin(179) else { return };
    stubs_0179(&mut t);
    particle_globals_0179(&mut t);
    lists_0179(&mut t);
    let f = t.at(0x6cc0);
    let task = SCRATCH + 0x100;
    let r = SCRATCH + 0x1_0000;
    let (node, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    positions_0179(&mut t, 0, 0);
    for (k, (i, timer)) in (0u32..).zip([
        (0u16, 0.5f32),
        (0, 3.0),
        (3, 3.0),
        (6, 3.0),
        (7, 9.0),
        (8, 9.0),
    ]) {
        junk(&mut t, r, 0x3000, k);
        // the objects' static nodes at real points (the throws aim from
        // them)
        for o in 0..8u32 {
            let sn = r + 0x9f0 + 0x480 * o + 0x48;
            for (j, x) in [0.5f32, -0.25, 1.5, 1.0].into_iter().enumerate() {
                t.set_u32(sn + 0x1c + 4 * j as u32, (x + o as f32).to_bits());
            }
        }
        t.set_u32(r + 0x18, timer.to_bits());
        t.set_u32(r + 0x1c, 2.0f32.to_bits());
        t.set_u16(r + 0x20, i);
        for (j, x) in [1.0f32, 0.0, 3.0, 1.0].into_iter().enumerate() {
            t.set_u32(r + 0x9d4 + 4 * j as u32, (x + k as f32).to_bits());
        }
        let parent = r + 0x84c;
        t.set_u32(parent + 0x10, parent);
        t.set_u16(parent + 0x18, 0);
        for (j, x) in [0.0f32, 0.0, 0.0].into_iter().enumerate() {
            t.set_u32(parent + 0x1c + 4 * j as u32, x.to_bits());
        }
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.write(node, &[0x3c; 0x20]);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x4d; 0x20]);
        t.write(next, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x6cc0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0179` 0x100069c0 (`r40::object_step_0179`): the timer below
/// the first mark, between the marks, past the end; the kind set and not
/// (slot 324's bit 0 and 1); state 1; other states.
#[test]
fn object_step_0179() {
    let Some(mut t) = twin(179) else { return };
    stubs_0179(&mut t);
    particle_globals_0179(&mut t);
    let f = t.at(0x69c0);
    let task = SCRATCH + 0x100;
    let o = SCRATCH + 0x1_0000;
    let (node, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let mut k = 0u32;
    for state in [0u16, 1, 2, 0xffff] {
        for timer in [0.0f32, 0.5, 2.0, 50.0, 1000.0] {
            for kind in [0u16, 1] {
                k += 1;
                lists_0179(&mut t);
                junk(&mut t, o, 0x400, k);
                t.set_u32(o + 0x18, timer.to_bits());
                t.set_u16(o + 0x1c, state);
                t.set_u16(o + 0x1e, kind);
                t.set_u32(SCRATCH + 0x7f40, k);
                junk_ops(&mut t, f, &[0x73, 0x106]);
                t.set_u32(task, node);
                t.set_u32(task + 4, o);
                t.write(node, &[0x3c; 0x20]);
                t.set_u32(node + 4, prev);
                t.set_u32(node + 8, next);
                t.write(prev, &[0x4d; 0x20]);
                t.write(next, &[0x5e; 0x20]);
                ownb::dirty_stack(&mut t, 0x69c0 + k, &[]);
                t.check_entry(f, Call::default(), &[task], Eax::Checked);
            }
        }
    }
}

/// `magic_0179` 0x10007120 (`r40::summon_task_0179`, with
/// `throw_start_0179`): every state (and past the table), the timers
/// below and past their marks, slot 240 answering 0 and 1.
#[test]
fn summon_task_0179() {
    let Some(mut t) = twin(179) else { return };
    stubs_0179(&mut t);
    let f = t.at(0x7120);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let task = SCRATCH + 0x100;
    let (node, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let start = t.img.rel32_target(f + 0x76);
    let ring = t.img.word(start + 0x15);
    let mut k = 0u32;
    for state in [0u16, 1, 2, 3, 4, 5] {
        for timer in [0.0f32, 1.0e4] {
            for done in [0u32, 1] {
                k += 1;
                junk_ops(&mut t, f, &[0x26, 0x8c, 0x115, 0x131, 0x3f]);
                junk_ops(
                    &mut t,
                    start,
                    &[0x58, 0x5d, 0x62, 0xc8, 0x246, 0x23f, 0x25f, 0x26f],
                );
                for o in [0x25fu32, 0x26f] {
                    t.set_u32(t.img.word(start + o), (1.5f32 + k as f32).to_bits());
                }
                t.set_u16(w(&t, 0x6), state);
                t.set_u32(w(&t, 0x50), timer.to_bits());
                t.set_u32(SCRATCH + 0x7f40, done);
                // the ring's last for the throw start, the static list
                t.write(SCRATCH + 0x3000, &[0x61; 0x20]);
                t.set_u32(ring, SCRATCH + 0x3000);
                lists_0179(&mut t);
                let sl = t.img.word(start + 0xac);
                t.set_u32(
                    t.img.word(start + 0x67),
                    if k.is_multiple_of(2) {
                        sl
                    } else {
                        SCRATCH + 0x3100
                    },
                );
                t.set_u32(task, node);
                t.write(node, &[0x3c; 0x20]);
                t.set_u32(node + 4, prev);
                t.set_u32(node + 8, next);
                t.write(prev, &[0x4d; 0x20]);
                t.write(next, &[0x5e; 0x20]);
                ownb::dirty_stack(&mut t, 0x7120 + k, &[]);
                t.check_entry(f, Call::default(), &[task], Eax::Checked);
            }
        }
    }
}

/// `magic_0179` 0x100072a0 (`r40::start_up_0179`): the static lists
/// empty and not, the caster and the target in two directions.
#[test]
fn start_up_0179() {
    let Some(mut t) = twin(179) else { return };
    stubs_0179(&mut t);
    let f = t.at(0x72a0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
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
    let lock = SCRATCH + 0x7f50;
    t.set_u32(w(&t, 0x748), lock);
    for (k, quad) in [0u32, 3, 5].into_iter().enumerate() {
        let k = k as u32;
        positions_0179(&mut t, k, quad);
        t.write(list, &[0x80, 0x81, 0x83, 0x84]);
        t.set_u32(list + 0x7c, k % 2 + 1);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(w(&t, 0x1f7), k);
        t.set_u8(w(&t, 0x60f), 1 + k as u8);
        // the lists' last words: the list itself or another node
        for (o, cmp) in [(0x28bu32, 0x2c6u32), (0x484, 0x4bf), (0x66c, 0x6b4)] {
            let l = w(&t, o);
            let v = if k == 1 {
                SCRATCH + 0x6800 + o
            } else {
                w(&t, cmp)
            };
            t.set_u32(l, v);
        }
        t.write(SCRATCH + 0x6a00, &[0x22; 0x400]);
        t.set_u32(w(&t, 0x7d4), SCRATCH + 0x6c00);
        ownb::dirty_stack(&mut t, 0x72a0 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
}

/// The stubs of `magic_0183`'s tests: every slot logs its first word; 218
/// a target list, 217 a position, 216 a height (ST0), 233 `a = b + c` (a
/// float third word added to four lanes), 229 `a = M[0..4] / 2 + b`, 244 a
/// matrix by its angle, 225/227 the load answers.
fn stubs_0183(t: &mut Twin) {
    all_stubs(t);
    let list = SCRATCH + 0x7e80;
    t.write(list, &[3, 1, 2, 0]);
    rec(t, 218, 2, move |_, _| list);
    rec(t, 217, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 + (a[0] & 0xf) as f32);
        }
        0x217
    });
    rec(t, 216, 1, |_, a| 0x216 ^ a[0]);
    t.stub_st0(216, |_, a| 1.25 + f64::from(a[0] & 7));
    rec(t, 233, 3, |m, a| {
        let k = f32::from_bits(a[2]);
        for i in 0..4 {
            let b = f32::from_bits(m.r32(a[1] + 4 * i));
            let x = if b.is_finite() { b * 0.5 + k } else { k };
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) * 0.5 + f32::from_bits(m.r32(a[2] + 4 * i));
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 244, 2, |m, a| {
        for i in 0..16 {
            wf(
                m,
                a[0] + 4 * i,
                0.25 * i as f32 + (a[1] & 0xff) as f32 * 0.01,
            );
        }
        0x244
    });
    rec(t, 225, 1, |m, _| m.r32(SCRATCH + 0x7f60));
    rec(t, 227, 1, |m, _| m.r32(SCRATCH + 0x7f64));
    rec(t, 212, 2, |_, a| 0x0212_0000 ^ a[0]);
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, _| 0x714);
    // 711 sees the game lock
    rec(t, 711, 3, |m, a| m.r32(SCRATCH + 0x7f00) ^ a[0]);
}

/// `magic_0183` 0x10001090 (`r40::setup_0183`).
#[test]
fn setup_0183() {
    let Some(mut t) = twin(183) else { return };
    stubs_0183(&mut t);
    let f = t.at(0x1090);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(f + 0x155), lock);
    let start = t.img.rel32_target(f + 0x1d4);
    t.set_u32(t.img.word(start + 0x04), lock);
    for k in 0..3u32 {
        t.set_u32(lock, 0x40 + k);
        junk_ops(&mut t, f, &[0xe5, 0xeb, 0x113, 0x216, 0x21c, 0x222]);
        t.set_u32(t.img.word(f + 0x1a6), SCRATCH + 0x7f20);
        t.set_u32(t.img.word(f + 0x1d9), SCRATCH + 0x7f20);
        ownb::dirty_stack(&mut t, 0x1090 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
}

/// `magic_0183` 0x10001350 (`r40::camera_task_0183`): the load steps
/// (busy, asked, not loaded), every kind, the frames 0, 1, 0x1d, 0x1e,
/// 0x31, 0x32, 0x45, 0x46 and past, the height above and below the top,
/// the camera id set and not.
#[test]
fn camera_task_0183() {
    let Some(mut t) = twin(183) else { return };
    stubs_0183(&mut t);
    let f = t.at(0x1350);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let mut k = 0u32;
    let mut case =
        |t: &mut Twin, kind: u32, flags: [u32; 3], answers: [u32; 2], top: f32, id: u32| {
            k += 1;
            // junk first: the gates' flags share their globals with the
            // counts (+0x12/+0x39, +0x3f/+0x5b, +0x6a/+0x92)
            junk_ops(
                t,
                f,
                &[0x39, 0x92, 0x5b, 0x427, 0x48d, 0x47b, 0x493, 0x3e5, 0x417],
            );
            t.set_u32(w(t, 0x12), flags[0]);
            t.set_u32(w(t, 0x3f), flags[1]);
            t.set_u32(w(t, 0x6a), flags[2]);
            t.set_u32(SCRATCH + 0x7f60, answers[0]);
            t.set_u32(SCRATCH + 0x7f64, answers[1]);
            t.set_u32(w(t, 0x97), kind);
            t.set_u32(w(t, 0x2ff), top.to_bits());
            t.set_u32(w(t, 0x407), id);
            for o in [
                0x249u32, 0x259, 0x261, 0x269, 0x276, 0x2a1, 0x2ba, 0x2cb, 0x14d, 0x18b, 0x21f,
                0x367, 0x37f, 0x39d, 0x3bb,
            ] {
                t.set_u32(
                    w(t, o),
                    (0.25 * (o % 13) as f32 - 1.0 + k as f32 * 0.125).to_bits(),
                );
            }
            // the angle halfword with junk above it
            t.set_u32(w(t, 0x118), 0x5a5a_fff0 + k);
            ownb::dirty_stack(t, 0x1350 + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        };
    case(&mut t, 0xbf, [0, 0, 0], [1, 1], 9.0, 0);
    case(&mut t, 0xbf, [0, 0, 0], [0, 0], 9.0, 0);
    case(&mut t, 0xbf, [0, 1, 0], [0, 2], 9.0, 0);
    for kind in [0x47u32, 0x60, 0x83, 0x50] {
        case(&mut t, kind, [1, 1, 1], [0, 1], 9.0, 0);
    }
    for n in [
        0u32,
        1,
        0x1d,
        0x1e,
        0x31,
        0x32,
        0x45,
        0x46,
        0x47,
        0xffff_fff0,
    ] {
        for (top, id) in [(100.0f32, u32::MAX), (-100.0, 5), (-100.0, u32::MAX)] {
            case(&mut t, 0xbf + n, [1, 1, 1], [0, 1], top, id);
        }
    }
}

/// `magic_0183` 0x10001800 (`r40::draw_0183`): the frames before 0xa1,
/// 0xa1, between, 0xbf (the distance), 0xc0, the last with the cross
/// (0x118) and past it, 0x183; the two parts' halfwords set and not,
/// `pppRunPart` done and not.
#[test]
fn draw_0183() {
    let Some(mut t) = twin(183) else { return };
    stubs_0183(&mut t);
    let f = t.at(0x1800);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(
                m,
                a[3] + 4 * i,
                0.75 * i as f32 + (a[1] & 0xff) as f32 * 0.01,
            );
        }
        0x237
    });
    for s in [236u32, 243, 231] {
        let at = if s == 236 { 2usize } else { 0 };
        rec(&mut t, s, at + 1, move |m, a| {
            for i in 0..16u32 {
                wf(m, a[at] + 4 * i, 0.125 * i as f32 - (s & 7) as f32);
            }
            s
        });
    }
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(&mut t, 266, 3, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * ((i + 1) % 3)))
                * f32::from_bits(m.r32(a[2] + 4 * ((i + 2) % 3)));
            wf(m, a[0] + 4 * i, x);
        }
        0x266
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    // pppRunPart: the done answer, and the lock it sees logged
    rec(&mut t, 712, 2, |m, _| {
        let l = m.r32(SCRATCH + 0x7f00);
        let n = m.r32(SCRATCH + 0x7f6c);
        m.w32(SCRATCH + 0x7f6c, n.rotate_left(3) ^ l);
        m.r32(SCRATCH + 0x7f68)
    });
    let mat = SCRATCH + 0x7a00;
    let holder = SCRATCH + 0x7a80;
    let lock = SCRATCH + 0x7f00;
    for o in [0x4f8u32, 0x6b8] {
        t.set_u32(w(&t, o), mat);
    }
    for o in [0x496u32, 0x4c1, 0x64c, 0x7b6] {
        t.set_u32(w(&t, o), holder);
    }
    t.set_u32(w(&t, 0x659), lock);
    let mut k = 0u32;
    let mut case = |t: &mut Twin, kind: u32, halves: [u16; 2], done: u32| {
        k += 1;
        // junk first: the frame count's increment (+0x843) names the
        // frame's own global, and the halfwords sit under the counts
        // (+0x68a, +0x7f4)
        junk_ops(
            t,
            f,
            &[0x1ff, 0x151, 0x45e, 0x470, 0x491, 0x68a, 0x7f4, 0x843],
        );
        t.set_u32(w(t, 0x1b), kind);
        t.set_u32(w(t, 0x4d1), 0x5a5a_0000 | u32::from(halves[0]));
        t.set_u32(w(t, 0x691), 0xa5a5_0000 | u32::from(halves[1]));
        t.set_u32(SCRATCH + 0x7f68, done);
        t.set_u32(lock, 0x40 + k);
        t.write(mat, &[0x3c; 0x40]);
        for o in [0x1c7u32, 0x1cf, 0x1d7, 0x1df, 0x530, 0x55b, 0x6f0, 0x71b] {
            t.set_u32(
                w(t, o),
                (0.5 * (o % 11) as f32 + k as f32 * 0.25 - 2.0).to_bits(),
            );
        }
        ownb::dirty_stack(t, 0x1800 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    };
    case(&mut t, 0xa0, [1, 1], 1);
    case(&mut t, 0xa1, [1, 1], 1);
    case(&mut t, 0xb0, [1, 0], 0);
    case(&mut t, 0xbf, [1, 1], 1);
    case(&mut t, 0xbf, [0, 1], 0);
    case(&mut t, 0xc0, [1, 1], 0);
    case(&mut t, 0xbf + 0x59, [1, 1], 1);
    case(&mut t, 0xbf + 0x5a, [1, 0], 1);
    case(&mut t, 0x183, [0, 0], 1);
    case(&mut t, 0x183, [1, 1], 1);
}

/// The stubs of `magic_0185`'s tests ([`stubs_0183`]'s, the same family)
/// and its sprite slots: 241 a "random" float to its pointer, 247 a
/// matrix by its angle, 772 a handle (negative on odd calls), 773 the
/// ready answer, 776/777 two words.
fn stubs_0185(t: &mut Twin) {
    stubs_0183(t);
    rec(t, 241, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x7f70).wrapping_add(1);
        m.w32(SCRATCH + 0x7f70, k);
        wf(m, a[0], (k % 5) as f32 * 0.375 - 0.5);
        0x241
    });
    rec(t, 247, 2, |m, a| {
        for i in 0..16 {
            wf(
                m,
                a[0] + 4 * i,
                0.125 * i as f32 + (a[1] & 0xff) as f32 * 0.01,
            );
        }
        0x247
    });
    rec(t, 772, 2, |m, a| {
        let k = m.r32(SCRATCH + 0x7f74).wrapping_add(1);
        m.w32(SCRATCH + 0x7f74, k);
        if k & 1 == 0 { a[0] } else { 0x8000_0000 | a[0] }
    });
    rec(t, 773, 2, |m, _| m.r32(SCRATCH + 0x7f78));
    rec(t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    // 777 reads the sprite's matrix
    rec(t, 777, 1, |m, a| {
        let mut x = 0x0777_0000 ^ a[0];
        for i in 0..16 {
            x = x.rotate_left(3) ^ m.r32(a[0] + 4 * i);
        }
        x
    });
    // 875/873 log the sprite record they get (its 0x20 bytes) and the
    // lock (where the stand-in table makes them traps); 775 the lock
    for s in [875u32, 873] {
        rec(t, s, 4, move |m, a| {
            let l = m.r32(SCRATCH + 0x7f00);
            let mut x = l;
            for i in 0..8 {
                x = x.rotate_left(5) ^ m.r32(a[3] + 4 * i);
            }
            let n = m.r32(SCRATCH + 0x7f90);
            m.w32(SCRATCH + 0x7f90, n.rotate_left(7) ^ x);
            (s << 16) ^ a[0]
        });
    }
    rec(t, 775, 2, |m, a| m.r32(SCRATCH + 0x7f00) ^ a[0]);
    // 248 logs the vector it adds
    rec(t, 248, 3, |m, a| {
        let mut x = 0u32;
        for i in 0..4 {
            x = x.rotate_left(5) ^ m.r32(a[2] + 4 * i);
        }
        m.w32(SCRATCH + 0x7f94, m.r32(SCRATCH + 0x7f94).rotate_left(3) ^ x);
        0x248
    });
}

/// `magic_0185`'s spark records (operands of the sprite loop at
/// 0x100015a0): every other one free, the frames and timers varied.
fn records_0185(t: &mut Twin, k: u32) {
    let sp = t.at(0x15a0);
    let first = t.img.word(sp + 0x1d);
    let end = t.img.word(sp + 0x243);
    let mut i = 0u32;
    let mut at = first;
    while at < end {
        let rec = at - 0x16;
        for j in 0..4u32 {
            t.set_u32(rec + 4 * j, (0.5 * j as f32 + i as f32).to_bits());
        }
        t.set_u32(rec + 0x10, (1.0 + 0.25 * i as f32).to_bits());
        t.set_u16(
            rec + 0x14,
            [0x1f0u16, 0x100, 0x3000, 0xffff][((i + k) % 4) as usize],
        );
        // free, frame 0, 1, or the last one (the record freed after it)
        let last = t.u32(t.img.word(sp + 0x22e)) as u16;
        t.set_u16(
            at,
            match (i + k) % 4 {
                3 => 0xffff,
                2 => last.wrapping_sub(1),
                n => n as u16,
            },
        );
        i += 1;
        at += 0x18;
    }
}

/// `magic_0185` 0x10001a00 (`r40::spark_task_0185`, with
/// `spark_burst_0185`): the load steps, every kind, the frames inside
/// and outside the bursts, the records full and not.
#[test]
fn spark_task_0185() {
    let Some(mut t) = twin(185) else { return };
    stubs_0185(&mut t);
    let f = t.at(0x1a00);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let burst = t.img.rel32_target(f + 0x102);
    let take = t.img.rel32_target(burst + 0x92);
    let first = t.img.word(take + 0x3);
    let end = t.img.word(take + 0x12);
    let mut k = 0u32;
    let mut case = |t: &mut Twin, kind: u32, flags: [u32; 3], answers: [u32; 2], free: u32| {
        k += 1;
        // junk first: the gates' flags share their globals with the counts
        junk_ops(t, f, &[0x39, 0x8e, 0x5b]);
        t.set_u32(w(t, 0x12), flags[0]);
        t.set_u32(w(t, 0x3f), flags[1]);
        t.set_u32(w(t, 0x6a), flags[2]);
        t.set_u32(SCRATCH + 0x7f60, answers[0]);
        t.set_u32(SCRATCH + 0x7f64, answers[1]);
        t.set_u32(w(t, 0x93), kind);
        // `free` free records, the rest used
        let mut at = first;
        let mut n = 0;
        while at < end {
            t.write(at - 0x16, &[0x5c; 0x18]);
            t.set_u16(at, if n < free { 0xffff } else { 1 });
            n += 1;
            at += 0x18;
        }
        for o in [0x71u32, 0x86, 0xbb, 0x137, 0x148, 0x16f, 0x1ac] {
            t.set_u32(
                t.img.word(burst + o),
                (0.5 + (o % 7) as f32 * 0.25).to_bits(),
            );
        }
        t.set_u32(t.img.word(burst + 0x11), 0x1000 + k);
        ownb::dirty_stack(t, 0x1a00 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    };
    case(&mut t, 0x30, [0, 0, 0], [1, 1], 9);
    case(&mut t, 0x30, [0, 0, 0], [0, 0], 9);
    case(&mut t, 0x30, [0, 1, 0], [0, 2], 9);
    for (kind, free) in [
        (0x24u32, 9u32),
        (0x8f, 2),
        (0xa1, 0),
        (0x27, 9),
        (0x28, 9),
        (0x4a, 3),
        (0x4b, 9),
    ] {
        case(&mut t, kind, [1, 1, 1], [0, 1], free);
    }
}

/// `magic_0185` 0x100015a0 (`r40::sprites_0185`, internal; its `eax` is
/// junk): the records free and used, ready and not, the timers below and
/// past the frame's length, the last frame.
#[test]
fn sprites_0185() {
    let Some(mut t) = twin(185) else { return };
    stubs_0185(&mut t);
    let f = t.at(0x15a0);
    let lock = SCRATCH + 0x7f00;
    for o in [0xfcu32, 0x12b, 0x15e, 0x17b, 0x1bd, 0x1ee] {
        t.set_u32(t.img.word(f + o), lock);
    }
    t.set_u32(t.img.word(f + 0xdd), SCRATCH + 0x7f80);
    for k in 0..4u32 {
        records_0185(&mut t, k);
        t.set_u32(SCRATCH + 0x7f78, k & 1);
        t.set_u32(lock, 0x40 + k);
        ownb::dirty_stack(&mut t, 0x15a0 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Junk, |g, img, fr| {
            r40::sprites_0185(g, img, fr.at, fr.esp);
            Ret::eax(0)
        });
    }
    assert!(t.u32(SCRATCH + 0x7f74) != 0, "no handle asked");
}

/// `magic_0185` 0x10001090 (`r40::setup_0185`): no targets and three.
#[test]
fn setup_0185() {
    let Some(mut t) = twin(185) else { return };
    stubs_0185(&mut t);
    let f = t.at(0x1090);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 246, 1, |_, a| 0x246 ^ a[0]);
    t.stub_st0(246, |_, a| 3.5 + f64::from(a[0] & 3));
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + (a[0] & 7) as f32);
        }
        0x210
    });
    let lock = SCRATCH + 0x7f00;
    t.set_u32(w(&t, 0x345), lock);
    let start = t.img.rel32_target(f + 0x3c4);
    t.set_u32(t.img.word(start + 0x04), lock);
    for (k, n) in [(0u32, 0u32), (1, 3)] {
        t.set_u32(lock, 0x40 + k);
        junk_ops(&mut t, f, &[0x90, 0x5e, 0x28, 0x43a, 0x440, 0x446, 0x337]);
        t.set_u32(w(&t, 0x9b), n);
        for o in [0x396u32, 0x3c9, 0x3fd] {
            t.set_u32(w(&t, o), SCRATCH + 0x7f20);
        }
        for o in [0x114u32, 0x129, 0x1ec, 0x1fc, 0x20c] {
            let at = w(&t, o);
            for j in 0..2u32 {
                t.set_u32(at + 4 * j, (1.5 + j as f32 + k as f32).to_bits());
            }
        }
        let recs = w(&t, 0xa4) - 8;
        junk(&mut t, recs, 3 * 0x88 + 0x48, k);
        ownb::dirty_stack(&mut t, 0x1090 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
}

/// `magic_0185` 0x10001b20 (`r40::frame_0185`, with `sprites_0185`): not
/// loaded; frame 0; the sprite frames; every `n2` way (0, below 10,
/// 10..0x13, 0x14, between, 0x32, past) and below `n1`; the three parts'
/// halfwords set and not, `pppRunPart` done and not; slot 240's answers;
/// frame 0xde.
#[test]
fn frame_0185() {
    let Some(mut t) = twin(185) else { return };
    stubs_0185(&mut t);
    let f = t.at(0x1b20);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let lock = SCRATCH + 0x7f00;
    rec(&mut t, 712, 2, |m, _| {
        let l = m.r32(SCRATCH + 0x7f00);
        let n = m.r32(SCRATCH + 0x7f6c);
        m.w32(SCRATCH + 0x7f6c, n.rotate_left(3) ^ l);
        m.r32(SCRATCH + 0x7f68)
    });
    rec(&mut t, 240, 1, |m, a| m.r32(SCRATCH + 0x7f40) ^ (a[0] & 1));
    let mat = SCRATCH + 0x7a00;
    let holder = SCRATCH + 0x7a80;
    for o in [
        0x156u32, 0x189, 0x1b4, 0x1d7, 0x29f, 0x2d2, 0x2fd, 0x320, 0x3e8, 0x41b, 0x446, 0x469,
    ] {
        t.set_u32(w(&t, o), mat);
    }
    for o in [0x78u32, 0x115, 0x229, 0x372, 0x4bb] {
        t.set_u32(w(&t, o), holder);
    }
    for o in [0x236u32, 0x37f, 0x4c8] {
        t.set_u32(w(&t, o), lock);
    }
    // the sprites' globals (all records free)
    let sp = t.img.rel32_target(f + 0xaf);
    for o in [0xfcu32, 0x12b, 0x15e, 0x17b, 0x1bd, 0x1ee] {
        t.set_u32(t.img.word(sp + o), lock);
    }
    t.set_u32(t.img.word(sp + 0xdd), SCRATCH + 0x7f80);
    let (first, end) = (t.img.word(sp + 0x1d), t.img.word(sp + 0x243));
    let mut at = first;
    while at < end {
        t.set_u16(at, 0xffff);
        at += 0x18;
    }
    // a real sine table (the game's pointer is to a zeroed page here)
    let sine = SCRATCH + 0x4_0000;
    let tbl: Vec<u8> = (0..0x1000u32)
        .flat_map(|i| {
            ((i as f32) * std::f32::consts::TAU / 4096.0)
                .sin()
                .to_le_bytes()
        })
        .collect();
    t.write(sine, &tbl);
    t.set_u32(w(&t, 0x62d), sine);
    // the targets: three records with their actors and points
    let recs = w(&t, 0x517);
    for i in 0..3u32 {
        let r = recs + 0x88 * i;
        t.write(r, &[0x40 + i as u8; 0x88]);
        t.set_u32(r, 0x00ac_0010 + i);
        t.set_u32(r + 4, 0x00ac_0020 + i);
        t.set_u32(r + 0x7c - 0x40, (2.5 + i as f32).to_bits());
    }
    let mut k = 0u32;
    let mut case = |t: &mut Twin, frame: u32, loaded: u32, halves: [u16; 3], done: u32| {
        k += 1;
        junk_ops(t, f, &[0x7f6, 0xe6, 0xfe, 0x110, 0x267, 0x3b0, 0x4f9]);
        t.set_u32(w(t, 0x6), loaded);
        t.set_u32(w(t, 0x13), frame);
        t.set_u32(w(t, 0x125), 0x5a5a_0000 | u32::from(halves[0]));
        t.set_u32(w(t, 0x26e), 0xa5a5_0000 | u32::from(halves[1]));
        t.set_u32(w(t, 0x3b7), 0x3c3c_0000 | u32::from(halves[2]));
        t.set_u32(w(t, 0x50c), 3);
        t.set_u32(SCRATCH + 0x7f68, done);
        t.set_u32(SCRATCH + 0x7f40, k & 1);
        t.set_u32(lock, 0x40 + k);
        t.write(mat, &[0x3c; 0x40]);
        for o in [0x184u32, 0x1af, 0x2cd, 0x2f8, 0x416, 0x441] {
            t.set_u32(w(t, o), (0.5 * (o % 11) as f32 + 1.25).to_bits());
        }
        ownb::dirty_stack(t, 0x1b20 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    };
    case(&mut t, 0x80, 0, [1, 1, 1], 1);
    case(&mut t, 0, 1, [1, 1, 1], 1);
    for frame in [0x27u32, 0x28, 0x60, 0x65, 0x6a, 0x6b] {
        case(&mut t, frame, 1, [1, 1, 1], 0);
    }
    for n2 in [0u32, 3, 9, 10, 0x13, 0x14, 0x15, 0x31, 0x32, 0x33] {
        for (halves, done) in [([1u16, 1, 1], 1u32), ([0, 1, 0], 0), ([1, 0, 1], 1)] {
            case(&mut t, 0x6f + n2, 1, halves, done);
        }
    }
    case(&mut t, 0xde, 1, [0, 0, 0], 0);
}

/// `magic_0178` 0x100066e0 (`r40::group_task_0178`): the timer below and
/// past its mark; state 0 waiting and not, 1, 2 (no count, the byte
/// counting down and reaching 0), others; the static list empty and not.
#[test]
fn group_task_0178() {
    let Some(mut t) = twin(178) else { return };
    all_stubs(&mut t);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x7c00);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x7c00 + 4 * i, 0x9000 + i);
    }
    rec(&mut t, 714, 5, |_, _| 0x714);
    rec(&mut t, 779, 2, |m, a| m.r32(SCRATCH + 0x7f00) ^ a[0]);
    rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[0]);
    let f = t.at(0x66e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the particle node's globals and the unlink's lock
    let pn = t.at(0x3590);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let ul = t.at(0x3e30);
    for o in [0x45u32, 0x65] {
        t.set_u32(t.img.word(ul + o), lock);
    }
    let owner = SCRATCH + 0x7e00;
    t.write(owner, &[0x2e; 0x40]);
    t.set_u32(w(&t, 0x1cf), owner);
    let task = SCRATCH + 0x100;
    let r = SCRATCH + 0x1_0000;
    let (node, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let statics = w(&t, 0xdf);
    let mut k = 0u32;
    for (state, timer, wait, count, byte) in [
        (0u16, 0.0f32, 0u16, 0u16, 0u8),
        (0, 0.0, 1, 0, 0),
        (1, 0.0, 0, 0, 0),
        (2, 0.0, 0, 0, 0x30),
        (2, 0.0, 0, 3, 0x30),
        (2, 0.0, 0, 3, 0x10),
        (5, 0.0, 0, 0, 0),
        (1, 1.0e9, 0, 3, 0x10),
    ] {
        k += 1;
        junk(&mut t, r, 0x800, k);
        t.set_u32(r + 0x7e4, timer.to_bits());
        t.set_u16(r + 0x538, state);
        t.set_u16(r + 0x524, wait);
        t.set_u16(r + 0x520, count);
        t.set_u8(r + 0x2e3, byte);
        // the node the state-2 end unlinks: a child of a parent
        let child = SCRATCH + 0x3000;
        let parent = SCRATCH + 0x3100;
        t.write(child, &[0x31; 0x40]);
        t.write(parent, &[0x32; 0x40]);
        t.set_u32(r + 0x2bc, child);
        t.set_u32(child + 0x14, parent);
        t.set_u32(child + 4, parent);
        t.set_u32(child + 8, parent);
        t.set_u32(parent + 0xc, child);
        t.set_u32(parent + 0x10, child);
        t.set_u16(parent + 0x18, 1);
        // the static list empty or with a last node
        t.write(statics, &[0; 0x20]);
        let last = if k.is_multiple_of(2) {
            statics
        } else {
            SCRATCH + 0x3200
        };
        t.set_u32(statics + 0x10, last);
        t.set_u32(statics + 0xc, statics);
        t.write(SCRATCH + 0x3200, &[0x33; 0x20]);
        // the other lists the nodes join: empty
        for o in [0x1ebu32, 0x244] {
            let l = w(&t, o);
            t.write(l, &[0; 0x20]);
            t.set_u32(l + 0xc, l);
            t.set_u32(l + 0x10, l);
        }
        junk_ops(&mut t, f, &[0x288, 0x297, 0x169, 0x181, 0x191, 0x291]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.write(node, &[0x3c; 0x20]);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x4d; 0x20]);
        t.write(next, &[0x5e; 0x20]);
        t.set_u32(lock, 0x40 + k);
        ownb::dirty_stack(&mut t, 0x66e0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0184` 0x10001030 (`r40::sprites_0184`, `sprites_0185`'s loop
/// on 0184's layout): the records free and used, ready and not, the
/// timers below and past the frame's length, the last frame; slot 772's
/// `edx` as the handle's high word.
#[test]
fn sprites_0184() {
    let Some(mut t) = twin(184) else { return };
    stubs_0185(&mut t);
    t.stub_edx(772, |m, _| {
        m.r32(SCRATCH + 0x7f74).wrapping_mul(0x0101_0101)
    });
    let f = t.at(0x1030);
    let lock = SCRATCH + 0x7f00;
    for o in [0xe0u32, 0x10e, 0x141, 0x15e, 0x1a0, 0x1d1] {
        t.set_u32(t.img.word(f + o), lock);
    }
    t.set_u32(t.img.word(f + 0xc1), SCRATCH + 0x7f80);
    let (first, end) = (t.img.word(f + 0x1d), t.img.word(f + 0x226));
    let last = t.u32(t.img.word(f + 0x211)) as u16;
    for k in 0..4u32 {
        let mut i = 0u32;
        let mut at = first;
        while at < end {
            let rec = at - 0x16;
            for j in 0..4u32 {
                t.set_u32(rec + 4 * j, (0.5 * j as f32 + i as f32).to_bits());
            }
            t.set_u32(rec + 0x10, (1.0 + 0.25 * i as f32).to_bits());
            t.set_u16(
                rec + 0x14,
                [0x1f0u16, 0x100, 0x3000, 0xffff][((i + k) % 4) as usize],
            );
            t.set_u16(
                at,
                match (i + k) % 4 {
                    3 => 0xffff,
                    2 => last.wrapping_sub(1),
                    n => n as u16,
                },
            );
            i += 1;
            at += 0x18;
        }
        t.set_u32(SCRATCH + 0x7f78, k & 1);
        t.set_u32(lock, 0x40 + k);
        ownb::dirty_stack(&mut t, 0x1030 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Junk, |g, img, fr| {
            r40::sprites_0184(g, img, fr.at, fr.esp);
            Ret::eax(0)
        });
    }
    assert!(t.u32(SCRATCH + 0x7f74) != 0, "no handle asked");
}
