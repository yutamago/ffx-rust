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

use ffx_magic::Ret;
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

/// `magic_0431` 0x10001690 (`r45::script_end_0431`): each script and
/// motion done or waiting; when the first of each is done, the show and
/// sounds (as `magic_0431.rs`'s test) and the node given back.
#[test]
fn script_end_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x1690);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    // the show and sounds
    let show = t.img.rel32_target(f + 0x72);
    let sw = |o: u32| img.word(show + o);
    rec(&mut t, 384, 1, |_, a| 0x0384_0000 ^ a[0]);
    for s in [25u32, 24] {
        rec(&mut t, s, 2, move |_, a| {
            (0x0100_0000 + s) ^ a[0] ^ (a[1] << 4)
        });
    }
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    rec(&mut t, 772, 2, |_, a| a[0] ^ 0x0772_0000);
    t.stub_edx(772, |_, a| a[1] ^ 0x0edd_0000);
    rec(&mut t, 779, 4, |_, a| a[3] ^ 0x779);
    let b = SCRATCH + 0x700;
    t.set_u32(sw(0x01), b);
    t.write(b, &[0u8, 1, 2, 3, 4]);
    for i in 0..8 {
        t.set_u32(sw(0x0c) + 4 * i, 0x0a00_0000 + i);
    }
    let rp = t.img.rel32_target(show + 0xbc);
    let parts = SCRATCH + 0xc000;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0a30);
    let lock = SCRATCH + 0x5e0;
    t.set_u32(t.img.word(rp + 0x43), lock);
    t.set_u32(t.img.word(rp + 0x5f), lock);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    // the scripts' and motions' slots
    rec(&mut t, 411, 1, |_, a| a[0] & 1);
    rec(&mut t, 20, 1, |_, a| a[0] & 1);
    let give = t.img.rel32_target(f + 0x7e);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0x6), s);
    // (script 1 done, script 2 done, motion 1 done, motion 2 done)
    for (k, (a1, a2, m1, m2)) in (0u32..).zip([
        (true, true, true, true),
        (true, false, true, false),
        (false, true, true, true),
        (true, true, false, true),
        (false, false, false, false),
    ]) {
        t.write(w(0x11), &[0x3b; 0x30]);
        t.set_u32(w(0x27), 0x0027_0000 + k);
        for (ctx, done) in [(w(0x2f), a1), (w(0x3e), a2)] {
            t.set_u8(ctx + 0x56, if done { 0 } else { 2 });
            t.set_u16(ctx + 0x54, 0xffff);
        }
        for (m, done) in [(w(0x4a), m1), (w(0x54), m2)] {
            t.set_u32(m + 0x14, u32::from(done));
            t.set_u16(m + 0x18, 5);
        }
        junk(&mut t, s + 0xd00, 0x20, k);
        t.set_u32(lock, 0x77);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x1690 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0431` 0x100014d0 (`r45::packed_sprite_0431`, internal): as
/// `magic_o6_fam.rs`'s packed sprite test: descriptors with their fields
/// spread, slot 885 answering 0 or not; 885 fills its buffer from its
/// words and the lock, 905 hashes the buffer and the colour words.
#[test]
fn packed_sprite_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x14d0);
    all_stubs(&mut t);
    let lock = SCRATCH + 0x3f00;
    t.set_u32(t.img.word(f + 0x29), lock);
    t.stub(885, move |m, a| {
        let mut x = 0x885 ^ m.r32(lock).rotate_left(13);
        for (i, w) in (0u32..).zip(&a[..6]) {
            x = x.rotate_left(7) ^ *w;
            m.w32(a[6] + 4 * i, x);
        }
        m.w32(SCRATCH + 0x3f10, x);
        if m.r32(SCRATCH + 0x3f20) == 0 {
            0
        } else {
            x | 1
        }
    });
    t.stub(905, |m, a| {
        let mut x = 0x905u32;
        for (i, w) in (0u32..).zip(&a[..9]) {
            x = x.rotate_left(5) ^ *w ^ i;
        }
        for i in 0..6 {
            x = x.rotate_left(3) ^ m.r32(a[1] + 4 * i);
        }
        for i in 0..4 {
            x = x.rotate_left(3) ^ m.r32(a[7] + 4 * i);
        }
        m.w32(SCRATCH + 0x3f14, x);
        x
    });
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    let (owner, buffer) = (SCRATCH + 0x4000, SCRATCH + 0x5000);
    let cases = [
        (0x1234_5678u32, 0x0000_0002u32, 1u32),
        (0xfedc_ba98, 0xffff_fff1, 1),
        (0x4c00_3fff, 0x0000_0003, 1),
        (0x0000_0000, 0x0000_0000, 1),
        (0x8765_4321, 0x0000_000c, 0),
    ];
    for (c, (lo, hi, answers)) in (0u32..).zip(cases) {
        t.write(owner, &[0x2d; 0x840]);
        t.set_u32(owner + 0x830, 0x0083_0000 + c);
        t.write(buffer, &[0x5c; 0x20]);
        t.set_u32(lock, 0x55 + c);
        t.set_u32(SCRATCH + 0x3f20, answers);
        ownb::dirty_stack(&mut t, 0x14d0 + c, &[]);
        let call = Call {
            ecx: owner,
            ..Call::default()
        };
        t.check(
            f,
            call,
            &[0x00a0_0000 + c, buffer, lo, hi],
            Eax::Checked,
            ffx_magic::r45::packed_sprite_0431,
        );
    }
}

/// `magic_0431` 0x10003030 (`r45::sprite_frame_0431`, with the fixed
/// packet `r45::packet_0431` and the sprite): the angle and the turn
/// wrapping up, down and not; slot 885 answering 0 or not.
#[test]
fn sprite_frame_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x3030);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let sp = t.img.rel32_target(f + 0x153);
    let lock = SCRATCH + 0x3f00;
    t.set_u32(t.img.word(sp + 0x29), lock);
    t.stub(885, move |m, a| {
        let mut x = 0x885 ^ m.r32(lock).rotate_left(13);
        for (i, w) in (0u32..).zip(&a[..6]) {
            x = x.rotate_left(7) ^ *w;
            m.w32(a[6] + 4 * i, x);
        }
        if m.r32(SCRATCH + 0x3f20) == 0 {
            0
        } else {
            x | 1
        }
    });
    t.stub(905, |m, a| {
        let mut x = 0x905u32;
        for (i, w) in (0u32..).zip(&a[..9]) {
            x = x.rotate_left(5) ^ *w ^ i;
        }
        for i in 0..16 {
            x = x.rotate_left(3) ^ m.r32(a[2] + 4 * i);
        }
        for i in 0..4 {
            x = x.rotate_left(3) ^ m.r32(a[7] + 4 * i);
        }
        m.w32(SCRATCH + 0x3f14, x);
        x
    });
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    let desc = SCRATCH + 0x700;
    rec(&mut t, 719, 3, move |_, _| desc);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(&mut t, 329, 3, |m, a| {
        let x = f32::from_bits(a[2]);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + x * (i % 3) as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    let spin = SCRATCH + 0x7e20;
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |m, _| f64::from(m.r32(spin)) * 1.75 - 7.0);
    rec(&mut t, 382, 2, |_, a| 0x0382_0000 ^ a[0]);
    let sel = SCRATCH + 0x180;
    t.set_u32(w(0x23), sel);
    for i in 0..0x30 {
        t.set_u32(w(0x32) + 4 * i, SCRATCH + 0x4000 + 0x10 * i);
    }
    junk(&mut t, SCRATCH + 0x4000, 0x1000, 0);
    t.set_u32(w(0xb9), 0x0c0c_0000);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0x19), s);
    let hi = f32::from_bits(t.u32(w(0x77)));
    // (the angle, slot 333's step, 885 answers)
    for (k, (a, spin_k, answers)) in (0u32..).zip([
        (0.5f32, 0u32, 1u32),
        (hi + 0.5, 1, 1),
        (-hi - 0.5, 2, 0),
        (2.0, 3, 1),
        (-2.5, 6, 1),
        (1.0, 8, 1),
    ]) {
        junk(&mut t, s + 0xd00, 0x20, k);
        setf(&mut t, s + 0xd04, a);
        t.set_u32(spin, spin_k);
        t.write(sel, &[0x5a, (k as u8) % 4]);
        t.set_u32(desc, 0x1234_5678 ^ (k << 9));
        t.set_u32(desc + 4, 0x0000_0003 ^ k);
        t.set_u32(lock, 0x55 + k);
        t.set_u32(SCRATCH + 0x3f20, answers);
        // the fixed packet's buffer, junk before it is built
        let pk = t.img.rel32_target(f + 0x1e);
        let buf = t.img.word(pk + 0xc);
        junk(&mut t, buf, 0x60, k);
        ownb::dirty_stack(&mut t, 0x3030 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0431` 0x100018f0 (`r45::actor_motions_0431`, internal): the
/// selection's actors at varied bytes.
#[test]
fn actor_motions_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x18f0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    for (s, n) in [(357u32, 3usize), (356, 3), (362, 4), (399, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0] ^ a[1]);
    }
    let sel = SCRATCH + 0x180;
    t.set_u32(w(0x1), sel);
    for i in 0..0x30 {
        t.set_u32(w(0x12) + 4 * i, 0x0c00 + 3 * i);
    }
    t.set_u32(w(0x4d), 0x0a4d_0000);
    t.set_u32(w(0xe5), 0x0ae5_0000);
    for k in 0..3u32 {
        t.write(sel, &[0x5a, k as u8 + 1, 7 - k as u8]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(ffx_magic::r45::actor_motions_0431(g, img, fr.at))
        });
    }
}

