//! Round 43's own ports (`ffx_magic::r43`) against each original. Needs
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

use ffx_magic::{Ret, Xmm, r43};
use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};
use sa::rec;

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

fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

/// `n` bytes that differ from their neighbours (`seed` picks the pattern).
fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// The entry's stack pointer (at the return address).
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// The frame area below the entry's stack pointer filled with a pattern.
fn scrub(t: &mut Twin) {
    t.write(ENTRY_ESP - 0x3000, &varied(0x3000, 0x5b));
}

/// Logs each call of `slot` into a row of its own (16 rows of 0x40
/// bytes at `log`): the call's number, its first `n` words and the word
/// at `watch`. Answers `ret(m, args, k)`.
fn log_calls(
    t: &mut Twin,
    slot: u32,
    log: u32,
    n: usize,
    watch: Option<u32>,
    ret: impl Fn(&mut dyn StubMem, &[u32], u32) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let k = m.r32(log);
        m.w32(log, k + 1);
        let row = log + 4 + 0x40 * (k % 16);
        m.w32(row, slot);
        for (i, w) in (1u32..).zip(&a[..n.min(a.len())]) {
            m.w32(row + 4 * i, *w);
        }
        if let Some(p) = watch {
            let at = m.r32(p);
            m.w32(row + 0x3c, m.r32(at));
        }
        ret(m, a, k)
    });
}

/// `magic_0445` 0x10007890 (`r43::summon_start_0445`, cdecl `(actor)`):
/// as `s4_summon`'s start test, with the lock word as the game left it,
/// the cleared word set before, and slot 291 writing its three outputs
/// through the local (each pointer logged, the local read back).
#[test]
fn summon_start_0445() {
    let Some(mut t) = sa::twin(445) else { return };
    all_stubs(&mut t);
    let at = t.at(0x7890);
    let targets = t.img.rel32_target(at + 0x9);
    let statics = t.img.rel32_target(targets + 0xe0);
    let root = t.img.word(statics + 0x10d);
    let lock = sa::node(21);
    t.set_u32(t.img.word(at + 0xd5), lock);
    let list = sa::node(20);
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        list
    });
    rec(&mut t, 531, 2, |m, a| {
        m.w32(a[1], a[0]);
        0x531
    });
    rec(&mut t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, _| 2.5);
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
    // the local's old value (the entry's ecx), then the three outputs
    rec(&mut t, 291, 5, |m, a| {
        let old = m.r32(a[2]);
        m.w32(SCRATCH + 0x7f0, old);
        m.w32(a[2], 0x2910_0000 ^ a[0]);
        m.w32(a[3], m.r32(a[3]) ^ 0x0291);
        m.w32(a[4], m.r32(a[4]) + 3);
        m.w32(SCRATCH + 0x7f4, m.r32(a[4]));
        (a[1] & 0x3f) + 2
    });
    rec(&mut t, 523, 7, |_, a| 0x5230_0000 ^ a[1]);
    rec(&mut t, 575, 0, |_, _| 0x575);
    for (k, old) in [0u32, 7, 0x1234].into_iter().enumerate() {
        let k = k as u32;
        t.write(list, &[0x80, 0x81]);
        t.set_u32(lock, old);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, 0);
        t.set_u32(t.img.word(at + 0x1a3), 0x5555_5555 + k);
        // the words the start clears, set before
        t.set_u32(t.img.word(at + 0x9e), 0x9d9d_0000 + k);
        t.set_u32(t.img.word(at + 0x1c2), 0x1c2c_0000 + k);
        // the words the start sets (a store of 0, a u16), junk before
        junk_ops(&mut t, at, &[0x6d, 0x82]);
        // slot 291's two inputs
        t.set_u32(t.img.word(at + 0x1ad), 0x0d0d_0000 + k);
        t.set_u8(t.img.word(at + 0x19d), 0x40 + k as u8 * 0x31);
        t.check_entry(at, Call::this(0x0ecc_0000 + k), &[0x30 + k], Eax::Checked);
    }
}

/// `magic_0445` 0x10006bf0 (`r43::object_setup_0445`): the parent empty
/// and holding a node, the random offsets made and kept; slot 325 copies
/// its vector and logs `B`'s links, the slots log their words.
#[test]
fn object_setup_0445() {
    let Some(mut t) = sa::twin(445) else { return };
    let f = t.at(0x6bf0);
    let log = SCRATCH + 0x6000;
    log_calls(&mut t, 325, log, 2, None, move |m, a, k| {
        m.w32(log + 0x400 + 8 * (k % 8), m.r32(a[0].wrapping_sub(0x18)));
        m.w32(log + 0x404 + 8 * (k % 8), m.r32(a[0].wrapping_sub(0x14)));
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ k);
        }
        0x0325_0000 ^ a[0]
    });
    log_calls(&mut t, 307, log, 0, None, |_, _, k| 0x307 + k);
    t.stub_st0(307, move |m, _| 0.625 + f64::from(m.r32(log) % 5));
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    let pn = t.img.rel32_target(f + 0x19a);
    let start = t.img.rel32_target(pn + 0x88);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x2f0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    t.set_u32(w(&t, 0x16b), SCRATCH + 0x900);
    let parent = w(&t, 0x7b);
    let pl = w(&t, 0x181);
    let o = SCRATCH + 0x1000;
    let r = SCRATCH + 0x400;
    let p = SCRATCH + 0x480;
    for (k, (held, keep)) in [(false, 0u32), (true, 0), (false, 1), (true, 0x8000_0000)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        t.set_u32(log, 0);
        t.write(o, &varied(0x340, k));
        t.write(r, &varied(0x60, k + 1));
        t.write(p, &varied(0x10, k + 5));
        t.write(SCRATCH + 0x900, &varied(0x80, k + 4));
        t.set_u32(w(&t, 0x27), SCRATCH + 0x500);
        t.write(SCRATCH + 0x500, &varied(0x20, k + 2));
        t.set_u32(w(&t, 0x73), if held { SCRATCH + 0x540 } else { parent });
        t.write(SCRATCH + 0x540, &varied(0x20, k + 3));
        t.write(pl, &varied(0x1c, k + 6));
        t.set_u32(pl + 0x10, pl);
        for (o_, x) in [
            (0x108u32, 0.5f32),
            (0x205, 0.25),
            (0x212, 2.0),
            (0x21a, 1.0),
            (0x257, -0.75),
        ] {
            t.set_u32(w(&t, o_), x.to_bits());
        }
        // the task ring's flags
        junk_ops(&mut t, f, &[0x45]);
        scrub(&mut t);
        let c = Call {
            ecx: o,
            edx: r,
            ..Call::default()
        }
        .with_xmm(&[
            Xmm::f32(0.0),
            Xmm::f32(0.0),
            Xmm::f32(9.0),
            Xmm::f32(1.5 + k as f32),
        ]);
        t.check(f, c, &[p, keep], Eax::Checked, |g, img, fr| {
            Ret::eax(r43::object_setup_0445(g, img, fr))
        });
    }
}

/// An empty list head at `l` (its child list itself).
fn empty_list(t: &mut Twin, l: u32) {
    t.write(l, &[0; 0x20]);
    t.set_u32(l + 0x10, l);
    t.set_u32(l + 0xc, l);
}

/// A task at `SCRATCH + 0x700` for the record `obj` (its node in a ring
/// of three).
fn task_at(t: &mut Twin, obj: u32) -> u32 {
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

/// The node slots the tasks' callees make: 941 a program table, 714 the
/// part start, 713 the give-back, 325 a vector copy, 705 a float in ST0
/// that steps with each call; the particle start's lock and pointer
/// globals (the start wrapper behind `particle node` at `pn`).
fn node_slots(t: &mut Twin, pn: u32) {
    let table = SCRATCH + 0x3400;
    for j in 0..0x100 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x10);
        }
        0x325
    });
    let k705 = SCRATCH + 0x3300;
    rec(t, 705, 0, move |m, _| {
        let c = m.r32(k705);
        m.w32(k705, c + 1);
        0x705 + c
    });
    t.stub_st0(705, move |m, _| f64::from(m.r32(k705) % 7) * 1.5 - 3.0);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
}

