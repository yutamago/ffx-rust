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

/// Slot 248 (`a + b`) with only the first three lanes logged: the
/// summon set-up's `w` lanes are frame junk (0 in the port).
fn add_xyz(t: &mut Twin) {
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            if i < 3 {
                fold(m, 248, i, v.to_bits());
            }
        }
        0x248
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
    add_xyz(&mut t);
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

/// `magic_0645` 0x10007c60 (`r22::summon_task_0645`): the start (state
/// 0, through to 0x14), 0x14 waiting and at 0 (on to 10, waiting) and
/// at -1, 10 waiting and at 0, 11 waiting and at 0, 12 waiting, at 0
/// with the task's bit 2, the particle node's bit 2 and neither (on to
/// 20000), 20000, and three other states.
#[test]
fn summon_task_0645() {
    let Some(mut t) = twin(645) else { return };
    let f = t.at(0x7c60);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    vector_slots(&mut t);
    add_xyz(&mut t);
    // the summon set-up's slots (as `summon_setup_0645`)
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
    // the handle's matrix
    rec(&mut t, 210, 2, |m, a| {
        let k = m.r32(row(210)) as f32;
        for i in 0..16u32 {
            wf(m, a[1] + 4 * i, 0.125 * i as f32 - k * 0.5);
        }
        0x210
    });
    for (s, n) in [
        (7u32, 1usize),
        (8, 2),
        (639, 1),
        (713, 1),
        (719, 3),
        (211, 2),
        (941, 1),
        (714, 5),
        (215, 1),
        (413, 1),
        (14, 4),
        (538, 2),
        (213, 2),
        (35, 5),
        (34, 2),
        (547, 1),
        (276, 3),
        (219, 2),
        (257, 2),
        (38, 3),
        (296, 2),
        (541, 3),
        (30, 1),
        (36, 2),
        (228, 2),
        (325, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the bone walk's DLL tables (as `summon_setup_0645`)
    let ss = t.img.rel32_target(f + 0x184);
    let bone = t.img.rel32_target(ss + 0x294);
    let table = t.img.word(bone + 0xed);
    rec(&mut t, 32, 2, |_, a| 0x0032_0000 ^ a[0]);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
    rec(&mut t, 1001, 1, |m, a| 0x1001_0000 ^ m.r32(a[0]));
    for o in [0xb0u32, 0x192] {
        t.set_u32(t.img.word(ss + o) + 0x1c, magic::TRAPS + 16 * 1001);
    }
    let vt = SCRATCH + 0x880;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    // the particle node's globals (as `magic_s3.rs`'s test)
    let pnw = t.img.rel32_target(f + 0x53c);
    let pn = t.img.rel32_target(pnw + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(t.img.word(pnw + 0x0d), owner);
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
    let task = SCRATCH + 0x100;
    let (tn, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let o = SCRATCH + 0x1_0000;
    let list = op(&t, 0x5d);
    let clist = op(&t, 0x1a3);
    let ring_last = t.img.word(ss + 0x2e);
    let ring_flags = t.img.word(ss + 0x49);
    let old = SCRATCH + 0x800;
    // (state, count, the task's bit 2, the particle node's bit 2)
    let cases: [(u32, u32, u8, u8); 17] = [
        (0, 7, 0, 0),
        (0x14, 5, 0, 0),
        (0x14, 0, 0, 0),
        (0x14, u32::MAX, 0, 0),
        (10, 3, 0, 0),
        (10, 0, 0, 0),
        (11, 2, 0, 0),
        (11, 0, 0, 0),
        (12, 1, 0, 0),
        (12, 0, 4, 0),
        (12, 0, 0, 4),
        (12, 0, 0xfb, 0xfb),
        (0x2710, 9, 0, 0),
        (5, 1, 0, 0),
        (0x13, 0, 0, 0),
        (0x30, 2, 0, 0),
        (9, 0, 0, 0),
    ];
    for (k, (state, count, bit, pbit)) in (0u32..).zip(cases) {
        junk(&mut t, o, 0x6600, k);
        // the model nodes' and the particle node's parents: none
        for n in [0x5880u32, 0x5918, 0x62e0] {
            t.set_u32(o + n + 0x14, 0);
        }
        t.set_u32(o + 0x18, state);
        t.set_u32(o + 0x1c, count);
        t.set_u8(o + 0x63c8, bit);
        t.set_u8(o + 0x62e0 + 0xe8, pbit);
        t.set_u32(o + 0x58d4, 0x00d4_0000 + k);
        junk(&mut t, task, 0x100, k + 1);
        t.set_u32(task, tn);
        t.set_u32(task + 4, o);
        t.set_u32(tn + 4, prev);
        t.set_u32(tn + 8, next);
        // the static lists: empty, or with a last node
        for (l, i) in [(list, 0u32), (clist, 1)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        for o2 in [0x26fu32, 0x27c] {
            t.set_u32(t.img.word(ss + o2), vt);
        }
        t.set_u8(t.img.word(ss + 0x286), 0x30 + k as u8);
        t.set_u32(op(&t, 0x40b), SCRATCH + 0x7f20);
        t.set_u32(lock, 0x77 + k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(32)) >= 1, "the summon set-up's bone walk");
    assert!(t.u32(row(713)) >= 1, "the particle node's slot 713");
}

/// `magic_0645` 0x10006990 (`r22::spark_task_0645`): states 0, 1 waiting
/// and done (on to 10: a spark born and drawn), 10 with the alpha up
/// (sparks 5, 100, -7 and the last index: none born), at 0 and below (on
/// to 20000), 11 (no sparks, three, past the cap of 96, a negative
/// count; the alpha dropping or not), 20000 and three other states.
#[test]
fn spark_task_0645() {
    let Some(mut t) = twin(645) else { return };
    let f = t.at(0x6990);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    vector_slots(&mut t);
    rec(&mut t, 210, 2, |m, a| {
        let k = m.r32(row(210)) as f32;
        for i in 0..16u32 {
            wf(m, a[1] + 4 * i, 0.125 * i as f32 - k * 0.5);
        }
        0x210
    });
    // slot 230: the scale matrix's words folded into the output's
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16u32 {
            let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ i;
            m.w32(a[0] + 4 * i, v);
        }
        0x230
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| f64::from(m.r32(row(307)) % 13) * 0.07 + 0.02);
    rec(&mut t, 324, 0, |m, _| 0x0324_0000 + m.r32(row(324)) * 0x111);
    // the sequence step's slots (as `magic_s6_misc.rs`'s test)
    let ss = t.img.rel32_target(f + 0x49f);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(ss + 0x24), lock);
    let loaded = SCRATCH + 0x7e0;
    rec(&mut t, 779, 2, move |m, _| 0x0779_0000 ^ m.r32(lock));
    rec(&mut t, 773, 2, move |m, _| m.r32(loaded));
    rec(&mut t, 774, 5, move |m, a| 0x0774_0000 ^ m.r32(a[2]));
    rec(&mut t, 775, 2, |_, _| 0x775);
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 | (a[0] << 4));
    rec(&mut t, 777, 1, |m, a| 0x0777_0001 ^ m.r32(a[0]));
    rec(&mut t, 778, 6, |m, a| 0x0778_0000 ^ m.r32(a[2]));
    let task = SCRATCH + 0x100;
    let (tn, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let o = SCRATCH + 0x1_0000;
    let (src, q) = (SCRATCH + 0x400, SCRATCH + 0x1000);
    let zg = op(&t, 0x331);
    // (state, wait, alpha, sparks, alpha drops, loaded)
    let cases: [(u32, u32, u32, u32, u32, u32); 18] = [
        (0, 7, 0x40, 5, 0, 0),
        (1, 3, 0x40, 5, 0, 0),
        (1, 0, 0, 5, 1, 1),
        (10, 2, 0x40, 5, 0, 0),
        (10, 2, 0x40, 100, 1, 1),
        (10, 2, 0x40, (-7i32).cast_unsigned(), 0, 0),
        (10, 2, 0x40, i32::MAX.cast_unsigned(), 0, 1),
        (10, 2, 0, 5, 0, 0),
        (10, 2, (-4i32).cast_unsigned(), 5, 0, 0),
        (11, 2, 0x40, 0, 0, 0),
        (11, 2, 0x30, 3, 1, 1),
        (11, 2, 0x30, 0x70, 0, 0),
        (11, 2, 0x30, (-2i32).cast_unsigned(), 1, 0),
        (0x2710, 2, 0x30, 3, 0, 0),
        (5, 2, 0x30, 3, 0, 0),
        (0x4e20, 2, 0x30, 3, 0, 0),
        (12, 0, 0x30, 3, 0, 0),
        (9, 0, 0x30, 3, 0, 0),
    ];
    for (k, (state, wait, alpha, sparks, drops, is_loaded)) in (0u32..).zip(cases) {
        junk(&mut t, o - 0x200, 0x1200, k);
        t.set_u32(o + 0xe6c, state);
        t.set_u32(o + 0xe70, wait);
        t.set_u32(o + 0xe78, alpha);
        t.set_u32(o + 0xe74, sparks);
        t.set_u32(o + 0xe7c, drops);
        t.set_u32(o + 0xe80, src);
        junk(&mut t, src, 0x80, k + 1);
        t.set_u32(src + 0x54, 0x00c0_0054 + k);
        t.set_u32(o + 0xe50, q);
        // the sparks' sequence: three steps, the records' steps and times
        t.write(q, &[0x3c; 0x100]);
        t.set_u16(q + 6, 3);
        for i in 0..4u32 {
            let r = q + 0x10 + 8 * i;
            t.set_u16(r, (0x80 + 0x20 * i) as u16);
            t.set_u16(r + 2, 0x180);
            let strip = q + 0x80 + 0x20 * i;
            t.set_u32(strip + 4, 0x10);
            t.set_u8(strip + 0x14, (i * 9) as u8);
        }
        for i in 0..0x60u32 {
            let rec_at = o + 0xcd0 + 4 * i;
            t.set_u16(rec_at, (i % 4) as u16);
            t.set_u16(rec_at + 2, (0x40 * i) as u16);
        }
        junk(&mut t, task, 0x40, k + 2);
        t.set_u32(task, tn);
        t.set_u32(task + 4, o);
        t.set_u32(tn + 4, prev);
        t.set_u32(tn + 8, next);
        t.set_u32(zg, 0x00ab_0000 + k);
        t.set_u32(loaded, is_loaded);
        t.set_u32(lock, 0x5a5a_0000 + k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(324)) >= 3, "the sparks born");
    assert!(t.u32(row(776)) + t.u32(row(774)) >= 3, "the sequence steps");
}