/// `magic_0431` 0x10004220 (`r45::objects_start_0431`): the yaw wrapping
/// up, down and not; the object list empty and not.
#[test]
fn objects_start_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x4220);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) + (a[1] & 0xf) as f32;
            wf(m, a[3] + 4 * i, v);
        }
        0x237
    });
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0x1c), s);
    t.set_u32(w(0x54), 0x0c0c_0000);
    let hi = f32::from_bits(t.u32(w(0xaf)));
    for (k, yaw) in (0u32..).zip([0.5f32, hi + 3.0, -hi - 0.5, 5.0]) {
        junk(&mut t, s, 0xd10, k);
        floats(&mut t, s + 0x820, 4, k);
        setf(&mut t, s + 0xd00, yaw);
        magic_r11::pools(&mut t, f, 318);
        ownb::dirty_stack(&mut t, 0x4220 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0431` 0x10003d30 (`r45::object_place_0431`): records 0..3,
/// points varied; the matrix slots write what they read.
#[test]
fn object_place_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x3d30);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    for s in [210u32, 230] {
        rec(&mut t, s, 2, move |m, a| {
            for i in 0..16 {
                let v = (a[1] & 0x1f) as f32 * 0.25 + i as f32 - (s & 3) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let mut v = 0.0f32;
            for j in 0..4 {
                v += rf(m, a[1] + 16 * i + 4 * j) * rf(m, a[2] + 4 * j);
            }
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    rec(&mut t, 306, 6, |_, a| 0x3060_0000 ^ a[2]);
    let sel = SCRATCH + 0x180;
    t.set_u32(w(0x30), sel);
    for i in 0..0x30 {
        t.set_u32(w(0x3c) + 4 * i, 0x0c00 + 3 * i);
    }
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0x20), s);
    let pts = SCRATCH + 0x2000;
    let o = SCRATCH + 0x3000;
    for k in 0..4u32 {
        junk(&mut t, s + 0xc80, 0x100, k);
        let r = s + 0xc8c + 0x30 * k;
        t.set_u32(r, 0x00c8_0000 + k);
        for j in 0..3u32 {
            t.set_u32(r + 4 + 4 * j, pts + 0x10 * j);
        }
        floats(&mut t, pts, 12, k);
        junk(&mut t, o, 0x120, k);
        t.set_u32(o + 0x120, k);
        t.write(sel, &[k as u8 + 2, 0x5a]);
        ownb::dirty_stack(&mut t, 0x3d30 + k, &[]);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
}

/// `magic_0266` 0x10002500 (`r45::fading_sprite_0266`, internal): each
/// mode with its counter running out and not, a mode past 2 and a
/// negative one; the angle wrapping and not; slot 885 answering. The
/// camera rows (544) and the sprite (885, 905) write and hash what they
/// get.
#[test]
fn fading_sprite_0266() {
    let Some(mut t) = sa::twin(266) else { return };
    let f = t.at(0x2500);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    t.stub(544, |m, a| {
        for j in 0..4 {
            let v = (a[0] * 4 + j) as f32 * 0.75 - 3.0;
            m.w32(a[1] + 4 * j, v.to_bits());
        }
        0x544
    });
    t.stub(553, |m, a| {
        let mut x = 0x553u32;
        for r in 0..4u32 {
            for j in 0..4u32 {
                x = x.rotate_left(3) ^ m.r32(a[1 + r as usize] + 4 * j);
            }
        }
        m.w32(SCRATCH + 0x3f18, x);
        x
    });
    for (s, n) in [(235u32, 1usize), (210, 2), (13, 2)] {
        rec(&mut t, s, n, move |m, a| {
            let p = if s == 235 { a[0] } else { a[1] };
            for i in 0..16 {
                m.w32(p + 4 * i, (s << 16) ^ i);
            }
            s
        });
    }
    let sp = t.img.rel32_target(f + 0x15f);
    let lock = SCRATCH + 0x3f00;
    t.set_u32(t.img.word(sp + 0x51), lock);
    t.stub(885, move |m, a| {
        let mut x = 0x885 ^ m.r32(lock).rotate_left(13);
        for (i, w) in (0u32..).zip(&a[..6]) {
            x = x.rotate_left(7) ^ *w;
            m.w32(a[6] + 4 * i, x);
        }
        x | 1
    });
    t.stub(905, |m, a| {
        let mut x = 0x905u32;
        for (i, w) in (0u32..).zip(&a[..9]) {
            x = x.rotate_left(5) ^ *w ^ i;
        }
        for i in 0..16 {
            x = x.rotate_left(3) ^ m.r32(a[2] + 4 * i);
        }
        for i in 0..4 {
            x = x.rotate_left(3) ^ m.r32(a[7] + 4 * i);
        }
        m.w32(SCRATCH + 0x3f14, x);
        x
    });
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    let desc = SCRATCH + 0x700;
    rec(&mut t, 719, 3, move |_, _| desc);
    let owner = SCRATCH + 0x4000;
    t.set_u32(w(0x111), owner);
    let lim = f32::from_bits(t.u32(w(0x10b)));
    // (mode, counter, the angle)
    for (k, (mode, c, a)) in (0u32..).zip([
        (0u32, 9u32, 0.5f32),
        (0, 1, lim),
        (1, 5, 1.0),
        (1, 1, -2.0),
        (2, 17, lim - 0.01),
        (2, 1, 0.0),
        (3, 4, 0.0),
        (u32::MAX, 4, 0.0),
    ]) {
        t.set_u32(w(0x14), mode);
        t.set_u32(w(0x6f), c);
        setf(&mut t, w(0xfb), a);
        t.write(owner, &[0x2d; 0x840]);
        t.set_u32(owner + 0x830, 0x0083_0000 + k);
        t.set_u32(desc, 0x1234_5678 ^ (k << 9));
        t.set_u32(desc + 4, 0x0000_0003 ^ k);
        t.set_u32(lock, 0x55 + k);
        ownb::dirty_stack(&mut t, 0x2500 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r45::fading_sprite_0266(g, img, fr)
        });
    }
}

/// `magic_0266` 0x10001f10 (`r45::actors_step_0266`, internal): the wait
/// counting down, the timer running out, no actors, and three actors
/// with their counters, spins and angles wrapping both ways; the timer
/// above and below 30. The matrix slots write what they read; 277 and
/// 15..17 answer floats in ST0.
#[test]
fn actors_step_0266() {
    let Some(mut t) = sa::twin(266) else { return };
    let f = t.at(0x1f10);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * ((i + s) % 3) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * rf(m, a[2] + 4 * i) + 0.25;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    for s in [277u32, 15, 16, 17] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| 1.0 + f64::from((a[0] ^ s) & 3) * 0.25);
    }
    let sc = t.img.rel32_target(f + 0x1c0);
    let sel = SCRATCH + 0x180;
    for o in [0x28u32, 0x74] {
        t.set_u32(t.img.word(sc + o), sel);
    }
    t.write(sel, &[3, 1, 4, 1, 5, 9, 2, 6]);
    for i in 0..0x30 {
        t.set_u32(t.img.word(sc + 0x33) + 4 * i, 0x0c00 + 3 * i);
        t.set_u32(t.img.word(sc + 0x7f) + 4 * i, 0x0d00 + 5 * i);
    }
    let hi = f32::from_bits(t.u32(w(0x82)));
    let lo = f32::from_bits(t.u32(w(0xc9)));
    let recs = w(0x99) - 0x84;
    // (wait, timer, actors)
    for (k, (wait, timer, n)) in (0u32..).zip([
        (2i32, 5i32, 3u32),
        (0, 1, 3),
        (0, 0, 3),
        (0, 9, 0),
        (0, 40, 3),
        (0, 12, 3),
        (0, -3, 2),
    ]) {
        t.set_u32(w(0x14), wait.cast_unsigned());
        t.set_u32(w(0x33), timer.cast_unsigned());
        t.set_u32(w(0x74), n);
        junk(&mut t, recs, 0x88 * 3, k);
        for j in 0..3u32 {
            let r = recs + 0x84 + 0x88 * j;
            floats(&mut t, r - 0x84, 0x20, k + j);
            t.set_u32(r, (k + j) % 3);
            setf(&mut t, r - 4, 0.25 * j as f32 - 0.25);
            setf(&mut t, r - 0x3c, [hi - 0.1, lo + 0.1, 0.5][j as usize]);
            setf(&mut t, r - 0x1c, [lo + 0.1, hi - 0.1, -0.5][j as usize]);
        }
        ownb::dirty_stack(&mut t, 0x1f10 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r45::actors_step_0266(g, img, fr)
        });
    }
}

/// `magic_0266` 0x10001e00 (`r45::frame_0266`): the four answers all 1
/// (the actors' modes reset, the grid drawn) and each of them not 1 in
/// turn (the object's script waiting with its flag clear, the actors'
/// timer running, the motion waiting, the objects busy).
#[test]
fn frame_0266() {
    let Some(mut t) = sa::twin(266) else { return };
    let f = t.at(0x1e00);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    all_stubs(&mut t);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xf) as f32 * 0.75 + i as f32 - 3.0);
        }
        0x262
    });
    for (s, n) in [
        (399u32, 2usize),
        (214, 1),
        (210, 2),
        (211, 2),
        (217, 2),
        (255, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    for s in [277u32, 15, 16, 17] {
        t.stub_st0(s, move |_, a| 1.0 + f64::from((a[0] ^ s) & 3) * 0.25);
    }
    t.stub(885, |_, a| a[0] | 1);
    // the actors' matrix slots write what they read
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * ((i + s) % 3) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * rf(m, a[2] + 4 * i) + 0.25;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    // the sprite's mode past 2 (nothing drawn), the actors' wait 0
    let fs = call(0x14);
    t.set_u32(t.img.word(fs + 0x14), 7);
    let st = call(0x7f);
    t.set_u32(t.img.word(st + 0x14), 0);
    t.set_u32(t.img.word(st + 0x74), 0);
    let motion = call(0x86);
    let ro = call(0x8d);
    let list = t.img.word(ro + 0x7c);
    let o = w(0x5a);
    let sel = SCRATCH + 0x180;
    t.write(sel, &[3, 1, 4, 1, 5, 9, 2, 6]);
    let am = call(0xc9);
    t.set_u32(t.img.word(am + 0x15), sel);
    for i in 0..0x30 {
        t.set_u32(t.img.word(am + 0x2e) + 4 * i, 0x0c00 + 3 * i);
    }
    let dg = call(0xed);
    t.set_u32(t.img.word(dg + 0x08), 0);
    // the caster and `*G_D` (b6 test gap: both read 0 before)
    t.set_u32(w(0x2b), 0x0c0c_0044);
    t.set_u32(w(0xdd), 0x0ddd_0000);
    // (script flag, actors' timer, motion done, objects busy)
    for (k, (flag, timer, done, busy)) in (0u32..).zip([
        (true, 1u32, 1u32, false),
        (false, 1, 1, false),
        (true, 5, 1, false),
        (true, 1, 0, false),
        (true, 1, 1, true),
    ]) {
        junk(&mut t, o, 0x40, k);
        t.set_u16(o + 0x30, if flag { 0x8000 } else { 0x0001 });
        t.set_u32(o + 0x2c, 9);
        t.set_u32(o + 0x10, 0);
        t.set_u32(t.img.word(st + 0x33), timer);
        t.set_u32(t.img.word(motion + 0x02), done);
        t.set_u16(t.img.word(motion + 0x11), 5);
        let n = SCRATCH + 0x9f00;
        let obj = SCRATCH + 0xa000;
        t.set_u32(n + 8, obj);
        t.set_u16(obj + 0x124, 0x8001);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.set_u32(w(0xbe), 2);
        ownb::dirty_stack(&mut t, 0x1e00 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0266` 0x100021a0 (`r45::sparks_0266`, internal): the count run
/// out (and negative), the ring with room for both sparks, for one, for
/// none; `frnd` (314) and slot 307 answer stepping floats in ST0, `rand`
/// is the game's.
#[test]
fn sparks_0266() {
    let Some(mut t) = sa::twin(266) else { return };
    let f = t.at(0x21a0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + (a[1] & 0xf) as f32;
            wf(m, a[3] + 4 * i, v);
        }
        0x237
    });
    let k = SCRATCH + 0x7e10;
    for s in [314u32, 307] {
        rec(&mut t, s, 0, move |m, _| {
            let c = m.r32(k);
            m.w32(k, c + 1);
            (s << 16) ^ c
        });
        // (slot 307 takes no words: it sees whatever the stack holds)
        t.stub_st0(s, move |m, a| {
            let x = if s == 314 {
                f64::from(f32::from_bits(a[0]))
            } else {
                1.5
            };
            x * 0.125 * f64::from(m.r32(k) % 7) - 0.25
        });
    }
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    let take = t.img.rel32_target(f + 0x25);
    let tw = |o: u32| img.word(take + o);
    let ring = SCRATCH + 0x5000;
    t.set_u32(tw(0x16), ring);
    let hp = SCRATCH + 0x700;
    t.set_u32(w(0x6e), hp);
    t.set_u32(t.img.word(f + 0x45), 0x0c0c_0045);
    // (count, ring entries, free)
    for (c, (n, entries, free)) in (0u32..).zip([
        (0i32, 4u32, 4u32),
        (-2, 4, 4),
        (3, 4, 4),
        (1, 4, 1),
        (2, 4, 0),
        (5, 4, 2),
    ]) {
        t.set_u32(w(0x11), n.cast_unsigned());
        junk(&mut t, ring, 0x50 * entries, c);
        for i in 0..entries {
            t.set_u32(ring + 0x50 * i + 0x48, u32::from(i >= free));
        }
        t.set_u32(tw(0x09), entries);
        t.set_u32(tw(0x01), 0);
        t.set_u32(hp + 4, 0x0ade_0000 + c);
        ownb::dirty_stack(&mut t, 0x21a0 + c, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r45::sparks_0266(g, img, fr)
        });
    }
}