/// `magic_0445` 0x10006ed0 (`r43::task_0445`), after the summon start
/// set the DLL's lists up: every case in a run's order (0, 0x46, 0x47
/// below and from its timer, 0x3c, 0x50, 0xa with three throws, 0xb
/// waiting, taken, free and last, 0x3d both ways, 0x3e below, empty and
/// with targets, 0x48 at timer 1 and from 0x28, 0x51 below and at 0x7a,
/// 0x3f below and from 15 (then 0x2710), states no case takes).
#[test]
fn task_0445() {
    let Some(mut t) = sa::twin(445) else { return };
    let f = t.at(0x6ed0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x39a);
    node_slots(&mut t, pn);
    // the start first (its own stubs)
    let start = t.at(0x7890);
    let lock = SCRATCH + 0x7f80;
    t.set_u32(t.img.word(start + 0xd5), lock);
    let models = SCRATCH + 0x7e00;
    t.write(models, &[3, 5, 7, 9]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        models
    });
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, (a[0] + i).wrapping_mul(0x0100_0001));
        }
        0x531
    });
    t.stub_st0(289, |_, _| 2.5);
    for s in [413u32, 215, 14, 333] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(333, |_, _| 0.75);
    // the colour node's unit block
    t.stub_st0(790, |_, _| 1.25);
    // the colour node's four integers
    rec(&mut t, 558, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, (a[1] & 0xff) + 3 * i);
        }
        0x558
    });
    rec(&mut t, 684, 1, |_, a| a[0] & 1);
    // 307's ST0 steps with each call
    let k307 = SCRATCH + 0x7e40;
    rec(&mut t, 307, 0, move |m, _| {
        let c = m.r32(k307);
        m.w32(k307, c + 1);
        0x307 + c
    });
    t.stub_st0(307, move |m, _| 0.375 + f64::from(m.r32(k307) % 5));
    let dmg = SCRATCH + 0x7e50;
    rec(&mut t, 291, 5, move |m, a| {
        m.w32(a[2], m.r32(a[2]) ^ 0x291);
        m.r32(dmg)
    });
    let targets = SCRATCH + 0x7e60;
    let ntargets = SCRATCH + 0x7e70;
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(ntargets));
        targets
    });
    let ok411 = SCRATCH + 0x7e74;
    rec(&mut t, 411, 1, move |m, _| m.r32(ok411));
    for (s, n) in [
        (296u32, 2usize),
        (541, 3),
        (213, 2),
        (228, 2),
        (234, 3),
        (878, 1),
        (331, 1),
        (353, 1),
        (271, 1),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    t.check_entry(start, Call::this(0x0ecc_0000), &[0x31], Eax::Checked);
    let tb = SCRATCH + 0x8000;
    junk(&mut t, tb, 0x1400, 3);
    let task = task_at(&mut t, tb);
    // the objects' table (`*G_N` of them, 0x98 apart) the targets are
    // looked up in
    let objs = w(0x4ff);
    t.write(targets, &[2, 4, 0x31, 6, 9]);
    // (state, timer, wait T+0x20, throws T+0x24, targets, 411, kept)
    let cases: [(u32, u32, u32, u32, u32, u32, u16); 24] = [
        (0, 5, 0, 0, 0, 0, 0),
        (0x46, 5, 0, 0, 0, 0, 0),
        (0x47, 9, 0, 0, 0, 0, 0),
        (0x47, 10, 0, 0, 5, 0, 0),
        (0x47, 12, 0, 0, 0, 0, 0),
        (0x3c, 3, 0, 0, 0, 0, 0),
        (0x50, 3, 0, 0, 0, 0, 0),
        (0xa, 3, 7, 3, 0, 0, 0),
        (0xb, 3, 4, 2, 0, 0, 0),
        (0xb, 3, 0, 2, 0, 0, 0),
        (0xb, 3, 0, 0xffff_fffd, 0, 0, 0),
        (0xb, 3, 0, 1, 0, 0, 0),
        (0x3d, 3, 0, 0, 0, 0, 0),
        (0x3d, 3, 0, 0, 0, 1, 0),
        (0x3e, 0x68, 0, 0, 0, 0, 2),
        (0x3e, 0x69, 0, 0, 0, 0, 0),
        (0x3e, 0x70, 0, 0, 0, 0, 3),
        (0x48, 1, 0, 0, 0, 0, 2),
        (0x48, 0x28, 0, 0, 0, 0, 2),
        (0x51, 0x79, 0, 0, 0, 0, 0),
        (0x51, 0x7a, 0, 4, 0, 0, 0),
        (0x3f, 14, 0, 0, 0, 0, 0),
        (0x3f, 15, 0, 0, 0, 0, 0),
        (0x60, 1, 0, 0, 0, 0, 0),
    ];
    for (k, (state, timer, wait, n, nt, ok, kept)) in (0u32..).zip(cases) {
        t.set_u32(tb + 0x18, state);
        t.set_u32(tb + 0x1c, timer);
        t.set_u32(tb + 0x20, wait);
        t.set_u32(tb + 0x24, n);
        t.set_u32(dmg, n);
        t.set_u32(ntargets, nt);
        t.set_u32(ok411, ok);
        t.set_u16(tb + 0x12d0, kept);
        for i in 0..4u32 {
            t.set_u16(tb + 0x1290 + 2 * i, (0x30 + i + k) as u16);
        }
        if k == 9 {
            // the object 2 taken
            t.set_u32(tb + 0x30 + 0x310 * 2 + 0x2d0, 3);
        }
        if state == 0x47 {
            // the model nodes' objects (`*G_N` of them, 0x98 apart) the
            // targets are looked up in
            t.set_u32(w(0x4f1), 3);
            for (i, id) in [7u32, 2, 0x31].into_iter().enumerate() {
                t.set_u32(objs + 0x98 * i as u32, id);
            }
        }
        // words a case stores whole or in part, junk before
        let ops: &[u32] = match state {
            0x50 => &[0x663],
            0x3f => &[0xcd, 0x14a, 0x176],
            0xa | 0xb => &[0x7b5, 0x7a1],
            _ => &[],
        };
        junk_ops(&mut t, f, ops);
        ownb::dirty_stack(&mut t, 0x6ed0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    // the unlinked node, then states past the cases
    for (k, state) in (30u32..).zip([0x2710u32, 0x2711, 0x52, 5]) {
        t.set_u32(tb + 0x18, state);
        let task = task_at(&mut t, tb);
        ownb::dirty_stack(&mut t, 0x6ed0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    // each slot reached (the original's calls)
    for (s, n) in [
        (228u32, 1u32),
        (213, 1),
        (411, 2),
        (296, 5),
        (541, 3),
        (234, 2),
        (878, 2),
        (288, 2),
        (291, 2),
    ] {
        let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * s);
        assert!(c >= n, "slot {s}: {c} calls");
    }
}

/// `magic_0273` 0x10007920 (`r43::strike_start_0273`): `T + 0x18` set;
/// the model's bit 0x400 clear; the strike started with the parent empty
/// and holding a node, the divisor 2, 3, 5 and -1 (0 faults in the
/// original).
#[test]
fn strike_start_0273() {
    let Some(mut t) = sa::twin(273) else { return };
    let f = t.at(0x7920);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], a[0] ^ (0x262 + i as u32));
        }
        0x262
    });
    rec(&mut t, 12, 1, |_, a| 0x12 ^ a[0]);
    t.stub_st0(12, |m, a| f64::from(m.r32(a[0]) & 0xff) * 0.5 - 2.0);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    let tb = SCRATCH + 0x1000;
    let model = SCRATCH + 0x1800;
    let parent = w(0x42);
    // (T+0x18, bit, parent holds a node, T+0x60, T+0x30)
    let cases: [(u32, u16, bool, i32, i32); 6] = [
        (1, 0x400, false, 9, 2),
        (0, 0x3ff, false, 9, 2),
        (0, 0x400, false, 9, 2),
        (0, 0xc00, true, -9, 3),
        (0, 0x400, true, 7, 5),
        (0, 0x400, false, -7, -1),
    ];
    for (k, (busy, bits, held, num, den)) in (0u32..).zip(cases) {
        junk(&mut t, tb, 0x100, k);
        junk(&mut t, model, 0x100, k + 1);
        t.set_u32(tb + 0x18, busy);
        t.set_u32(tb + 0x34, model);
        t.set_u16(model + 0x90, bits);
        t.set_u32(model + 0x54, 0x40 + k);
        t.set_u32(tb + 0x60, num.cast_unsigned());
        t.set_u32(tb + 0x30, den.cast_unsigned());
        empty_list(&mut t, parent);
        if held {
            let last = SCRATCH + 0x2000;
            t.write(last, &[0x3c; 0x20]);
            t.set_u32(parent + 0x10, last);
        }
        t.set_u32(w(0x70), 0x0007_0005 + k);
        let task = task_at(&mut t, tb);
        ownb::dirty_stack(&mut t, 0x7920 + k, &[]);
        let call = Call {
            eax: 0x0eaa_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[task], Eax::Checked);
    }
}

/// The vector and float slots the summon callees use, with their outputs
/// written: 217 a position `(m, v)`, 531 a battle position `(c, v)`, 262
/// a model's position `(m, x, y, z)`, 601 an angle into `a[1]`; 12, 333
/// and 705 floats in ST0; the vector slots of `summon_stubs`.
fn summon_slots(t: &mut Twin) {
    summon_stubs::vector_stubs(t);
    for s in [217u32, 531] {
        rec(t, s, 2, move |m, a| {
            for i in 0..4 {
                let v = ((a[0] & 0xf) as f32 + i as f32) * 0.75 - 1.0;
                wf(m, a[1] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xff) as f32 * 0.5 + i as f32);
        }
        0x262
    });
    rec(t, 601, 2, |m, a| {
        wf(m, a[1], (a[0] & 7) as f32 * 0.25 - 0.5);
        0x601
    });
    for s in [12u32, 333] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| f64::from(a[0] & 7) * 0.375 - 1.0);
    }
    for s in [413u32, 215, 14] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
}

