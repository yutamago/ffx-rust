//! Round 45's own ports (`ffx_magic::r45`) against each original. Needs
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
#[path = "magic/sa.rs"]
mod sa;
#[path = "magic/summon_stubs.rs"]
mod summon_stubs;

use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};
use sa::rec;

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn setf(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// `n` floats at `at`, varied by `k` (small, never NaN).
fn floats(t: &mut Twin, at: u32, n: u32, k: u32) {
    for i in 0..n {
        let v = ((i * 37 + k * 11) % 41) as f32 * 0.125 - 2.5;
        setf(t, at + 4 * i, v);
    }
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

/// The entry's stack pointer (at the return address).
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// `magic_0394`'s slots: 262 a position from its first word, 12 and 333
/// floats in ST0, the matrix slots writing what they read.
fn stubs_0394(t: &mut Twin) {
    all_stubs(t);
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xf) as f32 * 0.75 + i as f32 - 3.0);
        }
        0x262
    });
    rec(t, 12, 1, |_, a| 0x12 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 7) * 0.5 + 0.25);
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [328u32, 329, 330] {
        rec(t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * (i % 3) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * rf(m, a[2] + 4 * i) + 0.25;
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
    for (s, n) in [
        (255u32, 2usize),
        (33, 4),
        (32, 3),
        (11, 1),
        (238, 2),
        (541, 3),
        (38, 3),
        (719, 3),
        (239, 3),
        (295, 2),
        (531, 2),
        (306, 6),
        (210, 2),
        (217, 2),
        (211, 2),
    ] {
        rec(t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
}

/// The state block `S = *G_S` of `magic_0394` at `SCRATCH + 0x8000`.
fn block_0394(t: &mut Twin, g: u32, k: u32) -> u32 {
    let s = SCRATCH + 0x8000;
    junk(t, s, 0xd00, k);
    floats(t, s + 0xaf0, 0x20, k);
    floats(t, s + 0xb40, 0x10, k + 3);
    t.set_u32(g, s);
    s
}

/// `magic_0394` 0x10005640 (`r45::summon_start_0394`): the battle flag 1
/// (task list empty and not), 0 and 2; the yaws wrapping up, down and not
/// (`S + 0xc84` and slot 333's float).
#[test]
fn summon_start_0394() {
    let Some(mut t) = sa::twin(394) else { return };
    let f = t.at(0x5640);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    stubs_0394(&mut t);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let sel = SCRATCH + 0x180;
    for o in [0x13b, 0x168, 0x1db, 0x333] {
        t.set_u32(w(o), sel);
    }
    for i in 0..0x30 {
        t.set_u32(w(0x172) + 4 * i, 0x0c00 + 3 * i);
        t.set_u32(w(0x1eb) + 4 * i, 0x0d00 + 5 * i);
    }
    let spin = SCRATCH + 0x7e20;
    t.stub_st0(333, move |m, a| {
        let c = m.r32(spin);
        f64::from(a[0] & 0xff) * 0.5 - 3.0 + f64::from(c) * 4.0
    });
    let (first, last) = (w(0x357), w(0x36d));
    // (battle flag, list empty, the yaw, slot 333's shift)
    let cases: [(u32, bool, f32, u32); 6] = [
        (1, true, 0.5, 0),
        (1, false, 3.0, 1),
        (1, false, -4.0, 3),
        (0, true, -0.5, 0),
        (2, false, 2.9, 2),
        (0, false, 1.0, 1),
    ];
    for (k, (on, empty, yaw, shift)) in (0u32..).zip(cases) {
        let s = block_0394(&mut t, w(0x1d), k);
        setf(&mut t, s + 0xc84, yaw);
        t.set_u32(spin, shift);
        t.set_u32(w(0x12f), on);
        t.write(sel, &[3 + k as u8, 0x5a]);
        let tail = SCRATCH + 0x1800;
        junk(&mut t, tail, 0x20, k);
        t.set_u32(first, if empty { 0 } else { tail });
        t.set_u32(last, tail);
        t.set_u32(w(0x124), 0x0a00 + k);
        magic_r11::pools(&mut t, f, 1159);
        ownb::dirty_stack(&mut t, 0x5640 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0348` 0x10006940 (`r45::object_start_0348`): the task list
/// empty and holding a node, the record's flag halfword junk.
#[test]
fn object_start_0348() {
    let Some(mut t) = sa::twin(348) else { return };
    let f = t.at(0x6940);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let rn = t.img.rel32_target(f + 0x84);
    let head = t.img.word(rn + 0x37);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0xc), s);
    for k in 0..3u32 {
        junk(&mut t, s, 0xcf00, k);
        floats(&mut t, s + 0xd10, 0x8, k);
        floats(&mut t, s + 0xd20, 0x4, k + 2);
        let tail = SCRATCH + 0x1800;
        junk(&mut t, tail, 0x20, k);
        t.set_u32(head, if k == 1 { 0 } else { tail });
        t.set_u32(t.img.word(rn + 0x4f), tail);
        magic_r11::pools(&mut t, f, 193);
        ownb::dirty_stack(&mut t, 0x6940 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0392` 0x100014a0 (`r45::actors_pass_0392`, internal): the
/// actor's two calls skipped by the flag, by an actor past 31 and made;
/// the table with gaps and its last word set or not.
#[test]
fn actors_pass_0392() {
    let Some(mut t) = sa::twin(392) else { return };
    let f = t.at(0x14a0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    for (s, k) in [(217u32, 0x0217u32), (255, 0x0255)] {
        rec(&mut t, s, 2, move |m, a| {
            let v = m.r32(a[1]);
            m.w32(a[1], v ^ k);
            m.w32(a[1] + 8, a[0] ^ k);
            k
        });
    }
    rec(&mut t, 211, 2, |_, a| 0x0211_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ (a[1] << 8));
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    let reg = t.img.rel32_target(f + 0x78);
    let flag = SCRATCH + 0x600;
    rec(&mut t, 779, 4, move |m, a| m.r32(flag) ^ a[3]);
    t.set_u32(t.img.word(reg + 0x21), flag);
    t.set_u32(w(0x1f), 0x00ab_001f);
    // (flag, actor, the table's last word)
    for (k, (off, actor, last)) in (0u32..).zip([
        (0u32, 3u32, 0u32),
        (1, 3, 0x0a0f),
        (0, 0x20, 0),
        (0, u32::MAX, 0x0a0f),
        (0, 0x1f, 0x0a0f),
    ]) {
        t.set_u32(w(0x16), off);
        t.set_u32(w(0x27), actor);
        let table = w(0x6d);
        for i in 0..16 {
            let v = if i == 15 {
                last
            } else if i % 5 == 1 {
                0x0a00 + i
            } else {
                0
            };
            t.set_u32(table + 4 * i, v);
        }
        t.set_u32(flag, 0x77 + k);
        ownb::dirty_stack(&mut t, 0x14a0 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r45::actors_pass_0392,
        );
    }
}

/// `magic_0392` 0x100013c0 (`m0449::task_step_pass` through
/// `r45::TASK_STEP_PASS_0392`): the script and the motion each done or
/// not; when both are, this DLL's actors' pass and the node given back.
#[test]
fn task_step_0392() {
    let Some(mut t) = sa::twin(392) else { return };
    let at = t.at(0x13c0);
    let w = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    all_stubs(&mut t);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, at, 0x07), s);
    t.write(w(&t, at, 0x10), &[0x3b; 0x30]);
    t.set_u32(w(&t, at, 0x26), 0x0026_0026);
    let run = t.img.rel32_target(at + 0x33);
    let motion = t.img.rel32_target(at + 0x3a);
    let pass = t.img.rel32_target(at + 0x51);
    let gb = t.img.rel32_target(at + 0x5c);
    t.set_u32(w(&t, run, 0x07), SCRATCH + 0x700);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ (a[1] << 8));
    rec(&mut t, 779, 4, |_, a| a[3]);
    rec(&mut t, 211, 2, |_, a| 0x0211_0000 ^ a[0]);
    let reg = t.img.rel32_target(pass + 0x78);
    t.set_u32(t.img.word(reg + 0x21), SCRATCH + 0x600);
    t.set_u32(w(&t, pass, 0x16), 1);
    let table = w(&t, pass, 0x6d);
    for i in 0..16 {
        t.set_u32(table + 4 * i, if i == 3 { 0x0a03 } else { 0 });
    }
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x280, SCRATCH + 0x2c0);
    for (mode, done) in [(0u8, 1u32), (7, 1), (0, 0), (7, 0)] {
        t.set_u8(w(&t, run, 0x01), mode);
        t.set_u32(w(&t, motion, 0x02), done);
        t.set_u16(w(&t, motion, 0x11), 5);
        t.write(s, &vec![0x6c; 0x6000]);
        for i in 0..4 {
            t.set_u32(s + 0xf60 + 4 * i, 0x0f60_0000 + i);
        }
        t.write(node, &[0x3c; 0x20]);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x22; 0x10]);
        t.set_u32(w(&t, gb, 0x74), 0);
        t.set_u16(w(&t, gb, 0x8e), 7);
        t.set_u32(w(&t, gb, 0x48), node);
        ownb::dirty_stack(&mut t, 0x13c0, &[]);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// Matrix and vector slots of the 0385 burst: 235 an identity, 329 adding
/// its angle, 231 a product of the first rows, 229 `M V` per row; 12 and
/// 314 floats in ST0 (314's steps per call), 262 a position.
fn stubs_0385(t: &mut Twin) {
    all_stubs(t);
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(t, 329, 3, |m, a| {
        let x = f32::from_bits(a[2]);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + x * (i % 3) as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let mut v = 0.0f32;
            for j in 0..4 {
                v += rf(m, a[1] + 16 * i + 4 * j) * rf(m, a[2] + 4 * j);
            }
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xf) as f32 * 1.25 + i as f32 * 0.5 - 4.0);
        }
        0x262
    });
    rec(t, 12, 1, |_, a| 0x12 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 7) * 0.5 + 0.25);
    let k = SCRATCH + 0x7e10;
    rec(t, 314, 1, move |m, a| {
        let c = m.r32(k);
        m.w32(k, c + 1);
        0x0314_0000 ^ a[0] ^ c
    });
    t.stub_st0(314, move |m, a| {
        f64::from(f32::from_bits(a[0])) * 0.125 * f64::from(m.r32(k) % 7)
    });
}