/// What `r45::start_up_0266` needs: the pools, the object start's slots,
/// the actors (`n`, their bytes and characters), the vector slots writing
/// what they get, slot 333's yaw, the free ring of task nodes (`free`
/// nodes), the task list empty or not, the battle flag.
fn start_up_0266_setup(t: &mut Twin, f: u32, k: u32, n: u32, yaw: f64, listed: bool) {
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(t);
    magic_r11::pools(t, f, 2356);
    magic_r11::object_slots(t);
    magic_r11::object_start_auto(t, t.img.rel32_target(f + 0x6f));
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xff) as f32 * 0.5 + i as f32);
        }
        0x262
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * ((i + s) % 3) as f32;
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
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, a| yaw + f64::from(a[0] & 3));
    for s in [277u32, 15, 16, 17] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| 1.0 + f64::from((a[0] ^ s) & 3) * 0.25);
    }
    for (s, m) in [
        (215u32, 1usize),
        (413, 1),
        (711, 3),
        (710, 4),
        (210, 2),
        (211, 2),
        (357, 3),
        (356, 3),
        (362, 4),
        (399, 2),
    ] {
        rec(t, s, m, move |_, a| (s << 16) ^ a[0]);
    }
    // the task nodes' free ring (four nodes and the records' one)
    magic_r11::ring(t, w(0x61a), w(0x62f), 6, SCRATCH + 0x5000);
    // the actors: their count, bytes and characters
    let sel = SCRATCH + 0x180;
    t.write(sel, &[3, 1, 0x13, 2]);
    for o in [0x2f2, 0x333] {
        t.set_u32(w(o), sel);
    }
    for o in [0x2fd, 0x33e] {
        for i in 0..0x14 {
            t.set_u32(w(o) + 4 * i, 0x0000_0c00 + i);
        }
    }
    let am = t.img.rel32_target(f + 0x228);
    t.set_u32(t.img.word(am + 0x15), sel);
    for i in 0..0x14 {
        t.set_u32(t.img.word(am + 0x2e) + 4 * i, 0x0000_0d00 + i);
    }
    let sc = t.img.rel32_target(f + 0x286);
    for o in [0x28u32, 0x74] {
        t.set_u32(t.img.word(sc + o), sel);
    }
    for i in 0..0x14 {
        t.set_u32(t.img.word(sc + 0x33) + 4 * i, 0x0000_0e00 + i);
        t.set_u32(t.img.word(sc + 0x7f) + 4 * i, 0x0000_0f00 + i);
    }
    // the words the start stores whole or in part, junk before
    junk_ops(
        t,
        f,
        &[
            0x54c, 0x562, 0x59e, 0x5f6, 0x60a, 0x613, 0x7a3, 0x90c, 0x916, 0x1d5,
        ],
    );
    // the actors' records, junk before (b8 test gap: their zeros hid the
    // angle stores the turns read)
    junk(t, w(0x21a) - 0x44, 0x88 * 3, k);
    t.set_u32(w(0x20f), n);
    t.set_u32(w(0x8f), 0x0000_008f + k);
    t.set_u32(w(0x538), 0x0000_0538 + k);
    t.set_u32(w(0x4f6), 0x0000_04f6 + k);
    t.set_u32(w(0x551), 0x0000_0551 + k);
    t.set_u32(w(0x545), k & 1);
    t.write(SCRATCH + 0x600, &[0x44; 0x20]);
    t.set_u32(w(0x68f), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(0x6a2), SCRATCH + 0x600);
}