/// `magic_0273` 0x10007a60 (`r43::summon_task_0273`), the globals primed:
/// every case in a run's order: 0, 0x3c below 30 and at it, 0x3d, 0xa,
/// 0x14, 0x15 below 5, with two targets (one flagged) and none; 0x1e with
/// the flag clear and set (the zeroed vector); 0x1f with the model id 0
/// and not; 0x3e, 0xb, 0x16, 0x20 with the answer 0 and not, the flag
/// clear and set (the next case dispatched again, 0x20 to 0x2710); 0x2710
/// alone; states no case takes.
#[test]
fn summon_task_0273() {
    let Some(mut t) = sa::twin(273) else { return };
    let f = t.at(0x7a60);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x502);
    node_slots(&mut t, pn);
    summon_slots(&mut t);
    let ok = SCRATCH + 0x7e80;
    for s in [20u32, 411] {
        rec(&mut t, s, 1, move |m, _| m.r32(ok));
    }
    for (s, n) in [
        (366u32, 2usize),
        (239, 3),
        (388, 3),
        (228, 2),
        (248, 2),
        (666, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    // the list heads the nodes join (and the particle node's)
    let pnw = t.img.rel32_target(f + 0x354);
    for l in [0x16du32, 0x282, 0x2b0, 0x2ea, 0x408, 0x44a, 0x4e5, 0x3a7] {
        empty_list(&mut t, w(l));
    }
    let l = t.img.word(pnw + 0x26);
    empty_list(&mut t, l);
    let tb = SCRATCH + 0x1000;
    let objs = SCRATCH + 0x1100;
    let (o1, o2) = (SCRATCH + 0x1200, SCRATCH + 0x1400);
    // (state, timer, targets, flag bit of *G_F, answer, the second
    // object flagged)
    let cases: [(u32, u32, u32, u8, u32, bool); 26] = [
        (0, 3, 2, 0, 1, false),
        (0x3c, 0x1d, 2, 0, 1, false),
        (0x3c, 0x1e, 2, 0, 1, false),
        (0x3d, 3, 2, 0, 1, false),
        (0xa, 3, 2, 0, 1, false),
        (0x14, 3, 2, 0, 1, false),
        (0x15, 4, 2, 0, 1, false),
        (0x15, 5, 2, 0, 1, true),
        (0x15, 5, 0, 0, 1, false),
        (0x1e, 3, 2, 0, 1, false),
        (0x1e, 3, 2, 1, 1, false),
        (0x1f, 3, 2, 0, 0, false),
        (0x1f, 3, 2, 0, 0x20, false),
        (0x3e, 3, 2, 0, 0, false),
        (0x3e, 3, 2, 1, 0x40, false),
        (0x3e, 3, 2, 0, 0x40, false),
        (0xb, 3, 2, 1, 0, false),
        (0xb, 3, 2, 0, 0x40, false),
        (0x16, 3, 2, 0, 0, false),
        (0x16, 3, 2, 1, 0x40, false),
        (0x20, 3, 2, 1, 0, false),
        (0x20, 3, 2, 0, 0x40, false),
        (0x2710, 3, 2, 0, 1, false),
        (0x40, 3, 2, 0, 1, false),
        (0x2711, 3, 2, 0, 1, false),
        (0x12, 3, 2, 0, 1, false),
    ];
    for (k, (state, timer, n, bit, answer, flagged)) in (0u32..).zip(cases) {
        junk(&mut t, tb, 0x40, k);
        t.set_u32(tb + 0x18, state);
        t.set_u32(tb + 0x2c, timer);
        t.set_u32(tb + 0x30, n);
        t.set_u32(tb + 0x34, objs);
        t.set_u32(objs, o1);
        t.set_u32(objs + 4, o2);
        for (o, fl) in [(o1, false), (o2, flagged)] {
            junk(&mut t, o, 0x100, k + o);
            t.set_u16(o + 0x90, 0x0f00 | u16::from(fl));
        }
        t.set_u32(ok, answer);
        for o in [0x4f, 0x5a2] {
            t.set_u8(w(o), bit | 0x40);
        }
        let task = task_at(&mut t, tb);
        // case 6's vector, never written with the flag set: 0 on the
        // original's frame too
        let zero: &[u32] = if bit == 1 {
            &[0x1c, 0x18, 0x14, 0x10]
        } else {
            &[]
        };
        ownb::dirty_stack(&mut t, 0x7a60 + k, zero);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    // each slot reached (the original's calls)
    for (s, n) in [
        (366u32, 1u32),
        (239, 2),
        (388, 2),
        (531, 1),
        (20, 4),
        (411, 4),
    ] {
        let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * s);
        assert!(c >= n, "slot {s}: {c} calls");
    }
}

/// `magic_0273` 0x100081e0 (`r43::start_task_0273`), the globals primed:
/// every case in a run's order: 0 (three model nodes, two of them among
/// the targets), 0xa, 0xb with the model id 0 and not, 0x14, 0x15 below 5,
/// with the two kept nodes and none, 0x1e with the flag clear and set
/// (the zeroed vector), 0x1f with the model id 0 and not, 0xc and 0x20
/// with the answer 0 and not and the flag clear and set (dispatched again:
/// 0x14, 0x2710), 0x16 (dispatched to 0x1e), 0x2710 alone; states no case
/// takes.
#[test]
fn start_task_0273() {
    let Some(mut t) = sa::twin(273) else { return };
    let f = t.at(0x81e0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x5d0);
    let pn = t.img.rel32_target(pn + 0x35);
    node_slots(&mut t, pn);
    summon_slots(&mut t);
    let ok = SCRATCH + 0x7e80;
    for s in [20u32, 411] {
        rec(&mut t, s, 1, move |m, _| m.r32(ok));
    }
    for (s, n) in [
        (228u32, 2usize),
        (248, 2),
        (666, 2),
        (276, 2),
        (548, 1),
        (538, 2),
        (36, 2),
        (719, 3),
        (858, 5),
        (859, 1),
        (327, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    rec(
        &mut t,
        219,
        2,
        |_, a| if a[1] == 0 { 0 } else { SCRATCH + 0x7d00 },
    );
    // the summon nodes' targets (288) and the start's own list (218)
    let (la, lb) = (SCRATCH + 0x7e90, SCRATCH + 0x7ea0);
    t.write(la, &[4, 5, 6, 0]);
    t.write(lb, &[4, 9, 6, 0]);
    let nb = SCRATCH + 0x7eb0;
    let na = SCRATCH + 0x7eb4;
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(na));
        la
    });
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(nb));
        lb
    });
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    for l in [0x102u32, 0x1ca, 0x3d6, 0x430, 0x552, 0x243] {
        empty_list(&mut t, w(l));
    }
    let pnw = t.img.rel32_target(f + 0x4c0);
    let l = t.img.word(pnw + 0x26);
    empty_list(&mut t, l);
    // the centre's target count and bytes
    let centre = t.img.rel32_target(f + 0x282);
    t.set_u32(t.img.word(centre + 0x48), 2);
    t.write(t.img.word(centre + 0x55), &[3, 8]);
    let tb = SCRATCH + 0x1000;
    junk(&mut t, tb, 0x400, 1);
    // the nodes' visit handlers (`+0x1c` of each vtable the start
    // writes) and the bone's function table: traps, so that both sides
    // log them (the original would run the DLL's own code there)
    sa::handler_stubs(&mut t);
    let bone = t.img.rel32_target(f + 0x326);
    let plain = t.img.rel32_target(bone + 0xf9);
    let vts = [
        w(0x118),
        w(0x1e1),
        w(0x27d),
        w(0x2a1),
        t.img.word(bone + 0x99),
        t.img.word(t.img.rel32_target(f + 0x1fb) + 0x5),
    ];
    for vt in vts {
        t.set_u32(vt + 0x1c, magic::TRAPS + 16 * sa::HANDLER_SLOT);
    }
    let table = t.img.word(plain + 0xed);
    t.set_u32(table, magic::TRAPS + 16 * sa::HANDLER_SLOT_2);
    let nodes = w(0x15b);
    // (state, timer, kept, flag bit of *G_F, answer, 218's count)
    let cases: [(u32, u32, u32, u8, u32, u32); 23] = [
        (0, 3, 0, 0, 1, 3),
        (0xa, 3, 2, 0, 1, 3),
        (0xb, 3, 2, 0, 0, 3),
        (0xb, 3, 2, 0, 0x20, 3),
        (0x14, 3, 2, 0, 1, 3),
        (0x15, 4, 2, 0, 1, 3),
        (0x15, 5, 2, 0, 1, 3),
        (0x15, 5, 0, 0, 1, 3),
        (0x1e, 3, 2, 0, 1, 3),
        (0x1e, 3, 2, 1, 1, 3),
        (0x1f, 3, 2, 0, 0, 3),
        (0x1f, 3, 2, 0, 0x20, 3),
        (0xc, 3, 2, 0, 0, 3),
        (0xc, 3, 2, 1, 0x40, 3),
        (0xc, 3, 2, 0, 0x40, 3),
        (0x16, 3, 2, 0, 1, 3),
        (0x20, 3, 2, 1, 0, 3),
        (0x20, 3, 2, 0, 0x40, 3),
        (0x2710, 3, 2, 0, 1, 3),
        (0x21, 3, 2, 0, 1, 3),
        (0x2711, 3, 2, 0, 1, 3),
        (0x5, 3, 2, 0, 1, 3),
        (0, 3, 0, 0, 1, 0),
    ];
    for (k, (state, timer, kept, bit, answer, nt)) in (0u32..).zip(cases) {
        t.set_u32(tb + 0x18, state);
        t.set_u32(tb + 0x2c, timer);
        if state != 0 {
            t.set_u32(tb + 0x30, kept);
            t.set_u32(tb + 0x208, nodes);
            t.set_u32(tb + 0x20c, nodes + 0x98 * 2);
        }
        t.set_u32(ok, answer);
        t.set_u32(nb, nt);
        // the summon nodes' count: none in the last case
        t.set_u32(na, if k == 22 { 0 } else { 3 });
        t.set_u8(w(0x4f), bit | 0x40);
        let task = task_at(&mut t, tb);
        // case 7's vector, never written with the flag set: 0 on the
        // original's frame too
        let zero: &[u32] = if bit == 1 {
            &[0x24, 0x20, 0x1c, 0x18]
        } else {
            &[]
        };
        ownb::dirty_stack(&mut t, 0x81e0 + k, zero);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    // each slot reached (the original's calls)
    for (s, n) in [
        (531u32, 1u32),
        (20, 2),
        (411, 2),
        (218, 2),
        (36, 1),
        (228, 1),
    ] {
        let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * s);
        assert!(c >= n, "slot {s}: {c} calls");
    }
}

