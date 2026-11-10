//! Round 43's own ports (`ffx_magic::r43`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
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
    t.set_u32(w(0x4f1), 3);
    for (i, id) in [7u32, 2, 0x31].into_iter().enumerate() {
        t.set_u32(objs + 0x98 * i as u32, id);
    }
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
        (0xa, 3, 0, 3, 0, 0, 0),
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
