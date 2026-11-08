//! Round 22's ports (`ffx_magic::r22`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::manual_midpoint,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
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

fn kf(t: &Twin, f: u32, o: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(f + o)))
}

/// The vector and matrix slots as float maths both sides share: 234
/// (`a - b`), 248 (`a + b`), 235 (a matrix varied by its call), 328/329
/// (one word of the matrix plus the angle), 229 (`M[i] * 0.5 + v[i]`).
fn vector_slots(t: &mut Twin) {
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
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
            fold(m, s, k, v.to_bits());
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            // the input words themselves too (a tiny float would vanish
            // in the sum)
            let x = m.r32(a[2] + 4 * i);
            fold(m, 229, 4 + i, x);
            let v = rf(m, a[1] + 4 * i) * 0.5 + f32::from_bits(x);
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
}

/// Float slots: 251, 19, 17 (by their first word) and 12 (the ground `g`
/// plus the call count).
fn float_slots(t: &mut Twin, g: f64) {
    for (s, v) in [(251u32, 1.5f64), (19, 0.75), (17, 2.5)] {
        rec(t, s, if s == 17 { 2 } else { 1 }, move |_, a| {
            (s << 16) ^ a[0]
        });
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 3));
    }
    rec(t, 12, 4, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |m, _| g + f64::from(m.r32(row(12))) * 0.25);
}

/// `magic_0645` 0x10007460 (`r22::summon_setup_0645`): the node list
/// empty and not, the yaw by slot 333 (flag bit 0) or slot 601.
#[test]
fn summon_setup_0645() {
    let Some(mut t) = twin(645) else { return };
    let f = t.at(0x7460);
    let list = t.img.word(f + 0x63);
    let ring_last = t.img.word(f + 0x2e);
    let ring_flags = t.img.word(f + 0x49);
    // the bone walk's DLL tables, as magic_0342's test does: the
    // function table (rotation thunks) and the +0x1c visit of the two
    // static children's vtables (0x100035a0, `nodes::visit_plain`) answer
    // from logging stubs on both sides (the hook binds those functions to
    // their ports in the game)
    let bone = t.img.rel32_target(f + 0x294);
    let table = t.img.word(bone + 0xed);
    rec(&mut t, 32, 2, |_, a| 0x0032_0000 ^ a[0]);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
    rec(&mut t, 1001, 1, |m, a| 0x1001_0000 ^ m.r32(a[0]));
    for o in [0xb0u32, 0x192] {
        t.set_u32(t.img.word(f + o) + 0x1c, magic::TRAPS + 16 * 1001);
    }
    let vt = SCRATCH + 0x880;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    vector_slots(&mut t);
    rec(&mut t, 262, 4, |m, a| {
        let c = m.r32(row(262)) as f32;
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + c * 0.25);
        }
        0x262
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]);
            wf(m, a[0] + 4 * i, v);
        }
        0x233
    });
    rec(&mut t, 327, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0100_0000);
        }
        0x327
    });
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
    for (s, n) in [(211u32, 2usize), (719, 3), (941, 1), (714, 5)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let obj = SCRATCH + 0x8000;
    let src = SCRATCH + 0x2000;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (k, (empty, flag)) in (0u32..).zip([(true, 0u16), (false, 1), (true, 0x8001)]) {
        junk(&mut t, obj, 0x5300, k);
        junk(&mut t, src, 0x300, k + 1);
        for i in 0..4u32 {
            t.set_u32(src + 0x98 * i + 0x54, 0x00c0_0054 + k + 0x10 * i);
            t.set_u32(src + 0x98 * i + 0x50, 0x00c0_0050 + k + 0x10 * i);
            t.set_u16(src + 0x98 * i + 0x90, flag);
        }
        t.set_u32(src + 0xec, 0x00c0_00ec);
        t.set_u32(src + 0x184, 0x00c0_0184);
        t.set_u32(src + 0x21c, 0x00c0_021c);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        // the group's third object and the parent bone (DLL globals)
        // with a vtable, the parent's words varied
        let parent = t.img.word(f + 0x26f);
        junk(&mut t, parent, 0x180, k + 5);
        for o in [0x27cu32, 0x26f] {
            t.set_u32(t.img.word(f + o), vt);
        }
        // the walk's pass byte (a global), varied
        t.set_u8(t.img.word(f + 0x286), 0x30 + k as u8);
        scrub(&mut t);
        let c = Call {
            ecx: obj,
            edx: src,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r22::summon_setup_0645(g, img, fr)
        });
    }
    assert!(t.u32(row(32)) >= 3, "the bone's table call");
    assert!(t.u32(row(1001)) >= 3, "the walk's visits");
}