/// `magic_0388` 0x100068b0 (`r43::drift_task_0388`): every state in a
/// run's order with the timer above 0 and at 0 (0 dispatched to 0x14,
/// 0x15 to 0xa and on), 0xb below 0x80 and at it, 0xd above 0 and at
/// it, 0x2710; states between the cases, past the table and past 0x2710.
#[test]
fn drift_task_0388() {
    let Some(mut t) = sa::twin(388) else { return };
    let f = t.at(0x68b0);
    all_stubs(&mut t);
    let tb = SCRATCH + 0x1000;
    let m = SCRATCH + 0x1100;
    let (a, b) = (SCRATCH + 0x1200, SCRATCH + 0x1300);
    // (state, timer, T+0x20)
    let cases: [(u32, u32, u32); 16] = [
        (0, 3, 0),
        (0x15, 2, 0),
        (0x15, 0, 0),
        (0xb, 5, 0x10),
        (0xb, 5, 0x7e),
        (0xc, 4, 0x20),
        (0xc, 0, 0x20),
        (0xd, 4, 0x20),
        (0xd, 4, 3),
        (0xe, 4, 3),
        (0x2710, 4, 3),
        (5, 4, 3),
        (0x11, 4, 3),
        (0x40, 4, 3),
        (0x2711, 4, 3),
        (0x14, 0, 3),
    ];
    for (k, (state, timer, n)) in (0u32..).zip(cases) {
        junk(&mut t, tb, 0x40, k);
        floats(&mut t, tb + 0x24, 2, k);
        t.set_u32(tb + 0x18, state);
        t.set_u32(tb + 0x1c, timer);
        t.set_u32(tb + 0x20, n);
        t.set_u32(tb + 0x2c, m);
        t.set_u32(m + 0x13c, a);
        t.set_u32(m + 0x144, b);
        floats(&mut t, a + 0x1c, 3, k + 1);
        t.set_u32(b + 0x20, 0x100 + k);
        let task = task_at(&mut t, tb);
        ownb::dirty_stack(&mut t, 0x68b0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0388` 0x10006dd0 (`r43::colour_task_0388`), the globals primed:
/// 0 (the four colour objects), 1 and 2 before their timer and at it, 0xa
/// before its timer, with its object not free (and a negative index) and
/// free, 0x2710, states no case takes. Slot 237 writes the vector, 307
/// steps its ST0, 324 answers numbers of both signs.
#[test]
fn colour_task_0388() {
    let Some(mut t) = sa::twin(388) else { return };
    let f = t.at(0x6dd0);
    all_stubs(&mut t);
    let cs = t.img.rel32_target(f + 0x127);
    let pn = t.img.rel32_target(cs + 0x1ba);
    node_slots(&mut t, pn);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, (a[0] & 0xf) as f32 * 0.5 + i as f32 - 2.0);
        }
        0x237
    });
    let k307 = SCRATCH + 0x7e40;
    rec(&mut t, 307, 0, move |m, _| {
        let c = m.r32(k307);
        m.w32(k307, c + 1);
        0x307 + c
    });
    t.stub_st0(307, move |m, _| 0.375 + f64::from(m.r32(k307) % 5) * 0.25);
    let r324 = SCRATCH + 0x7e50;
    rec(&mut t, 324, 0, move |m, _| m.r32(r324));
    rec(&mut t, 234, 3, |_, a| 0x0234_0000 ^ a[0]);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let tb = SCRATCH + 0x8000;
    junk(&mut t, tb, 0x2400, 2);
    let m = SCRATCH + 0x1100;
    t.set_u32(tb + 0x24, m);
    t.set_u32(m + 0x54, 0x0054_0001);
    // (state, timer, index, 324's answer)
    let cases: [(u32, u32, u32, u32); 14] = [
        (0, 3, 0, 0),
        (1, 3, 4, 0),
        (1, 0, 4, 0),
        (2, 2, 5, 0),
        (2, 0, 5, 0xffff_fff3),
        (0xa, 2, 6, 0),
        (0xa, 0, 6, 0),
        (0xa, 0, 0xffff_ffff, 0),
        (0xa, 0, 13, 0x1d),
        (0xa, 0, 13, 0x8000_0005),
        (0x2710, 0, 13, 0),
        (5, 4, 13, 0),
        (0x2711, 4, 13, 0),
        (2, 0, 3, 0x41),
    ];
    for (k, (state, timer, index, r)) in (0u32..).zip(cases) {
        t.set_u32(tb + 0x18, state);
        t.set_u32(tb + 0x1c, timer);
        t.set_u32(tb + 0x20, index);
        t.set_u32(r324, r);
        // object 1 (index 13) free
        t.set_u32(tb + 0xc0 + 0x2c0 + 0x18, if k == 9 { 7 } else { 0x4e20 });
        if state == 2 {
            // the words case 2 stores (the root's flags, rate, count)
            junk(&mut t, tb + 0x74, 0x10, k);
        }
        let task = task_at(&mut t, tb);
        ownb::dirty_stack(&mut t, 0x6dd0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    // each slot reached (the original's calls)
    for (s, n) in [(324u32, 3u32), (237, 5), (234, 2)] {
        let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * s);
        assert!(c >= n, "slot {s}: {c} calls");
    }
}

/// `magic_0388` 0x10007770 (`r43::summon_task_0388`), the globals primed:
/// every state in a run's order, before its timer and at it (0 set up and
/// dispatched to 0x14; 0x14 to 0xa; 0xc to 0x1e; 0x20 to 0x2710 not
/// ready), 0x2710 ready, states no case takes (the byte table's, between
/// the cases, past them).
#[test]
fn summon_task_0388() {
    let Some(mut t) = sa::twin(388) else { return };
    let f = t.at(0x7770);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x54f);
    node_slots(&mut t, pn);
    summon_slots(&mut t);
    for s in [15u32, 16, 17, 277] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| f64::from(a[0] & 7) * 0.25 + f64::from(s & 3));
    }
    rec(&mut t, 7, 1, |_, a| 0x0f37 ^ a[0]);
    for (s, n) in [(8u32, 2usize), (211, 2), (290, 2), (234, 3), (325, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    for o in [0x88u32, 0x5f3, 0x692, 0x6c2, 0x4f8] {
        empty_list(&mut t, w(o));
    }
    let tb = SCRATCH + 0x8000;
    junk(&mut t, tb, 0x2d00, 3);
    t.set_u32(tb + 0x2288, 0x0228_8001);
    // the task ring's flags (bit 0 set: the start clears it)
    t.set_u32(w(0x3c4), 0x5a5a_5a5b);
    // (state, timer)
    let cases: [(u32, u32); 21] = [
        (0, 0),
        (0x14, 3),
        (0x14, 0),
        (0xa, 2),
        (0xa, 0),
        (0xb, 1),
        (0xb, 0),
        (0xc, 1),
        (0xc, 0),
        (0x1f, 0),
        (0x20, 2),
        (0x20, 0),
        (0x2710, 0),
        (0x2710, 0),
        (5, 3),
        (0x10, 3),
        (0x17, 3),
        (0x1e, 3),
        (0x30, 3),
        (0x2711, 3),
        (0xb, 0xffff_ffff),
    ];
    for (k, (state, timer)) in (0u32..).zip(cases) {
        t.set_u32(tb + 0x18, state);
        t.set_u32(tb + 0x1c, timer);
        if k == 13 {
            // the three sub-tasks done
            for o in [0x21f8u32, 0x38, 0x221c] {
                t.set_u32(tb + o, 0x4e20);
            }
        }
        let task = task_at(&mut t, tb);
        ownb::dirty_stack(&mut t, 0x7770 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(8u32, 2u32), (325, 2), (211, 1), (7, 1)] {
        let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * s);
        assert!(c >= n, "slot {s}: {c} calls");
    }
}

/// The load step at `load` in its four cases (busy, asked, loaded and
/// not), each after `setup`.
fn load_cases(t: &mut Twin, load: u32, setup: &dyn Fn(&mut Twin)) {
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        setup(t);
        rec(t, 225, 1, move |_, _| busy);
        rec(t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(t, load, &[0x61, 0x6b]);
        magic_h6::scrub(t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// The record start's slots (`r41::record_start_0250`): 217 and 234 write
/// vectors from what they get, 233 scales.
fn record_slots(t: &mut Twin) {
    for s in [217u32, 234] {
        rec(t, s, if s == 217 { 2 } else { 3 }, move |m, a| {
            let (dst, src) = if s == 217 {
                (a[1], a[0])
            } else {
                (a[0], a[1] ^ a[2])
            };
            for i in 0..4 {
                m.w32(dst + 4 * i, src.rotate_left(i + s) | 1);
            }
            s
        });
    }
    rec(t, 233, 3, |m, a| {
        for i in 0..4u32 {
            let v = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]) + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x233
    });
}

/// What `r43::start_up_0250` needs: the pools (its own and the record
/// start's free ring), the object start's slots, the actors (`n` of them,
/// their bytes and characters), slots 531/262/268 writing their vectors,
/// slot 333's yaw `yaw`, slot 599's answers, the task list empty or not.
fn start_up_0250_setup(t: &mut Twin, f: u32, k: u32, n: u32, yaw: f64, listed: bool) {
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(t);
    magic_r11::pools(t, f, 1705);
    magic_r11::object_slots(t);
    magic_r11::object_start_auto(t, t.img.rel32_target(f + 0x71));
    record_slots(t);
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xff) as f32 * 0.5 + i as f32);
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, (a[0] & 0xf) as f32 - i as f32 * 0.75);
        }
        0x268
    });
    rec(t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, (a[0] & 7) as f32 * 1.5 + i as f32);
        }
        0x531
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, a| yaw + f64::from(a[0] & 3));
    rec(t, 599, 1, |_, a| a[0] * 3 + 1);
    for (s, m) in [(600u32, 2usize), (215, 1), (413, 1), (711, 3), (710, 4)] {
        rec(t, s, m, move |_, a| (s << 16) ^ a[0]);
    }
    // the task pool's free ring (the record starts, the start's three
    // nodes and the two pool resets take from it) and the busy list
    magic_r11::ring(t, w(0x398), w(0x415), 12, SCRATCH + 0x5000);
    let rs = t.img.rel32_target(f + 0x132);
    let busy = SCRATCH + 0x1400;
    t.write(busy, &[0x66; 0x20]);
    t.set_u32(t.img.word(rs + 0x246), 0);
    t.set_u32(t.img.word(rs + 0x25f), busy);
    // the actors: their count, bytes and characters
    let sel = SCRATCH + 0x180;
    t.write(sel, &[3, 1, 0x13, 2]);
    for o in [0x1de, 0x23d, 0x2b0, 0x311] {
        t.set_u32(w(o), sel);
    }
    // the character table (the operands name the table itself)
    for o in [0x1e9, 0x248] {
        for i in 0..0x14 {
            t.set_u32(w(o) + 4 * i, 0x0000_0c00 + i);
        }
    }
    // the words the start stores whole or in part (zeros, u16s), junk
    junk_ops(
        t,
        f,
        &[
            0x3a0, 0x3aa, 0x3b4, 0x3be, 0x3dc, 0x3e6, 0x3f0, 0x400, 0x514, 0x528, 0x531,
        ],
    );
    t.set_u32(w(0x18f), n);
    t.set_u32(w(0x81), 0x0000_0081 + k);
    t.set_u32(w(0x4af), 0x0000_0045 + k);
    t.set_u32(w(0x31b), k & 1);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(0x477), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(0x48d), SCRATCH + 0x600);
}

/// `magic_0250` 0x10004260 (`r43::start_up_0250`, internal) and its load
/// step 0x10004130 (`r43::load_0250`): no actors, one and three; the
/// caster's yaw kept and wrapped both ways; the model flag both ways; the
/// task list empty and not.
#[test]
fn start_up_0250() {
    let Some(mut t) = twin(250) else { return };
    let f = t.at(0x4260);
    for (k, (n, yaw, listed)) in (0u32..).zip([
        (0u32, 0.25f64, false),
        (1, 9.5, true),
        (3, -7.0, false),
        (2, 3.0, true),
    ]) {
        start_up_0250_setup(&mut t, f, k, n, yaw, listed);
        ownb::dirty_stack(&mut t, 0x4260 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r43::start_up_0250,
        );
    }
    // the actors' loop reached
    let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * 268);
    assert!(c >= 3, "slot 268: {c} calls");
    let load = t.at(0x4130);
    load_cases(&mut t, load, &|t| {
        start_up_0250_setup(t, f, 2, 1, 1.0, true);
    });
}

/// `magic_0250` 0x100052c0 (`r43::caster_record_0250`): the caster's
/// record started from slot 531's place (the record start's free ring of
/// one and of three nodes).
#[test]
fn caster_record_0250() {
    let Some(mut t) = twin(250) else { return };
    let f = t.at(0x52c0);
    all_stubs(&mut t);
    record_slots(&mut t);
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, (a[0] & 7) as f32 * 1.5 + i as f32);
        }
        0x531
    });
    let rs = t.img.rel32_target(f + 0x53);
    let sel = SCRATCH + 0x180;
    t.write(sel, &[4, 9, 0x13, 2]);
    t.set_u32(t.img.word(rs + 0x2b), sel);
    t.set_u32(t.img.word(rs + 0x24), 0x00a1_0000);
    for c in 0..2u32 {
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        for i in 0..3 {
            t.write(node(i), &[0x77; 0x20]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(t.img.word(rs + 0x1df), node(1));
        t.set_u16(t.img.word(rs + 0x193), if c == 0 { 3 } else { 1 });
        let busy = SCRATCH + 0x1400;
        t.write(busy, &[0x66; 0x20]);
        t.set_u32(t.img.word(rs + 0x246), 0);
        t.set_u32(t.img.word(rs + 0x25f), busy);
        t.set_u32(t.img.word(f + 0x1b), 0x0000_0071 + c);
        junk_ops(&mut t, f, &[0x61, 0x6b]);
        ownb::dirty_stack(&mut t, 0x52c0 + c, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0250` 0x10004af0 (`r43::spawn_0250`): the record past 18 (the
/// cookie word); every spark placed (pool `B` with 8 records, two of them
/// taken; pool `A` with 6, one taken, the rover wrapping); `B` holding
/// only one free record (the second take fails: 0); `A` full and empty
/// (the index); `B` running out in the last phase; the yaw wrapped both
/// ways. Matrix and vector slots write what they read.
#[test]
fn spawn_0250() {
    spawn(250, 0x4af0);
}

/// `magic_0251` 0x10005490 (`r43::spawn_0250` through `r45::SPAWN_0251`):
/// the same cases on `magic_0251`'s copy.
#[test]
fn spawn_0251() {
    spawn(251, 0x5490);
}

fn spawn(n: u16, at: u32) {
    let Some(mut t) = sa::twin(n) else { return };
    let f = t.at(at);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, (a[0] & 0xf) as f32 * 0.5 + i as f32);
        }
        0x217
    });
    let yaw = SCRATCH + 0x7e00;
    rec(&mut t, 601, 2, move |m, a| {
        m.w32(a[1], m.r32(yaw));
        0x601
    });
    let k314 = SCRATCH + 0x7e10;
    rec(&mut t, 314, 1, move |m, a| {
        let c = m.r32(k314);
        m.w32(k314, c + 1);
        0x314 ^ a[0]
    });
    t.stub_st0(314, move |m, a| {
        f64::from(f32::from_bits(a[0])) * 0.125 * f64::from(m.r32(k314) % 7)
    });
    rec(&mut t, 404, 1, |_, a| 0x404 ^ a[0]);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 1.5);
    rec(&mut t, 235, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * (i % 3) as f32;
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
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x248
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    let take = t.img.rel32_target(f + 0xa5);
    let (bb, ba) = (SCRATCH + 0x2000, SCRATCH + 0x3000);
    let s = SCRATCH + 0x900;
    // (S+0xa4, B records, B taken, A records, A taken, A rover, yaw)
    let cases: [(u32, u32, &[u32], u32, &[u32], u32, f32); 7] = [
        (0x13, 8, &[], 6, &[], 0, 0.5),
        (2, 8, &[1, 4], 6, &[2], 4, 0.5),
        (2, 3, &[1, 2], 6, &[], 0, -7.0),
        (2, 8, &[], 4, &[0, 1, 2, 3], 1, 9.0),
        (2, 8, &[], 0, &[], 0, 0.5),
        (2, 5, &[], 6, &[], 5, 0.5),
        (0x12, 8, &[0], 6, &[5], 2, 3.0),
    ];
    for (k, (busy, nb, takenb, na, takena, rover, y)) in (0u32..).zip(cases) {
        junk(&mut t, s, 0xc0, k);
        t.set_u32(s + 0xa4, busy);
        t.set_u32(s + 0xa0, 0x00a0_0001 + k);
        t.set_u32(yaw, y.to_bits());
        junk(&mut t, bb, 0x50 * 8, k);
        for i in 0..8 {
            t.set_u32(bb + 0x50 * i + 0x48, u32::from(takenb.contains(&i)));
        }
        junk(&mut t, ba, 0x68 * 6, k + 1);
        for i in 0..6 {
            t.set_u32(ba + 0x68 * i + 0x60, u32::from(takena.contains(&i)));
        }
        let wt = |o: u32| t.img.word(take + o);
        let (ti, tn, tb) = (wt(0x01), wt(0x09), wt(0x16));
        // the ring take's pool is the last phase's
        assert_eq!((ti, tn, tb), (w(0x537), w(0x532), w(0x547)));
        t.set_u32(ti, 0);
        t.set_u32(tn, nb);
        t.set_u32(tb, bb);
        t.set_u32(w(0x288), rover);
        t.set_u32(w(0x283), na);
        t.set_u32(w(0x298), ba);
        ownb::dirty_stack(&mut t, 0x4af0 + k, &[]);
        t.check_entry(f, Call::default(), &[s], Eax::Checked);
    }
}