/// `magic_0385` 0x10004b00 (`r45::shard_burst_0385`): the wait counting
/// down, no count left, the tick below 4, and bursts (the pool with room,
/// running out after one shard, empty) with the yaw wrapping both ways and
/// not (the caster around the actors).
#[test]
fn shard_burst_0385() {
    let Some(mut t) = sa::twin(385) else { return };
    let f = t.at(0x4b00);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    stubs_0385(&mut t);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let mp = t.img.rel32_target(f + 0x75);
    let mw = |o: u32| img.word(mp + o);
    t.set_u32(mw(0x11), 3);
    t.set_u32(mw(0x46), 0x0c0c);
    t.set_u32(mw(0x4d), SCRATCH + 0x100);
    t.write(SCRATCH + 0x100, &[3, 1, 0, 2, 7]);
    for i in 0..8u32 {
        t.set_u32(mw(0x58) + 4 * i, 0xc400 + i);
    }
    let sel = SCRATCH + 0x180;
    t.set_u32(w(0x7a), sel);
    for i in 0..0x30 {
        t.set_u32(w(0x84) + 4 * i, 0x0c00 + 3 * i);
    }
    let take = t.img.rel32_target(f + 0x1d1);
    let tw = |o: u32| img.word(take + o);
    let recs = SCRATCH + 0x1_0000;
    t.set_u32(tw(0x16), recs);
    // (wait, count, tick, the caster, records free)
    let cases: [(i32, i32, u32, u32, u32); 9] = [
        (3, 5, 0, 0, 6),
        (0, 0, 0, 0, 6),
        (-1, -2, 0, 0, 6),
        (0, 5, 1, 0, 6),
        (0, 5, 3, 0x00, 6),
        (0, 1, 3, 0x0f, 6),
        (0, 2, 7, 0x09, 1),
        (0, 2, 3, 0x05, 0),
        (0, 9, 3, 0x0c, 2),
    ];
    for (k, (wait, n, tick, caster, free)) in (0u32..).zip(cases) {
        t.set_u32(w(0x1b), wait.cast_unsigned());
        t.set_u32(w(0x40), n.cast_unsigned());
        t.set_u32(w(0x53), tick);
        t.set_u32(w(0xba), 0x0ca0 + caster);
        t.write(sel, &[(k as u8) % 5, 0x5a]);
        junk(&mut t, recs, 0x140 * 6, k);
        for i in 0..6 {
            let r = recs + 0x140 * i;
            t.set_u32(r + 0x130, u32::from(i >= free));
            t.set_u16(r + 0x124, 0x8000);
        }
        t.set_u32(tw(0x09), 6);
        t.set_u32(tw(0x01), 0);
        magic_r11::pools(&mut t, f, 835);
        ownb::dirty_stack(&mut t, 0x4b00 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0297` 0x10007e30 (`r45::summon_start_0297`): as
/// `magic_s4_summon.rs`'s start test, with slot 803 handing out the
/// environment and the lock word as the game left it.
#[test]
fn summon_start_0297() {
    let Some(mut t) = sa::twin(297) else { return };
    let at = t.at(0x7e30);
    all_stubs(&mut t);
    let statics = t.img.rel32_target(at + 0xd);
    let root = t.img.word(statics + 0x10d);
    let lock = sa::node(21);
    t.set_u32(t.img.word(at + 0xf2), lock);
    t.set_u32(t.img.word(at + 0x119), lock);
    let list = sa::node(20);
    rec(&mut t, 803, 2, |_, a| 0x0080_0000 ^ a[1]);
    rec(&mut t, 718, 1, |_, _| 0x718);
    for s in [222u32, 221] {
        rec(&mut t, s, 4, move |_, _| s);
    }
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    rec(&mut t, 711, 3, move |m, a| {
        // the lock word while the program starts
        m.w32(list + 0x40, m.r32(lock));
        a[0] ^ 0x711
    });
    rec(&mut t, 710, 4, |_, _| 0x710);
    for (k, old) in [0u32, 7].into_iter().enumerate() {
        let k = k as u32;
        t.write(list, &[0x80, 0x81]);
        t.set_u32(lock, old);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, 0);
        t.set_u32(t.img.word(at + 0x1b8), 0x5555_5555 + k);
        // the words the start clears or sets, junk before
        junk_ops(
            &mut t,
            at,
            &[0x8, 0x26, 0x30, 0x63, 0x78, 0x94, 0xa9, 0xc5, 0xfb],
        );
        junk_ops(&mut t, at, &[0x19e, 0x1c9, 0x1d3, 0x1dd, 0x17b]);
        let tail = SCRATCH + 0x1800;
        junk(&mut t, tail, 0x20, k);
        t.set_u32(t.img.word(at + 0x17b), tail);
        t.check_entry(at, Call::default(), &[0x30 + k], Eax::Checked);
    }
}

/// `magic_0348` 0x100070b0 (`r45::light_setup_0348`): the scale task's
/// list empty and not, the state block junk with varied light words.
#[test]
fn light_setup_0348() {
    let Some(mut t) = sa::twin(348) else { return };
    let f = t.at(0x70b0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 8, 2, |_, a| 0x0008_0000 ^ a[0]);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let st = t.img.rel32_target(f + 0x3bf);
    let head = t.img.word(st + 0x79);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0xc), s);
    t.set_u32(w(0x3cc), 0x0d0d_0000);
    for k in 0..3u32 {
        junk(&mut t, s, 0xd200, k);
        floats(&mut t, s + 0xd10, 0x8, k);
        floats(&mut t, s + 0xd20, 0x4, k + 2);
        let tail = SCRATCH + 0x1800;
        junk(&mut t, tail, 0x20, k);
        t.set_u32(head, if k == 1 { 0 } else { tail });
        t.set_u32(t.img.word(st + 0x91), tail);
        magic_r11::pools(&mut t, f, 987);
        ownb::dirty_stack(&mut t, 0x70b0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0348` 0x10006a10 (`r45::packed_0348`): as `magic_r38.rs`'s
/// packed test: the counter run out (the node given back), counting down
/// (a negative divisor; a zero one faults in the original), the angle
/// wrapping and not, the texture filled and not.
#[test]
fn packed_0348() {
    let Some(mut t) = sa::twin(348) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6a10);
    let give = t.img.rel32_target(f + 0x2f);
    let desc = SCRATCH + 0x700;
    rec(&mut t, 719, 3, move |_, _| desc);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x2350_0000 ^ i);
        }
        0x235
    });
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[1] + 4 * i, a[0] ^ i);
        }
        0x210
    });
    let answer = SCRATCH + 0x7e0;
    rec(&mut t, 885, 7, move |m, _| m.r32(answer));
    rec(&mut t, 905, 9, |m, a| {
        let c = m.r32(SCRATCH + 0xb20);
        m.w32(SCRATCH + 0xb20, c + 1);
        0x0905_0000 ^ a[0]
    });
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    let draw = t.img.rel32_target(f + 0x10b);
    t.set_u32(t.img.word(draw + 0x3c), SCRATCH + 0x7f0);
    t.set_u32(SCRATCH + 0x7f0, 0x5a5a_0001);
    // (count, divisor, angle, filled)
    for (k, (c, total, angle, filled)) in (0u32..).zip([
        (0i32, 9i32, 1.0f32, 0u32),
        (-2, 9, 1.0, 1),
        (5, 9, 1.0, 1),
        (5, 7, -0.25, 0),
        (1, 9, 0.0, 1),
        (2, 4, 3.0, 1),
        (9, -3, -5.0, 7),
    ]) {
        let s = SCRATCH + 0x1_0000;
        junk(&mut t, s + 0xd000, 0x200, k);
        t.set_u32(t.img.word(f + 0x15), s);
        t.set_u32(s + 0xd17c, c.cast_unsigned());
        t.set_u32(s + 0xd178, total.cast_unsigned());
        setf(&mut t, s + 0xd174, angle);
        t.set_u32(desc, 0x1234_5678 ^ k);
        t.set_u32(desc + 4, 0x9abc_def0 ^ k);
        t.set_u32(answer, filled);
        t.set_u32(t.img.word(f + 0x99), SCRATCH + 0x3000 + 0x10 * k);
        t.set_u32(t.img.word(f + 0xce), 0x0ce0_0000 + k);
        junk(&mut t, SCRATCH + 0x3000, 0x900, k);
        t.set_u32(SCRATCH + 0x3830 + 0x10 * k, 0x0830_0000 + k);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x6a10 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
    assert!(t.u32(SCRATCH + 0xb20) > 0, "the texture was drawn");
}