/// `magic_0603` 0x1000b4d0 (`r22::task_0603`): held; state 2 counting,
/// at its end (the node with a parent first among its siblings, in the
/// middle, without a parent); state 0 above and below `O[0x514]`'s
/// bound, the timer ending below 0, at -0.0 and above; another state.
#[test]
fn task_0603() {
    let Some(mut t) = twin(603) else { return };
    let f = t.at(0xb4d0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    vector_slots(&mut t);
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]) + 0.5;
            wf(m, a[0] + 4 * i, v);
        }
        0x233
    });
    // handles_registered's slots and globals: the lock and the count
    let hr = t.img.rel32_target(f + 0xe5);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(hr + 0x21), lock);
    t.set_u32(t.img.word(hr + 0x81), 2);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 779, 2, move |m, a| 0x0779_0000 ^ a[0] ^ m.r32(lock));
    let task = SCRATCH + 0x100;
    let o = SCRATCH + 0x4000;
    let (parent, sib) = (SCRATCH + 0x1000, SCRATCH + 0x1100);
    let (lp, ln) = (SCRATCH + 0x1200, SCRATCH + 0x1300);
    // (hold, state, count, O[0x514], timer, parent: none / first / middle)
    let cases: [(u32, u32, f32, f32, f32, u32); 11] = [
        (1, 2, 0.0, 1.0, 0.0, 0),
        (0, 2, 0.5, 1.0, 0.0, 0),
        (0, 2, 1.99, 1.0, 0.0, 1),
        (0, 2, 1.99, 1.0, 0.0, 2),
        (0, 2, 2.5, 1.0, 0.0, 0),
        (0, 0, 0.5, 1.0, 0.1, 0),
        (0, 0, 0.5, 0.5, -0.25, 0),
        (0, 0, 0.5, 1.0, 0.15, 0),
        (0, 0, 0.5, 0.75, -0.0, 0),
        (0, 0, 0.5, 0.5, 0.5, 0),
        (0, 5, 0.0, 1.0, 0.0, 0),
    ];
    for (k, (hold, state, count, x, timer, par)) in (0u32..).zip(cases) {
        junk(&mut t, o, 0x2c00, k);
        junk(&mut t, parent, 0x400, k + 1);
        t.set_u32(task + 4, o);
        t.set_u32(op(&t, 0x12), hold);
        t.set_u32(o + 0x2a9c, state);
        t.set_u32(o + 0x2aac, count.to_bits());
        t.set_u32(o + 0x514, x.to_bits());
        t.set_u32(o + 0xe60, timer.to_bits());
        let node = o + 0x530;
        match par {
            0 => t.set_u32(node + 0x14, 0),
            p => {
                t.set_u32(node + 0x14, parent);
                t.set_u32(node + 4, if p == 1 { parent } else { sib });
                t.set_u32(node + 8, if p == 1 { sib } else { parent });
            }
        }
        t.set_u32(o + 0x41c, lp);
        t.set_u32(o + 0x420, ln);
        t.set_u32(lock, 0x10c0 + k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(779)) >= 3, "the handles");
}

/// `magic_0602` 0x10006ad0 (`r22::step_0602`): the first script done
/// (the sounds stopped and the node given back) and running (the blocks,
/// the motion scripts and the camera rigs), the second either way.
#[test]
fn step_0602() {
    let Some(mut t) = twin(602) else { return };
    let f = t.at(0x6ad0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, n) in [
        (941u32, 1usize),
        (772, 2),
        (779, 4),
        (326, 4),
        (229, 3),
        (284, 3),
        (285, 2),
        (286, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (413, 1),
        (215, 1),
        (19, 1),
        (21, 2),
        (332, 1),
        (6, 2),
        (547, 1),
        (295, 2),
        (541, 3),
        (38, 3),
        (568, 1),
        (566, 1),
        (546, 1),
        (213, 2),
        (219, 2),
        (220, 3),
        (711, 3),
        (710, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    for (s, n) in [(384u32, 3usize), (25, 3), (24, 3), (271, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    // the sounds' stop (as `magic_0602.rs`'s `sounds_stop`): the record,
    // the actors' table, no node
    let at = t.img.rel32_target(f + 0x1db);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x1), b);
    t.set_u32(b, 0x0302_0100);
    t.set_u32(b + 4, 0x0706_0504);
    let table = t.img.word(at + 0xc);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0384_0000 + i);
    }
    t.set_u32(t.img.word(at + 0xce), 0);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x26), 0x00c0_0026);
    let node = SCRATCH + 0x5000;
    let gb = t.img.rel32_target(f + 0x1e6);
    for (k, (s1, s2)) in (0u32..).zip([(0u8, 0u8), (0, 2), (2, 0), (2, 2)]) {
        t.write(r, &vec![0; 0x4000]);
        junk(&mut t, r + 0x1990, 0xe0, k);
        junk(&mut t, r + 0x17f0, 0x1c0, k + 1);
        t.set_u32(r + 0x2274, 0x5a5a_5a5a);
        t.write(w(&t, 0x10), &[0x5c; 0x30]);
        for (ctx, st) in [(w(&t, 0x2e), s1), (w(&t, 0x3d), s2)] {
            t.write(ctx, &[0; 0x58]);
            t.set_u32(ctx, SCRATCH + 0x700);
            t.set_u8(ctx + 0x56, st);
            t.set_u16(ctx + 0x54, 0xffff);
        }
        t.set_u32(w(&t, 0x07), r);
        for o in [0x48u32, 0x55] {
            t.set_u32(t.img.word(gb + o), 0x0bad_0000 + o);
        }
        t.write(node, &[0x21; 0x20]);
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, SCRATCH + 0x5100);
        t.set_u32(SCRATCH + 0x510c, 0);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
    assert!(t.u32(row(384)) >= 6, "the sounds' stop ran twice");
}