/// What `magic_0420`'s set-up needs: the zero globals primed, its node
/// parents (`P` and the second one) empty lists or holding a node, the
/// particle node's slots, slot 215's model, 12's height, 719's handles.
fn setup_0420_stubs(t: &mut Twin, f: u32) {
    all_stubs(t);
    let pn = t.img.rel32_target(f + 0x1af);
    node_slots(t, pn);
    rec(t, 12, 1, |_, a| 0x12 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 237, 4, |_, a| 0x237 ^ a[0]);
    rec(t, 306, 3, |_, a| 0x306 ^ a[2]);
    // the throw aim's square root (an import)
    summon_stubs::prime(t, f, SCRATCH + 0x4_0000);
}

/// Junk in the words `setup_0420` stores and the parents' links.
fn setup_0420_state(t: &mut Twin, f: u32, k: u32, held: bool) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let p = w(t, 0x45);
    empty_list(t, p);
    let q = w(t, 0x51f);
    empty_list(t, q);
    if held {
        let last = SCRATCH + 0x6100;
        t.write(last, &[0x6c; 0x20]);
        t.set_u32(p + 0x10, last);
        t.set_u32(q + 0x10, last);
    }
    // the task ring's last node and flags
    let ring_last = SCRATCH + 0x6200;
    t.write(ring_last, &[0x6d; 0x20]);
    t.set_u32(w(t, 0x545), ring_last);
    t.set_u32(w(t, 0x5a7), 0x5a5a_5a5b + k);
    junk_ops(
        t,
        f,
        &[
            0x36c, 0x376, 0x3d9, 0x5bd, 0x606, 0x654, 0x65e, 0x5ff, 0x64d,
        ],
    );
    for (o, n) in [(0x23a, 8u32), (0x250, 8)] {
        junk(t, w(t, o), n, k + o);
    }
    t.set_u32(w(t, 0x75), 0x0075_0000 + k);
    t.set_u8(w(t, 0x443), 3 + k as u8);
    t.set_u16(w(t, 0x240), 5 + k as u16);
}