/// `magic_0266` 0x10001490 (`r45::start_up_0266`, internal) and its load
/// step 0x10001210 (`r45::load_0266`): no actors, one and three; the yaw
/// kept and wrapped both ways; the battle flag both ways; the task list
/// empty and not.
#[test]
fn start_up_0266() {
    let Some(mut t) = sa::twin(266) else { return };
    let f = t.at(0x1490);
    for (k, (n, yaw, listed)) in (0u32..).zip([
        (0u32, 0.25f64, false),
        (1, 9.5, true),
        (3, -7.0, false),
        (2, 3.0, true),
    ]) {
        start_up_0266_setup(&mut t, f, k, n, yaw, listed);
        ownb::dirty_stack(&mut t, 0x1490 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r45::start_up_0266(g, img, fr)
        });
    }
    let load = t.at(0x1210);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0266_setup(&mut t, f, 2, 1, 1.0, true);
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let at = t.img.word(load + 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = t.img.word(load + o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        ownb::dirty_stack(&mut t, 0x1210, &[]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0434` 0x10004770 (`r45::project_0434`, internal): no points,
/// one and three; slot 325 copies, slot 229 multiplies.
#[test]
fn project_0434() {
    let Some(mut t) = sa::twin(434) else { return };
    let f = t.at(0x4770);
    all_stubs(&mut t);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * rf(m, a[2] + 4 * i) + 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    let (src, dst, mat) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x3000);
    for (k, n) in (0u32..).zip([0u32, 1, 3, u32::MAX]) {
        floats(&mut t, src, 16, k);
        junk(&mut t, dst, 0x40, k);
        floats(&mut t, mat, 16, k + 5);
        ownb::dirty_stack(&mut t, 0x4770 + k, &[]);
        let call = Call {
            ecx: src,
            edx: dst,
            ..Call::default()
        };
        t.check(f, call, &[n, mat], Eax::Checked, |g, img, fr| {
            ffx_magic::r45::project_0434(g, img, fr)
        });
    }
}

/// `magic_0434` 0x10007d80 (`r45::glow_start_0434`) and 0x100069a0
/// (`r45::summon_glow_0434`): the task list empty and not; slot 282's
/// float varied.
#[test]
fn glow_starts_0434() {
    let Some(mut t) = sa::twin(434) else { return };
    let img = t.img.clone();
    all_stubs(&mut t);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 282, 0, |_, _| 0x282);
    let spin = SCRATCH + 0x7e20;
    t.stub_st0(282, move |m, _| 1.5 + f64::from(m.r32(spin)) * 0.75);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, (a[1] & 0xf) as f32 + i as f32);
        }
        0x237
    });
    let s = SCRATCH + 0x1_0000;
    for (rva, ops) in [(0x7d80u32, (0xcdu32, 0xe2u32)), (0x69a0, (0x11a, 0x12f))] {
        let f = t.at(rva);
        let w = |o: u32| img.word(f + o);
        summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
        t.set_u32(w(0x13), s);
        if rva == 0x69a0 {
            t.set_u32(w(0x94), 0x0c0c_0094);
        }
        for k in 0..3u32 {
            junk(&mut t, s, 0x1b00, k);
            t.set_u32(spin, k);
            let tail = SCRATCH + 0x1800;
            junk(&mut t, tail, 0x20, k);
            t.set_u32(w(ops.0), if k == 1 { 0 } else { tail });
            t.set_u32(w(ops.1), tail);
            magic_r11::pools(&mut t, f, 347);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0434` 0x10005600 (`r45::bone_matrix_0434`, internal): no model
/// (the cookie word), bone 0 and bones 1..2 with their records.
#[test]
fn bone_matrix_0434() {
    let Some(mut t) = sa::twin(434) else { return };
    let f = t.at(0x5600);
    all_stubs(&mut t);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(
                m,
                a[0] + 4 * i,
                if i % 5 == 0 { 1.0 } else { 0.25 * i as f32 },
            );
        }
        0x235
    });
    rec(&mut t, 231, 3, |m, a| {
        let mut x = 0x231u32;
        for i in 0..16 {
            x = x.rotate_left(3) ^ m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 842, 3, |m, a| {
        let mut x = 0x842u32;
        for i in 0..16 {
            x = x.rotate_left(5) ^ m.r32(a[1] + 4 * i);
        }
        x ^ a[2]
    });
    let (o, b) = (SCRATCH + 0x1000, SCRATCH + 0x3000);
    let recs = SCRATCH + 0x4000;
    let ptrs = SCRATCH + 0x5000;
    for (k, (h, j)) in (0u32..).zip([(0u32, 1u32), (0x0830, 0), (0x0831, 1), (0x0832, 2)]) {
        junk(&mut t, o, 0x840, k);
        t.set_u32(o + 0x830, h);
        t.set_u32(o + 0x1bc, ptrs);
        t.set_u32(o + 0x328, SCRATCH + 0x6000);
        for i in 0..3 {
            t.set_u32(ptrs + 0x38 * i, recs + 0x10 * i);
            t.set_u16(recs + 0x10 * i, (2 + i * 3) as u16);
        }
        floats(&mut t, b, 16, k);
        ownb::dirty_stack(&mut t, 0x5600 + k, &[]);
        let call = Call {
            ecx: o,
            edx: j,
            ..Call::default()
        };
        t.check(f, call, &[b], Eax::Checked, |g, img, fr| {
            ffx_magic::r45::bone_matrix_0434(g, img, fr)
        });
    }
}

/// `magic_0434` 0x10006de0 (`r45::objects_start_0434`): the task lists
/// empty and not (one part registered, no banks).
#[test]
fn objects_start_0434() {
    let Some(mut t) = sa::twin(434) else { return };
    let f = t.at(0x6de0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 772, 2, |_, a| a[0] ^ 0x0772_0000);
    t.stub_edx(772, |_, a| a[1] ^ 0x0edd_0000);
    rec(&mut t, 779, 4, |_, a| a[3] ^ 0x779);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, (a[1] & 0xf) as f32 + i as f32);
        }
        0x237
    });
    // (prime first: it would fill the zero bank count)
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let rp = t.img.rel32_target(f + 0x9);
    let parts = SCRATCH + 0xc000;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0a30);
    let lock = SCRATCH + 0x5e0;
    t.set_u32(t.img.word(rp + 0x43), lock);
    t.set_u32(t.img.word(rp + 0x5f), lock);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(0x4), s);
    t.set_u32(w(0x4d), 0x0c0c_004d);

    for k in 0..3u32 {
        junk(&mut t, s, 0x1b00, k);
        floats(&mut t, s + 0xca0, 4, k);
        let tail = SCRATCH + 0x1800;
        junk(&mut t, tail, 0x20, k);
        t.set_u32(w(0x128), if k == 1 { 0 } else { tail });
        t.set_u32(w(0x13b), tail);

        t.set_u32(lock, 0x77 + k);
        magic_r11::pools(&mut t, f, 372);
        ownb::dirty_stack(&mut t, 0x6de0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0434` 0x10007580 (`r45::turn_start_0434`) and 0x10007770
/// (`r45::turn_end_0434`): the task list empty and not; the end with and
/// without its node (given back, the lock as the game left it). Slots
/// 277, 15, 16, 17 answer floats in ST0; the turns write what they read.
#[test]
fn turns_0434() {
    let Some(mut t) = sa::twin(434) else { return };
    let img = t.img.clone();
    all_stubs(&mut t);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * ((i + s) % 3) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    for s in [277u32, 15, 16, 17] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| 1.0 + f64::from((a[0] ^ s) & 3) * 0.25);
    }
    rec(&mut t, 829, 5, |_, a| 0x0829_0000 ^ a[0]);
    let sel = SCRATCH + 0x180;
    t.write(sel, &[3, 1, 4, 1]);
    let s = SCRATCH + 0x1_0000;
    let start = t.at(0x7580);
    let end = t.at(0x7770);
    let ws = |o: u32| img.word(start + o);
    let we = |o: u32| img.word(end + o);
    t.set_u32(ws(0x7), s);
    t.set_u32(ws(0x9a), sel);
    for i in 0..0x14 {
        t.set_u32(ws(0xa4) + 4 * i, 0x0c00 + 3 * i);
    }
    summon_stubs::prime(&mut t, start, SCRATCH + 0x4_0000);
    for k in 0..3u32 {
        junk(&mut t, s, 0x1b00, k);
        floats(&mut t, s + 0xc60, 0x10, k);
        let tail = SCRATCH + 0x1800;
        junk(&mut t, tail, 0x20, k);
        t.set_u32(ws(0x1a3), if k == 1 { 0 } else { tail });
        t.set_u32(ws(0x1bc), tail);
        magic_r11::pools(&mut t, start, 488);
        ownb::dirty_stack(&mut t, 0x7580 + k, &[]);
        t.check_entry(start, Call::default(), &[], Eax::Checked);
    }
    let give = t.img.rel32_target(end + 0x18a);
    let lock = SCRATCH + 0x5e0;
    t.set_u32(we(0x18f), lock);
    t.set_u32(we(0x1a8), SCRATCH + 0x6000);
    for (k, has) in (0u32..).zip([true, false, true]) {
        junk(&mut t, s, 0x1b00, k);
        floats(&mut t, s + 0xc60, 0x10, k + 1);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        t.set_u32(s + 0x1a60, if has { node } else { 0 });
        t.set_u32(lock, 0x77 + k);
        ownb::dirty_stack(&mut t, 0x7770 + k, &[]);
        t.check_entry(end, Call::default(), &[], Eax::Checked);
    }
}