/// `magic_0348` 0x10006810 (`r45::glow_start_0348`) and 0x10006e40
/// (`r45::orbs_start_0348`): the scale task's list empty and not; the
/// frame words the originals never write (`[ebp - 8]`, `[ebp - 0xc]`)
/// pinned to 0 (brief 2026-10-24).
#[test]
fn glow_orbs_start_0348() {
    let Some(mut t) = sa::twin(348) else { return };
    let img = t.img.clone();
    all_stubs(&mut t);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 8, 2, |_, a| 0x0008_0000 ^ a[0]);
    let s = SCRATCH + 0x1_0000;
    for (rva, gs) in [(0x6810u32, 0x1bu32), (0x6e40, 0x13)] {
        let f = t.at(rva);
        let w = |o: u32| img.word(f + o);
        summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
        t.set_u32(w(gs), s);
        let st = t.img.rel32_target(t.at(0x6810) + 0x10f);
        let head = t.img.word(st + 0x79);
        for k in 0..3u32 {
            junk(&mut t, s, 0xd200, k);
            floats(&mut t, s + 0xd10, 0x8, k);
            floats(&mut t, s + 0xd20, 0x4, k + 2);
            let tail = SCRATCH + 0x1800;
            junk(&mut t, tail, 0x20, k);
            t.set_u32(head, if k == 1 { 0 } else { tail });
            t.set_u32(t.img.word(st + 0x91), tail);
            if rva == 0x6e40 {
                t.set_u32(w(0x247), 0x0d0d_0000 + k);
            }
            magic_r11::pools(&mut t, f, 609);
            ownb::dirty_stack(&mut t, rva + k, &[0xc, 0x10]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}