/// `magic_0420` 0x100069d0 (`r43::setup_0420`, internal): the parents
/// empty and holding a node.
#[test]
fn setup_0420() {
    let Some(mut t) = sa::twin(420) else { return };
    let f = t.at(0x69d0);
    setup_0420_stubs(&mut t, f);
    for (k, held) in [false, true].into_iter().enumerate() {
        setup_0420_state(&mut t, f, k as u32, held);
        ownb::dirty_stack(&mut t, 0x69d0 + k as u32, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r43::setup_0420(g, img, fr)
        });
    }
}

/// `magic_0420` 0x10007040 (`r43::task_0420`): states 0 to 4 (2 and 4
/// waiting and answered, 3 before its timer, at it and with a NaN), a
/// state past the table.
#[test]
fn task_0420() {
    let Some(mut t) = sa::twin(420) else { return };
    let f = t.at(0x7040);
    let setup = t.img.rel32_target(f + 0x90);
    setup_0420_stubs(&mut t, setup);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let ok = SCRATCH + 0x7e80;
    rec(&mut t, 240, 1, move |m, _| m.r32(ok));
    for (s, n) in [(228u32, 2usize), (213, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0] ^ a[1]);
    }
    let k_at = w(0xaf);
    let kk = f32::from_bits(t.u32(k_at));
    // (state, answer, timer)
    let cases: [(u16, u32, f32); 11] = [
        (0, 0, 0.0),
        (1, 0, 0.0),
        (2, 0, 0.0),
        (2, 1, 0.0),
        (3, 0, kk - 1.0),
        (3, 0, kk),
        (3, 0, f32::NAN),
        (4, 0, 0.0),
        (4, 1, 0.0),
        (5, 0, 0.0),
        (0x20, 0, 0.0),
    ];
    for (k, (state, answer, timer)) in (0u32..).zip(cases) {
        setup_0420_state(&mut t, setup, k, k & 1 == 1);
        t.set_u16(w(0x5), state);
        // the halfword after the u16 state
        t.set_u16(w(0x5) + 2, 0x5a5a);
        t.set_u32(ok, answer);
        setf(&mut t, w(0xa8), timer);
        t.set_u32(w(0x30), 0x0030_0000 + k);
        let task = task_at(&mut t, SCRATCH + 0x1000);
        ownb::dirty_stack(&mut t, 0x7040 + k, &[]);
        let call = Call {
            eax: 0x5a5a_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[task], Eax::Checked);
    }
}

/// `magic_0420` 0x100066e0 (`r43::object_task_0420`): the clock below and
/// at its mark (and NaN); state 0 idle and going on, 1 (the set-up, the
/// parent empty and holding a node), 2 idle, counting and ending (the
/// byte wrapping to 0), a negative state and one past the cases.
#[test]
fn object_task_0420() {
    let Some(mut t) = sa::twin(420) else { return };
    let f = t.at(0x66e0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x208);
    node_slots(&mut t, pn);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
    for (s, n) in [(306u32, 3usize), (239, 3), (779, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let p = w(0xc6);
    let tb = SCRATCH + 0x8000;
    let mark = f32::from_bits(t.u32(w(0x2a)));
    // (state, clock, +0x524, +0x520, byte +0x2e3, parent holds a node)
    let cases: [(i16, f32, u16, u16, u8, bool); 10] = [
        (0, 0.0, 0, 0, 0, false),
        (0, 0.0, 3, 0, 0, false),
        (1, 0.0, 0, 0, 0, true),
        (2, 0.0, 0, 0, 0x20, false),
        (2, 0.0, 0, 1, 0x20, false),
        (2, 0.0, 0, 1, 0x10, false),
        (1, mark, 0, 1, 0x10, false),
        (1, f32::NAN, 0, 0, 0, false),
        (-3, 0.0, 0, 0, 0, false),
        (7, 0.0, 0, 0, 0, false),
    ];
    for (k, (state, clock, b524, b520, byte, held)) in (0u32..).zip(cases) {
        junk(&mut t, tb, 0x800, k);
        t.set_u16(tb + 0x538, state.cast_unsigned());
        // the clock: K added before the mark is compared
        let step = f32::from_bits(t.u32(w(0x23)));
        setf(&mut t, tb + 0x7e4, clock - step);
        t.set_u16(tb + 0x524, b524);
        t.set_u16(tb + 0x520, b520);
        t.set_u8(tb + 0x2e3, byte);
        // the node `T + 0x2bc` the end unlinks: under a parent of its own
        let n = tb + 0x2bc;
        let par = SCRATCH + 0x6300;
        empty_list(&mut t, par);
        t.set_u32(n + 0x14, par);
        t.set_u32(n + 4, par);
        t.set_u32(n + 8, par);
        t.set_u32(par + 0xc, n);
        t.set_u32(par + 0x10, n);
        t.set_u16(par + 0x18, 1);
        for o in [0x1eb, 0x272, 0x1b1] {
            empty_list(&mut t, w(o));
        }
        empty_list(&mut t, p);
        if held {
            let last = SCRATCH + 0x6100;
            t.write(last, &[0x6c; 0x20]);
            t.set_u32(p + 0x10, last);
        }
        junk_ops(&mut t, f, &[0x2b6, 0x2c5]);
        let task = task_at(&mut t, tb);
        ownb::dirty_stack(&mut t, 0x66e0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0420` 0x100071a0 (`r43::start_0420`): the start with the target
/// placed so that the yaw takes each of its nine paths (both products'
/// signs and quadrants, the zero cases). Slot 217 places a handle at
/// `(lo nibble - 7, 1, hi nibble - 7)`, slot 234 subtracts.
#[test]
fn start_0420() {
    let Some(mut t) = sa::twin(420) else { return };
    let f = t.at(0x71a0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    let list = SCRATCH + 0x7e00;
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        list
    });
    rec(&mut t, 531, 2, |m, a| {
        m.w32(a[1], a[0]);
        0x531
    });
    t.stub_st0(289, |_, _| 2.5);
    for s in [413u32, 215, 14, 333] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(333, |_, _| 0.75);
    rec(&mut t, 217, 2, |m, a| {
        let x = (a[0] & 0xf) as f32 - 7.0;
        let z = ((a[0] >> 4) & 0xf) as f32 - 7.0;
        for (i, v) in (0u32..).zip([x, 1.0, z, 1.0]) {
            wf(m, a[1] + 4 * i, v);
        }
        0x217
    });
    for (s, n) in [
        (268u32, 3usize),
        (666, 2),
        (718, 1),
        (719, 3),
        (710, 4),
        (222, 4),
        (221, 4),
        (551, 3),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let lock = SCRATCH + 0x7f80;
    t.set_u32(w(0x731), lock);
    // the lock word while the program starts
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f84, m.r32(lock));
        a[0] ^ 0x711
    });
    for (k, target) in (0u32..).zip([0x99u8, 0x55, 0x95, 0x59, 0x7a, 0x74, 0x97, 0x57, 0x77]) {
        t.write(list, &[target, 0x81]);
        t.set_u32(lock, 3 + k);
        // the task ring's last node
        let tail = SCRATCH + 0x6200;
        t.write(tail, &[0x6d; 0x20]);
        t.set_u32(w(0x7e8), tail);
        t.set_u32(w(0x829), 0x5a5a_5a5b + k);
        // a byte the start clears
        t.set_u8(w(0x18d), 0x5a);
        ownb::dirty_stack(&mut t, 0x71a0 + k, &[]);
        let call = Call {
            eax: 0x5a5a_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[0x77], Eax::Checked);
    }
}

/// `magic_0394`'s slots: 262 a position from its first word, 12 and 333
/// floats in ST0, 314/404 randoms in ST0 that step, the matrix slots
/// writing what they read.
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
    // the scaled matrix's scales
    for s in [277u32, 15, 16, 17] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| 1.0 + f64::from((a[0] ^ s) & 3) * 0.25);
    }
    let k = SCRATCH + 0x7e10;
    for s in [314u32, 404] {
        rec(t, s, 1, move |m, a| {
            let c = m.r32(k);
            m.w32(k, c + 1);
            (s << 16) ^ a[0] ^ c
        });
        t.stub_st0(s, move |m, a| {
            f64::from(f32::from_bits(a[0])) * 0.25 * f64::from(m.r32(k) % 5) - 0.5
        });
    }
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

/// `magic_0394` 0x10004b80 (`r43::landing_0394`): near the mark and far
/// from it.
#[test]
fn landing_0394() {
    let Some(mut t) = sa::twin(394) else { return };
    let f = t.at(0x4b80);
    stubs_0394(&mut t);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    // the state task's pool
    let st = t.img.rel32_target(f + 0x1ad);
    magic_r11::pools(&mut t, st, 150);
    let o = SCRATCH + 0x1000;
    // (near, the mark's x and z): the mark close, far, and at 26-28 off
    // along each axis and both (the distance's terms matter there)
    for (k, (near, mx, mz)) in [
        (true, -1.75f32, 0.25f32),
        (false, 900.0, 0.0),
        (true, -1.75, 0.25),
        (true, -28.0, 0.0),
        (true, 0.0, -28.0),
        (true, -20.0, -20.0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let gs = t.img.word(f + 0x1a);
        let s = block_0394(&mut t, gs, k);
        // a plain yaw
        setf(&mut t, s + 0xc84, 0.5 + k as f32);
        setf(&mut t, s + 0xb40, mx);
        setf(&mut t, s + 0xb44, 0.0);
        setf(&mut t, s + 0xb48, mz);
        junk(&mut t, o, 0x60, k);
        t.set_u32(t.img.word(f + 0x38), 0x0000_0040 + k);
        ownb::dirty_stack(&mut t, 0x4b80 + k, &[]);
        // (the far path's eax is the import's leftover; the last three
        // cases sit near the mark, either side: their memory tells)
        let eax = if near && k < 3 {
            Eax::Checked
        } else {
            Eax::Junk
        };
        t.check_entry(f, Call::default(), &[o], eax);
    }
    // (r45) the dx term: P's x is 2.0 here (slot 262's word 0x45), the
    // mark 29 off in x (27 away; 31 if the term's sign flipped) and its z
    // swept in steps of 10, so one case sits near the mark in z too
    for (k, mz) in (6u32..).zip([
        -70.0f32, -60.0, -50.0, -40.0, -30.0, -20.0, -10.0, 0.0, 10.0,
    ]) {
        let gs = t.img.word(f + 0x1a);
        let s = block_0394(&mut t, gs, k);
        setf(&mut t, s + 0xc84, 5.5);
        setf(&mut t, s + 0xb40, 29.0);
        setf(&mut t, s + 0xb44, 0.0);
        setf(&mut t, s + 0xb48, mz);
        junk(&mut t, o, 0x60, k);
        t.set_u32(t.img.word(f + 0x38), 0x0000_0045);
        ownb::dirty_stack(&mut t, 0x4b80 + k, &[]);
        t.check_entry(f, Call::default(), &[o], Eax::Junk);
    }
}

/// `magic_0394` 0x100053e0 (`r43::turn_step_0394`): the angles wrapping
/// both ways and not, the counter above 10, at 10 (party member, not, a
/// monster flag) and running out (the node given back).
#[test]
fn turn_step_0394() {
    let Some(mut t) = sa::twin(394) else { return };
    let f = t.at(0x53e0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    stubs_0394(&mut t);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let give = t.img.rel32_target(f + 0x251);
    let sel = SCRATCH + 0x180;
    t.set_u32(w(0x1dd), sel);
    t.set_u32(w(0x232), sel);
    for i in 0..0x30 {
        t.set_u32(w(0x213) + 4 * i, 0x0c00 + i);
    }
    let (hi, lo) = (
        f32::from_bits(t.u32(w(0x7))),
        f32::from_bits(t.u32(w(0x17))),
    );
    // (angle, counter, target byte, monster flag)
    let cases: [(f32, u32, u8, u32); 7] = [
        (hi + 1.0, 20, 3, 0),
        (lo - 1.0, 11, 3, 0),
        (0.0, 11, 0x25, 0),
        (0.0, 11, 0x20, 0),
        (0.0, 11, 3, 1),
        (0.0, 1, 3, 0),
        (0.0, 0, 3, 0),
    ];
    for (k, (a, n, b, mon)) in (0u32..).zip(cases) {
        let s = block_0394(&mut t, w(0x1e), k);
        setf(&mut t, s + 0xaf0, a);
        setf(&mut t, s + 0xaf4, -a);
        t.set_u32(s + 0xc90, n);
        t.write(sel, &[b, 0x5a]);
        t.set_u32(w(0x1d7), mon);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x53e0 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0394` 0x10004d60 (`r43::sparks_0394`, internal) and 0x100050d0
/// (`r43::flight_0394`): the pool with room and full after the first or
/// second spark; the flight's counter from 30 down to 0 (the curve, the
/// sparks below 15, the end), placed for a party member and an object
/// (grounded and not).
#[test]
fn flight_0394() {
    let Some(mut t) = sa::twin(394) else { return };
    let f = t.at(0x50d0);
    let sp = t.at(0x4d60);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    stubs_0394(&mut t);
    summon_stubs::prime(&mut t, sp, SCRATCH + 0x4_0000);
    let take = t.img.rel32_target(sp + 0x71);
    let tw = |o: u32| img.word(take + o);
    let (count, base, index, page) = (tw(0x0f), tw(0x37), tw(0x08), tw(0x2b));
    let pool = SCRATCH + 0x5000;
    let grounded = SCRATCH + 0x7e60;
    rec(&mut t, 11, 1, move |m, _| m.r32(grounded));
    // the party member's place, read through its pointer
    rec(&mut t, 255, 2, |m, a| {
        for i in 0..3 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x7e70 + 4 * i, v);
        }
        0x255
    });
    // (counter, records free, battle flag, me, grounded)
    let cases: [(i32, u32, u32, i32, u32); 9] = [
        (30, 8, 0, 3, 0),
        (29, 8, 0, 0x20, 0),
        (16, 8, 1, 3, 0),
        (15, 8, 0, 0x30, 1),
        (14, 1, 0, 3, 0),
        (10, 2, 1, 3, 1),
        (5, 3, 0, 3, 0),
        (1, 8, 0, 3, 0),
        (0, 8, 1, 3, 0),
    ];
    for (k, (n, free, battle, me, gr)) in (0u32..).zip(cases) {
        let s = block_0394(&mut t, w(0x17), k);
        t.set_u32(s + 0xc88, n.cast_unsigned());
        t.set_u32(w(0x1a6), battle);
        t.set_u32(w(0x24f), me.cast_unsigned());
        t.set_u32(grounded, gr);
        junk(&mut t, pool, 0x74 * 8, k);
        for i in 0..8 {
            t.set_u32(pool + 0x74 * i + 0x6c, u32::from(i >= free));
        }
        t.set_u32(count, 8);
        t.set_u32(base, pool);
        t.set_u32(index, 0);
        t.set_u32(page, 0);
        let o = SCRATCH + 0x1000;
        junk(&mut t, o, 0x60, k);
        ownb::dirty_stack(&mut t, 0x50d0 + k, &[]);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
    // the sparks alone, the pool full at once
    t.set_u32(count, 8);
    for i in 0..8 {
        t.set_u32(pool + 0x74 * i + 0x6c, 1);
    }
    ownb::dirty_stack(&mut t, 0x4d60, &[]);
    t.check(sp, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::r43::sparks_0394(g, img, fr)
    });
}

/// `magic_0251` 0x100050e0 (`r43::object_step_0251`): the objects running
/// (an empty list: 1) with flag 0x8000 clear, set with a free record, set
/// with none free (the actors shown again: a party member, a monster and
/// a member past 0x20, a battle), no records at all.
#[test]
fn object_step_0251() {
    let Some(mut t) = sa::twin(251) else { return };
    let f = t.at(0x50e0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    summon_stubs::prime(&mut t, f, SCRATCH + 0x4_0000);
    let ro = t.img.rel32_target(f + 0x34);
    let wr = |o: u32| img.word(ro + o);
    for (s, n) in [
        (296u32, 2usize),
        (257, 2),
        (210, 2),
        (211, 2),
        (217, 2),
        (255, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let list = SCRATCH + 0x180;
    t.write(list, &[3, 0x25, 0x20, 1]);
    t.set_u32(w(0x72), list);
    for i in 0..0x30 {
        t.set_u32(w(0xae) + 4 * i, 0x0c00 + i);
    }
    let recs = w(0x17);
    // the grid's and the part programs' counts (1) and records
    for at in [0xdf, 0xe7] {
        let c = t.img.rel32_target(f + at);
        let (n_at, r_at) = (t.img.word(c + 0x08), t.img.word(c + 0x0f));
        t.set_u32(n_at, 1);
        t.set_u32(r_at, SCRATCH + 0x5000);
    }
    t.write(SCRATCH + 0x5000, &[0x22; 0x80]);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| a[0] ^ 0x77);
    rec(&mut t, 779, 2, |_, a| 0x0779_0000 ^ a[0]);
    // (flag, a record free, records, battle)
    let cases: [(u32, bool, u32, u32); 5] = [
        (0, false, 4, 0),
        (0x8000, true, 4, 0),
        (0x8000, false, 4, 0),
        (0x8000, false, 3, 1),
        (0x8000, false, 0, 0),
    ];
    for (k, (flag, free, n, battle)) in (0u32..).zip(cases) {
        // run_objects: an empty list, its pointer globals
        t.set_u32(wr(0x1a), 0);
        t.set_u32(wr(0x25), SCRATCH + 0x60);
        let l = wr(0x7c);
        t.write(l, &[0; 0x10]);
        t.set_u32(wr(0x14), 0);
        t.set_u32(w(0x7), n);
        for i in 0..4 {
            t.set_u32(recs + 0xdc * i, u32::from(!(free && i == 2)));
        }
        t.set_u32(w(0x43), flag | 0x11);
        t.set_u32(w(0x6b), battle);
        let s = SCRATCH + 0x1000;
        junk(&mut t, s, 0x20, k);
        let s8 = SCRATCH + 0x1100;
        junk(&mut t, s8, 0x20, k + 1);
        t.set_u32(s + 8, s8);
        ownb::dirty_stack(&mut t, 0x50e0 + k, &[]);
        t.check_entry(f, Call::default(), &[s], Eax::Checked);
    }
    // both ways of showing the actors reached
    for (sl, n) in [(296u32, 1u32), (257, 2)] {
        let c = t.u32(SCRATCH + 0x2_0000 + 0x80 * sl);
        assert!(c >= n, "slot {sl}: {c} calls");
    }
}

/// `magic_0385` 0x10004e50 (`r43::weapon_frame_0385`): the matrix slots
/// write (and read back) what they get, slot 277's height in ST0; two
/// characters and heights.
#[test]
fn weapon_frame_0385() {
    let Some(mut t) = sa::twin(385) else { return };
    let f = t.at(0x4e50);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 236, 3, |m, a| {
        for i in 0..16 {
            wf(m, a[2] + 4 * i, (a[1] & 0xf) as f32 * 0.5 + i as f32 - 4.0);
        }
        0x236
    });
    rec(&mut t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
    t.stub_st0(277, |_, a| 0.5 + f64::from(a[0] & 3));
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(&mut t, 330, 3, |m, a| {
        let x = f32::from_bits(a[2]);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + x * (i % 3) as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x330
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    for i in 0..8 {
        t.set_u32(w(0x24) + 4 * i, 0x0c00 + i);
    }
    let s = SCRATCH + 0x1000;
    for (k, (who, c)) in (0u32..).zip([(3u32, 0x41u32), (6, 0x42)]) {
        junk(&mut t, s, 0x140, k);
        t.set_u32(s + 0x120, who);
        t.set_u32(w(0x2f), c);
        t.set_u32(w(0x42), c + 1);
        ownb::dirty_stack(&mut t, 0x4e50 + k, &[]);
        t.check_entry(f, Call::default(), &[s], Eax::Checked);
    }
}

/// `magic_0251` 0x100051d0 (`r43::curve_task_0251`): the delay running;
/// steps left (the parameter grows) with the script waiting; the last
/// step with the script done (the node given back); no steps; the angle
/// wrapping both ways.
#[test]
fn curve_task_0251() {
    let Some(mut t) = sa::twin(251) else { return };
    let f = t.at(0x51d0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x248
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [277u32, 15, 16, 17] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| 1.0 + f64::from((a[0] ^ s) & 3) * 0.25);
    }
    for s in [328u32, 329, 330] {
        rec(&mut t, s, 3, move |m, a| {
            let x = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + x * (i % 3) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    let give = t.img.rel32_target(f + 0x2a9);
    // the actor scale's selection and objects
    let asc = t.img.rel32_target(f + 0x209);
    let sel = SCRATCH + 0x180;
    t.write(sel, &[1, 3, 2, 5, 4, 6, 7, 0]);
    for o in [0x28, 0x74] {
        t.set_u32(t.img.word(asc + o), sel);
    }
    for o in [0x33, 0x7f] {
        for i in 0..8 {
            t.set_u32(t.img.word(asc + o) + 4 * i, 0x0b00 + i);
        }
    }
    let (hi, lo) = (
        f32::from_bits(t.u32(w(0x1d2))),
        f32::from_bits(t.u32(w(0x1e6))),
    );
    let tb = SCRATCH + 0x2000;
    // (delay, steps, script done, angle)
    // (`T + 0xc8` is 0.5: `hi - 0.5` lands on the bound, which stays)
    let cases: [(i32, i32, u32, f32); 7] = [
        (3, 5, 0, 0.0),
        (0, 5, 0, 0.0),
        (0, 1, 1, 0.0),
        (0, 0, 0, 0.0),
        (0, 4, 0, hi + 0.5),
        (0, 4, 1, lo - 1.0),
        (0, 4, 0, hi - 0.5),
    ];
    for (k, (delay, steps, done, angle)) in (0u32..).zip(cases) {
        junk(&mut t, tb, 0xe0, k);
        floats(&mut t, tb + 0x60, 0x18, k);
        t.set_u32(tb + 0xd0, delay.cast_unsigned());
        t.set_u32(tb + 0xcc, steps.cast_unsigned());
        setf(&mut t, tb + 0xc0, 0.125 * k as f32);
        setf(&mut t, tb + 0xc4, 0.25);
        setf(&mut t, tb + 0xc8, 0.5);
        setf(&mut t, tb + 0x60, angle);
        t.set_u32(tb + 0xd4, 2 + k);
        // the motion script: done (1), or waiting (`+0x18` 3)
        t.set_u32(tb + 0x14, done);
        t.set_u16(tb + 0x18, 3);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        t.set_u32(node + 0x14, tb);
        ownb::dirty_stack(&mut t, 0x51d0 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0348` 0x10006580 (`r43::end_task_0348`, cdecl `(node)`): the
/// script running or ended, the motion waiting or done; both done (the
/// lights and models, the node given back).
#[test]
fn end_task_0348() {
    let Some(mut t) = twin(348) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6580);
    let run = t.img.rel32_target(f + 0xc);
    let ms = t.img.rel32_target(f + 0x13);
    let lm = t.img.rel32_target(f + 0x27);
    let give = t.img.rel32_target(f + 0x32);
    for (s, n) in [
        (590u32, 2usize),
        (589, 2),
        (296, 2),
        (941, 1),
        (772, 2),
        (779, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let bytes = SCRATCH + 0x600;
    t.write(bytes, &[3, 5, 7, 9]);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], 2);
        bytes
    });
    // (script mode, motion done)
    for (k, (mode, done)) in (0u32..).zip([(4u8, false), (0, false), (4, true), (0, true)]) {
        let s = SCRATCH + 0x1_0000;
        junk(&mut t, s, 0xd200, k);
        t.set_u32(t.img.word(f + 0x07), s);
        // the lights' block (its own global)
        let e = SCRATCH + 0x3000;
        t.set_u32(t.img.word(lm + 0x6), e - t.img.word(lm + 0xe));
        t.write(e, &[0x5a; 0x240]);
        t.set_u32(e + 0x228, 0);
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u32(t.img.word(run + 0x07), 0x0bad_0007);
        magic_hi::motion(&mut t, ms, done);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x6580 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}
