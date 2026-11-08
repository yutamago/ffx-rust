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
    // the render record and its second matrix logged (rows of their own)
    let render = |m: &mut dyn StubMem, r: u32| {
        magic_owna::dump(m, 0x3fe, r, 8);
        let mx = m.r32(r + 0x1c);
        magic_owna::dump(m, 0x3fd, mx, 16);
    };
    rec(&mut t, 774, 5, move |m, a| {
        render(m, a[3]);
        0x0774_0000 ^ m.r32(a[2])
    });
    rec(&mut t, 775, 2, |_, _| 0x775);
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 | (a[0] << 4));
    rec(&mut t, 777, 1, |m, a| {
        magic_owna::dump(m, 0x3fc, a[0], 16);
        0x0777_0001 ^ m.r32(a[0])
    });
    rec(&mut t, 778, 6, move |m, a| {
        render(m, a[3]);
        0x0778_0000 ^ m.r32(a[2])
    });
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
        // the render record's two words the original never writes: 0 on
        // both sides (the port's junk-read value)
        let rb = ((magic::STACK_TOP - 0x104) & !0xf) - 0x130 + 0x20;
        t.set_u16(rb, 0);
        t.set_u32(rb + 0xc, 0);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(324)) >= 3, "the sparks born");
    assert!(t.u32(row(776)) + t.u32(row(774)) >= 3, "the sequence steps");
}

/// `magic_0478` 0x10006c40 (`r22::summon_task_0478`): the start (state
/// 0; the count by slot 333 and by slot 601, the scale above 1 and not,
/// the DLL's lists empty and not), 20, 21 waiting, at 0 and below (the
/// fades, on to 20000), 20000 and three other states.
#[test]
fn summon_task_0478() {
    let Some(mut t) = twin(478) else { return };
    let f = t.at(0x6c40);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 262, 4, |m, a| {
        let c = m.r32(row(262)) as f32;
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + c * 0.25);
        }
        0x262
    });
    rec(&mut t, 601, 2, move |m, a| {
        let v = m.r32(cell);
        m.w32(a[1], v);
        0x601
    });
    for (s, x) in [
        (15u32, 1.5f64),
        (16, -2.0),
        (17, 0.25),
        (277, 3.0),
        (664, 0.625),
        (333, 0.375),
        (12, 0.8125),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            x + f64::from(a[0] & 3) + f64::from(m.r32(row(s)) & 1)
        });
    }
    // slot 790 (no arguments: the unit block's float)
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |m, _| 0.0625 + f64::from(m.r32(row(790)) & 3));
    let scale = SCRATCH + 0x14;
    rec(&mut t, 251, 1, |_, a| 0x0251_0000 ^ a[0]);
    t.stub_st0(251, move |m, _| f64::from(f32::from_bits(m.r32(scale))));
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, i as f32 - 0.5);
        }
        0x290
    });
    // the actor's models: two numbers
    let models = SCRATCH + 0x20;
    t.set_u32(models, 0x0000_0503);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        models
    });
    // slot 558: four integers
    rec(&mut t, 558, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x10 * i + 3);
        }
        0x558
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 0.25 * i as f32 + 0.125);
        }
        0x268
    });
    for (s, n) in [
        (215u32, 1usize),
        (413, 1),
        (14, 4),
        (211, 2),
        (331, 1),
        (353, 1),
        (271, 1),
        (719, 3),
        (941, 1),
        (325, 2),
        (714, 5),
        (538, 2),
        (213, 2),
        (35, 5),
        (34, 2),
        (547, 1),
        (276, 3),
        (219, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the particle node's globals (as `magic_s3.rs`'s test)
    let pn = t.img.rel32_target(f + 0x52e);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
    // the blink object's task ring and its flags (as `magic_0478.rs`)
    let init = t.img.rel32_target(f + 0x6fd);
    let ring = SCRATCH + 0x3000;
    let task = SCRATCH + 0x100;
    let (tn, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let o = SCRATCH + 0x1_0000;
    let list = op(&t, 0x1e6);
    let list2 = op(&t, 0x542);
    let old = SCRATCH + 0x800;
    // (state, count, flag bit 0 of `O + 0xd0`, scale)
    let cases: [(u32, u32, u8, f32); 10] = [
        (0, 7, 1, 0.5),
        (0, 7, 0, 4.0),
        (0x14, 5, 0, 1.0),
        (0x15, 5, 0, 1.0),
        (0x15, 0, 0, 1.0),
        (0x15, u32::MAX, 0, 1.0),
        (0x2710, 9, 0, 1.0),
        (3, 1, 0, 1.0),
        (0x16, 0, 0, 1.0),
        (0x30, 2, 0, 1.0),
    ];
    for (k, (state, count, flag, s)) in (0u32..).zip(cases) {
        junk(&mut t, o, 0xc00, k);
        // the model nodes' parents: none
        for n in [0x24u32, 0x514] {
            t.set_u32(o + n + 0x14, 0);
        }
        t.set_u32(o + 0x18, state);
        t.set_u32(o + 0x20, count);
        t.set_u8(o + 0xd0, flag);
        t.set_u32(o + 0x90, 0x00e5_0000 + k);
        t.set_u32(o + 0x94, 0x00e5_0100 + k);
        t.set_u32(cell, (0.25 * k as f32).to_bits());
        t.set_u32(scale, s.to_bits());
        junk(&mut t, task, 0x40, k + 1);
        t.set_u32(task, tn);
        t.set_u32(task + 4, o);
        t.set_u32(tn + 4, prev);
        t.set_u32(tn + 8, next);
        // the DLL's lists: empty, or with a last node
        for (l, i) in [(list, 0u32), (list2, 1)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(t.img.word(init + 0x24), ring);
        t.set_u32(t.img.word(init + 0x3f), 0xff);
        t.set_u32(op(&t, 0x4e4), owner);
        t.set_u32(op(&t, 0x1ca), 0x00ac_0000 + k);
        t.set_u32(lock, 0x77 + k);
        // the fades' globals varied
        for o2 in [0x59u32, 0x81, 0xbc, 0xf7, 0x119, 0x12c, 0x13c, 0x144, 0x157] {
            t.set_u32(
                op(&t, o2),
                (0.0625 * (o2 % 13) as f32 + 0.1 * k as f32).to_bits(),
            );
        }
        t.set_u32(op(&t, 0x5e), 0x5a5a_5a5b ^ k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(719)) >= 2, "the start ran twice");
    assert!(
        t.u32(row(538)) + t.u32(row(213)) >= 1,
        "the end's model set-up"
    );
}

/// `magic_0482` 0x10006e10 (`r22::summon_task_0482`): the start (state
/// 0; slot 415 0x7a and not, the DLL's lists empty and not), 10, 11
/// waiting, at 0 (the second nodes and the fades; on to 20000, not yet
/// ended) and below with `O + 0xd38` 40000 (the end), 20000 waiting and
/// ending, and three other states.
#[test]
fn summon_task_0482() {
    let Some(mut t) = twin(482) else { return };
    let f = t.at(0x6e10);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let kind = SCRATCH + 0x10;
    for (s, x) in [(333u32, 0.375f64), (12, 0.8125)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            x + f64::from(a[0] & 3) + f64::from(m.r32(row(s)) & 1)
        });
    }
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |m, _| 0.0625 + f64::from(m.r32(row(790)) & 3));
    rec(&mut t, 415, 1, move |m, _| m.r32(kind));
    let models = SCRATCH + 0x20;
    t.set_u32(models, 0x0000_0503);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        models
    });
    rec(&mut t, 558, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x10 * i + 3);
        }
        0x558
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 0.25 * i as f32 + 0.125);
        }
        0x268
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16u32 {
            wf(
                m,
                a[0] + 4 * i,
                if i % 5 == 0 { 1.0 } else { 0.0625 * i as f32 },
            );
        }
        0x235
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16u32 {
            let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ i;
            m.w32(a[0] + 4 * i, v);
        }
        0x230
    });
    for (s, n) in [
        (215u32, 1usize),
        (413, 1),
        (14, 4),
        (211, 2),
        (331, 1),
        (353, 1),
        (271, 1),
        (719, 3),
        (941, 1),
        (325, 2),
        (714, 5),
        (239, 3),
        (538, 2),
        (213, 2),
        (35, 5),
        (34, 2),
        (547, 1),
        (276, 3),
        (219, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the particle node's globals (as `magic_s3.rs`'s test)
    let pn = t.img.rel32_target(f + 0x391);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
    let task = SCRATCH + 0x100;
    let (tn, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let o = SCRATCH + 0x1_0000;
    let list = op(&t, 0xa6);
    let list2 = op(&t, 0x374);
    let old = SCRATCH + 0x800;
    let ring = SCRATCH + 0x3000;
    // (state, count, slot 415's answer, `O + 0xd38`)
    let cases: [(u32, u32, u32, u32); 11] = [
        (0, 7, 0x7a, 0),
        (0, 7, 0x7b, 0),
        (0xa, 5, 0, 0),
        (0xb, 5, 0, 0),
        (0xb, 0, 0, 0),
        (0xb, u32::MAX, 0, 0x4e20),
        (0x2710, 9, 0, 0x4e20),
        (0x2710, 9, 0, 0x4e21),
        (3, 1, 0, 0),
        (0xc, 0, 0, 0),
        (0x30, 2, 0, 0),
    ];
    for (k, (state, count, kd, end)) in (0u32..).zip(cases) {
        junk(&mut t, o, 0xe00, k);
        for n in [0x24u32, 0x514] {
            t.set_u32(o + n + 0x14, 0);
        }
        t.set_u32(o + 0x18, state);
        t.set_u32(o + 0x20, count);
        t.set_u32(o + 0xd38, end);
        t.set_u32(o + 0x94, 0x00e5_0100 + k);
        t.set_u32(kind, kd);
        junk(&mut t, task, 0x40, k + 1);
        t.set_u32(task, tn);
        t.set_u32(task + 4, o);
        t.set_u32(tn + 4, prev);
        t.set_u32(tn + 8, next);
        for (l, i) in [(list, 0u32), (list2, 1)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(op(&t, 0x573), ring);
        t.set_u32(op(&t, 0x58b), 0xff);
        for o2 in [0x35eu32, 0x63b] {
            t.set_u32(op(&t, o2), owner);
        }
        t.set_u32(op(&t, 0x8a), 0x00ac_0000 + k);
        t.set_u32(op(&t, 0x248), 0x00ad_0000 + k);
        t.set_u8(op(&t, 0x675), (0x30 + k) as u8);
        t.set_u32(lock, 0x77 + k);
        for o2 in [
            0x3c7u32, 0x3fb, 0x436, 0x467, 0x47a, 0x48d, 0x49d, 0x4a5, 0x4b8, 0x73f, 0x747,
        ] {
            t.set_u32(
                op(&t, o2),
                (0.0625 * (o2 % 13) as f32 + 0.1 * k as f32).to_bits(),
            );
        }
        t.set_u32(op(&t, 0x730), 0x5a5a_5a5b ^ k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(719)) >= 4, "the starts and the second nodes");
    assert!(t.u32(row(239)) >= 2, "the second nodes ran twice");
    assert!(
        t.u32(row(538)) + t.u32(row(213)) >= 1,
        "the end's model set-up"
    );
}

/// `magic_0483` 0x10001390 (`r22::step_0483`): off; the first frame and
/// later ones; slot 261's three answers: (2) the parts' start and their
/// run (the lock, slot 712 either way, the draws from 0x3a, the stop
/// word), (1) the start, waiting, running from 10 and ending, (0) the
/// start (slot 262's point and the double sums), the bone follow, the
/// second part from 0x3c, past 0x50; none of them; the count 0x8c.
#[test]
fn step_0483() {
    let Some(mut t) = twin(483) else { return };
    let f = t.at(0x1390);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let cells = SCRATCH + 0x40;
    // 261: an answer per argument (cells + 4 a); 253 and 712 from cells
    rec(&mut t, 261, 1, move |m, a| m.r32(cells + 4 * (a[0] & 3)));
    rec(&mut t, 253, 0, move |m, _| m.r32(cells + 0x10));
    let lockw = SCRATCH + 0x60;
    rec(&mut t, 712, 2, move |m, a| {
        m.r32(cells + 0x14) ^ (m.r32(lockw) << 8) ^ a[1]
    });
    let table = SCRATCH + 0x80;
    for i in 0..8u32 {
        t.set_u32(table + 4 * i, 0x0941_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, move |m, a| {
        0x0714_0000 ^ a[2] ^ m.r32(lockw)
    });
    rec(&mut t, 237, 4, |m, a| {
        let c = m.r32(row(237)) as f32;
        for i in 0..3u32 {
            wf(
                m,
                a[3] + 4 * i,
                0.5 * i as f32 + c * 0.25 - (a[1] % 7) as f32,
            );
        }
        0x237
    });
    rec(&mut t, 262, 4, |m, a| {
        let c = m.r32(row(262)) as f32;
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32 - c * 0.125);
        }
        0x262
    });
    for (s, n) in [
        (711u32, 3usize),
        (710, 4),
        (723, 1),
        (724, 1),
        (713, 1),
        (258, 1),
        (231, 3),
        (230, 2),
        (214, 1),
    ] {
        rec(&mut t, s, n, move |m, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0) ^ (m.r32(lockw) << 4)
        });
    }
    // the pointer globals: the lock, the stop word, the part pointer, the
    // base
    let (stopw, pw, basew) = (SCRATCH + 0x64, SCRATCH + 0x68, SCRATCH + 0x6c);
    for (o, cell) in [(0x28u32, lockw), (0x88, stopw), (0x101, pw), (0xc0, basew)] {
        t.set_u32(op(&t, o), cell);
    }
    // (on, phase, 261 (0, 1, 2), 253, 712, counters: e8, f5, 3d1, 3df,
    // 5b6, 5cd; halfwords 240, 521, 7d9, 898)
    #[allow(clippy::type_complexity)]
    let cases: Vec<(u32, u32, [u32; 3], u32, u32, [u32; 6], [u16; 4])> = vec![
        (0, 0, [0, 0, 1], 0, 0, [0; 6], [0; 4]),
        (1, 0, [0, 0, 1], 0, 0, [0, 0, 0, 0, 0, 0], [0; 4]),
        (1, 3, [0, 0, 1], 1, 1, [0x40, 1, 0, 0, 0, 0], [1, 0, 0, 0]),
        (1, 3, [0, 0, 1], 0, 0, [0x39, 1, 0, 0, 0, 0], [0, 0, 0, 0]),
        (1, 3, [0, 1, 0], 0, 0, [0x8c, 0, 0, 0, 0, 0], [0; 4]),
        (1, 3, [0, 1, 0], 0, 0, [0, 0, 0, 1, 0, 0], [0; 4]),
        (1, 3, [0, 1, 0], 0, 1, [0, 0, 0xa, 1, 0, 0], [0, 1, 0, 0]),
        (1, 3, [0, 1, 0], 1, 0, [0, 0, 0xc, 1, 0, 0], [0, 0, 0, 0]),
        (1, 3, [0, 1, 0], 0, 0, [0, 0, 5, 1, 0, 0], [0, 1, 0, 0]),
        (1, 3, [1, 0, 0], 0, 0, [0, 0, 0, 0, 0, 0], [0; 4]),
        (1, 3, [1, 0, 0], 0, 1, [0, 0, 0, 0, 0x50, 1], [0, 0, 1, 1]),
        (1, 3, [1, 0, 0], 1, 0, [0, 0, 0, 0, 0x3c, 1], [0, 0, 1, 1]),
        (1, 3, [1, 0, 0], 1, 0, [0, 0, 0, 0, 0x3b, 1], [0, 0, 1, 1]),
        (1, 3, [0, 1, 0], 0, 1, [0, 0, 9, 1, 0, 0], [0, 1, 0, 0]),
        (1, 3, [1, 0, 0], 0, 0, [0, 0, 0, 0, 0x51, 1], [0, 0, 1, 1]),
        (1, 3, [0, 0, 0], 0, 0, [0x8c, 0, 0, 0, 0, 0], [0; 4]),
    ];
    for (k, (on, phase, ans, a253, a712, ctr, hw)) in (0u32..).zip(cases) {
        t.set_u32(op(&t, 0x12), on);
        t.set_u32(op(&t, 0x1f), phase);
        for (i, a) in (0u32..).zip(ans) {
            t.set_u32(cells + 4 * i, a);
        }
        t.set_u32(cells + 0x10, a253);
        t.set_u32(cells + 0x14, a712);
        for (o, v) in [0xe8u32, 0xf5, 0x3d1, 0x3df, 0x5b6, 0x5cd]
            .into_iter()
            .zip(ctr)
        {
            t.set_u32(op(&t, o), v);
        }
        for (o, v) in [0x240u32, 0x521, 0x7d9, 0x898].into_iter().zip(hw) {
            t.set_u16(op(&t, o), v);
        }
        t.set_u32(lockw, 0x10c0 + k);
        t.set_u32(stopw, 0x5a5a_5a5a ^ k);
        junk(&mut t, pw, 4, k);
        // the parts the draws run (halfword +0x1e set)
        for o in [0x37cu32, 0x393] {
            let p = op(&t, o);
            t.set_u16(p + 0x1e, 1);
        }
        // the points the moves read, varied
        for o in [
            0x477u32, 0x4a3, 0x4b3, 0x6be, 0x763, 0x7a2, 0x7f8, 0x81a, 0x832, 0x8e, 0xad, 0xbb,
            0x59,
        ] {
            t.set_u32(
                op(&t, o),
                (0.0625 * (o % 17) as f32 + 0.25 * k as f32).to_bits(),
            );
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(714)) >= 6, "the part starts");
    assert!(t.u32(row(712)) >= 4, "the parts' runs");
    assert!(t.u32(row(214)) >= 1, "the count 0x8c");
}

/// `magic_0491`'s flight: the slots and globals of the start
/// (0x10003c50), the step (0x100044f0), the burst and the trail.
/// Where `magic_0491`'s test keeps the ground's lowering.
const GROUND_0491: u32 = SCRATCH + 0x7f8;

fn flight_setup_0491(t: &mut Twin) {
    magic_r11::object_slots(t);
    let step = t.at(0x44f0);
    let start = t.img.rel32_target(step + 0x3a);
    let obj_start = t.img.rel32_target(start + 0xd8);
    magic_r11::object_start_auto(t, obj_start);
    rec(t, 262, 4, |m, a| {
        let c = m.r32(row(262)) as f32;
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + c * 0.75 + (a[0] % 5) as f32);
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        let c = m.r32(row(268)) as f32;
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 1.25 - i as f32 + c * 0.5);
        }
        0x268
    });
    for (s, x) in [(333u32, 0.375f64), (314, 1.75), (404, -2.5), (12, 0.625)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        // (slot 12, the ground: lowered by the float at GROUND)
        t.stub_st0(s, move |m, a| {
            let low = if s == 12 {
                f64::from(rf(m, GROUND_0491))
            } else {
                0.0
            };
            x + f64::from(a[0] & 7) * 0.5 + f64::from(m.r32(row(s)) % 3) - low
        });
    }
    rec(t, 235, 1, |m, a| {
        for i in 0..16u32 {
            wf(
                m,
                a[0] + 4 * i,
                if i % 5 == 0 { 1.0 } else { 0.0625 * i as f32 },
            );
        }
        0x235
    });
    rec(t, 329, 3, |m, a| {
        for i in 0..16u32 {
            let v = rf(m, a[1] + 4 * i) + f32::from_bits(a[2]) * 0.125;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    rec(t, 229, 3, |m, a| {
        // the matrix's words too
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            fold(m, 0x3f0, i, x);
        }
        for i in 0..4 {
            let x = m.r32(a[2] + 4 * i);
            fold(m, 229, 4 + i, x);
            let v = rf(m, a[1] + 4 * i) * 0.5 + f32::from_bits(x);
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
    // the burst's (as `magic_r20.rs`'s test) and the sparks' slots
    rec(t, 618, 5, |m, a| {
        m.w32(a[3], (-150i32).cast_unsigned());
        0x618
    });
    rec(t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| f64::from(m.r32(row(307)) % 13) * 0.07 + 0.02);
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            wf(m, a[0] + 4 * i, f32::from_bits(v) * 0.5);
        }
        0x232
    });
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    // the trail's slots (as `magic_s4_misc.rs`'s test)
    let lock = SCRATCH + 0x9000;
    let td = t.img.rel32_target(step + 0x3b4);
    t.set_u32(t.img.word(td + 0x463), lock);
    t.set_u32(t.img.word(td + 0x520), 0x0cc0_0cc0);
    for s in [723u32, 724] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, (i as f32 + 1.0) * 0.5);
            }
            s
        });
    }
    rec(t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 773, 4, |_, a| (a[3] >> 1) & 1);
    for s in [774u32, 775, 778, 779] {
        rec(t, s, 2, move |_, a| s ^ a[0]);
    }
    rec(t, 776, 1, |_, a| a[0] << 4);
    rec(t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
}

/// The trail block of the owner `o` (`o + 0x260`): `n` sprites.
fn trail_0491(t: &mut Twin, o: u32, n: u32, k: u32) {
    let obj = o + 0x260;
    t.set_u32(obj + 4, 1);
    t.set_u16(obj + 8, (k % 3) as u16);
    t.set_u16(obj + 0xa, 0x100);
    for j in 0..32u32 {
        let r = 1.0 + j as f32 * 0.3;
        let a = j as f32 * 0.7;
        for (i, x) in (0u32..).zip([r * a.cos(), j as f32 * 0.2, r * a.sin(), 1.0]) {
            t.set_u32(obj + 0xc + 16 * j + 4 * i, x.to_bits());
        }
    }
    t.write(obj + 0x20c, &[10, 200, 0, 0x80, 250, 3, 128, 0x40]);
    for (off, x) in [(0x214u32, 1.5f32), (0x218, 0.25), (0x21c, 0.75)] {
        t.set_u32(obj + off, x.to_bits());
    }
    t.set_u32(obj + 0x220, 3 + k);
    t.set_u32(obj + 0x224, n);
}

/// `magic_0491` 0x10003c50 (`r22::flight_start_0491`) and 0x100044f0
/// (`r22::flight_step_0491`): the start with the actor from the global
/// and through the map, the yaws wrapped down, up and kept, the ground
/// above and below the middle point; the step's two waits (counting and
/// at their end: the start), the time capped and not, no sparks (the
/// trail fading, gone: given back), sparks counting (six records, the
/// pool running out) and at their end (the burst).
#[test]
fn flight_0491() {
    let Some(mut t) = twin(491) else { return };
    flight_setup_0491(&mut t);
    let step = t.at(0x44f0);
    let start = t.img.rel32_target(step + 0x3a);
    let op = |t: &Twin, f: u32, o: u32| t.img.word(f + o);
    let o = SCRATCH + 0x1_0000;
    let map = SCRATCH + 0x600;
    let take = t.img.rel32_target(step + 0x251);
    let recs = SCRATCH + 0x2_0000;
    let sprite_owner = SCRATCH + 0x700;
    // (actor index, the table's yaw input, ground)
    let starts: [(u32, f32, f32); 5] = [
        (u32::MAX, 0.25, 10.0),
        (1, 5.0, -10.0),
        (2, -5.0, 0.0),
        (u32::MAX, 0.0, f32::NAN),
        (3, 0.0, 0.0),
    ];
    let globals = |t: &mut Twin, k: u32| {
        t.write(map, &[2, 0, 1, 3, 1, 2, 0, 3]);
        for (o2, v) in [
            (0x37u32, 0x00a0_0037),
            (0x7d, 0x00a0_007d),
            (0x181, 0x0000_0181),
            (0x3df, 0x00a0_03df),
        ] {
            t.set_u32(op(t, start, o2), v + k);
        }
        for o2 in [0x3eu32, 0x188, 0x3e6, 0x111] {
            t.set_u32(op(t, start, o2), map);
        }
        for o2 in [0x49u32, 0x3f1, 0x11b] {
            let tab = op(t, start, o2);
            for i in 0..4 {
                t.set_u32(tab + 4 * i, 0x00ac_0000 + 0x100 * i + k);
            }
        }
        t.set_u32(op(t, start, 0x502), sprite_owner);
        t.set_u32(sprite_owner + 0x10, 0x00b0_0010 + k);
    };
    for (k, (idx, _yaw, _g)) in (0u32..).zip(starts) {
        junk(&mut t, o, 0x600, k);
        // the last case's ground far below the middle point
        t.set_u32(GROUND_0491, if k == 4 { 100.0f32 } else { 0.0 }.to_bits());
        t.set_u32(o + 0x504, idx);
        globals(&mut t, k);
        scrub(&mut t);
        let c = Call {
            ecx: o,
            ..Call::default()
        };
        t.check(start, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r22::flight_start_0491(g, img, fr)
        });
    }
    // the step: (wait 1, wait 2, time, rate, sparks left, trail count,
    // records in the pool)
    let task = SCRATCH + 0x100;
    let cases: [(u32, u32, f32, f32, u32, u32, u32); 9] = [
        (3, 0, 0.0, 0.1, 0, 2, 0x40),
        (1, 0, 0.0, 0.1, 0, 2, 0x40),
        (0, 4, 0.0, 0.1, 0, 2, 0x40),
        (0, 0, 0.5, 0.5, 0, 3, 0x40),
        (0, 0, 0.95, 0.1, 0, 0, 0x40),
        (0, 0, 0.5, 0.0833, 5, 1, 0x40),
        (0, 0, 0.5, 0.0833, 5, 1, 3),
        (0, 0, 0.5, 0.0833, 1, 1, 0x40),
        (
            (-1i32).cast_unsigned(),
            (-2i32).cast_unsigned(),
            0.75,
            0.5,
            2,
            1,
            0,
        ),
    ];
    for (k, (w1, w2, tm, rate, sparks, n, pool)) in (0u32..).zip(cases) {
        junk(&mut t, o, 0x600, k + 7);
        globals(&mut t, k);
        t.set_u32(o + 0x504, u32::MAX);
        t.set_u32(o + 0x50c, w1);
        t.set_u32(o + 0x510, w2);
        t.set_u32(o + 0x4fc, tm.to_bits());
        t.set_u32(o + 0x500, rate.to_bits());
        t.set_u32(o + 0x508, sparks);
        for i in 0..16u32 {
            t.set_u32(
                o + 0x4bc + 4 * i,
                (0.5 * i as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        trail_0491(&mut t, o, n, k);
        junk(&mut t, task, 0x40, k);
        t.set_u32(task + 0x14, o);
        // a given-back task: its links to itself
        t.set_u32(task, task);
        t.set_u32(task + 4, task);
        t.set_u32(task + 8, task);
        t.set_u32(task + 0xc, 0);
        let w = |t: &Twin, o2: u32| t.img.word(take + o2);
        t.set_u32(w(&t, 0x37), recs);
        for i in 0..0x48 {
            junk(&mut t, recs + 0x74 * i, 0x74, k + i);
            t.set_u32(recs + 0x74 * i + 0x6c, 0);
        }
        t.set_u32(w(&t, 0x0f), pool);
        t.set_u32(w(&t, 0x08), 0);
        t.set_u32(w(&t, 0x2b), 0);
        let burst = t.img.rel32_target(step + 0x246);
        for o2 in [0x14u32, 0x58, 0x65, 0x111, 0x15d, 0x1cc] {
            t.set_u32(op(&t, burst, o2), 0x0a00_0000 + o2);
        }
        scrub(&mut t);
        t.check_entry(step, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(325)) >= 6, "the sparks");
    assert!(t.u32(row(618)) >= 1, "the burst");
}

/// `magic_0601` 0x100035f0 (`r22::bone_build_0601`) and 0x10007f80
/// (`r22::node_build_0601`): no parent, a parent with no children and
/// one with a last child; the node build with its two stack words.
#[test]
fn bone_build_0601() {
    let Some(mut t) = twin(601) else { return };
    let f = t.at(0x35f0);
    let nb = t.at(0x7f80);
    rec(&mut t, 281, 1, |_, a| 0x0281_0000 ^ a[0]);
    rec(&mut t, 287, 3, |m, _| 0x0287_0000 + m.r32(row(287)));
    rec(&mut t, 283, 2, |_, a| 0x0283_0000 ^ a[0] ^ a[1]);
    let (r, n, parent, kid) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x800,
        SCRATCH + 0x900,
    );
    for (k, par) in (0u32..).zip([0u32, 1, 2]) {
        junk(&mut t, n, 0x220, k);
        junk(&mut t, r, 0x28, k + 1);
        junk(&mut t, parent, 0x40, k + 2);
        junk(&mut t, kid, 0x40, k + 3);
        t.set_u32(r, n);
        let p = match par {
            0 => 0,
            1 => {
                t.set_u32(parent + 0x10, parent);
                parent
            }
            _ => {
                t.set_u32(parent + 0x10, kid);
                parent
            }
        };
        t.set_u32(r + 4, p);
        scrub(&mut t);
        let c = Call {
            ecx: r,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r22::bone_build_0601(g, img, fr)
        });
    }
    for k in 0..2u32 {
        junk(&mut t, n, 0x220, k + 5);
        // the record's parent (operand +0x18): a list with a last child
        let v1 = t.img.word(nb + 0x18);
        junk(&mut t, v1, 0x20, k);
        t.set_u32(v1 + 0x10, if k == 0 { v1 } else { kid });
        scrub(&mut t);
        let c = Call {
            ecx: n,
            ..Call::default()
        };
        let args = [1, 2, 3, 4, 0x00a4_0000 + k, 6, 0x00b6_0000 + k];
        t.check(nb, c, &args, Eax::Checked, |g, img, fr| {
            ffx_magic::r22::node_build_0601(g, img, fr)
        });
    }
}

/// `magic_0610` 0x10007180 (`r22::cue_start_0610`) running 0x1000d6c0
/// (`r22::start_up_0610`): the waits, the once flag, slot 227 below 1,
/// and the start-up with the history empty, of one and of five entries,
/// the free ring of several nodes and of one, the task list empty and
/// not, slot 520 0x1ad0001 and not.
#[test]
fn start_up_0610() {
    let Some(mut t) = twin(610) else { return };
    let f = t.at(0x7180);
    let su = t.at(0xd6c0);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    let gs = |t: &Twin, op: u32| t.img.word(su + op);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 5, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 28, 2, |m, _| m.r32(SCRATCH + 0x18));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    rec(&mut t, 520, 0, |m, _| m.r32(SCRATCH + 0x20));
    // the caster placement's slots (as `magic_r18.rs`)
    rec(&mut t, 262, 4, |m, a| {
        for (i, v) in [1.5f32, -2.25, 0.75].into_iter().enumerate() {
            m.w32(a[1 + i], (v + (a[0] & 0xf) as f32).to_bits());
        }
        0x0262
    });
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.125 - 1.0);
    for (s, x) in [(277u32, 0.5f64), (15, 1.25), (16, -0.75), (17, 2.5)] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.0625 + x);
    }
    rec(&mut t, 293, 1, |_, _| 0);
    let list = SCRATCH + 0x760;
    t.write(list, &[5, 0, 9, 0, 0x22, 0, 0, 0]);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], 3);
        list
    });
    for (s, n) in [
        (210u32, 2usize),
        (211, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (238, 2),
        (296, 2),
        (257, 2),
        (719, 3),
        (711, 3),
        (710, 4),
        (271, 1),
        (287, 3),
        (283, 2),
        (281, 1),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
    let lock = SCRATCH + 0x7f0;
    t.set_u32(gs(&t, 0x84), lock);
    // slot 711 sees the game lock held
    rec(&mut t, 711, 3, move |m, a| {
        0x0711_0000 ^ a[0] ^ (m.r32(lock) << 8)
    });
    magic_r11::pools(&mut t, su, 943);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    // (225, 5, 28, once, 227, history n, free nodes, listed, 520)
    #[allow(clippy::type_complexity)]
    let cases: [(u32, u32, u32, u32, u32, u32, u16, bool, u32); 7] = [
        (1, 0, 0, 0, 1, 0, 4, false, 0),
        (0, 1, 0, 0, 1, 0, 4, false, 0),
        (0, 0, 2, 0, 1, 0, 4, false, 0),
        (0, 0, 0, 0, 0, 0, 4, false, 0),
        (0, 0, 0, 1, 1, 0, 4, false, 0x01ad_0001),
        (0, 0, 0, 0, 3, 1, 3, true, 0),
        (0, 0, 0, 1, 1, 5, 4, true, 0x01ad_0001),
    ];
    for (k, (w225, w5, w28, once, r227, hist, free, listed, r520)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x14, w5);
        t.set_u32(SCRATCH + 0x18, w28);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(SCRATCH + 0x20, r520);
        t.set_u32(g(&t, 0x50), once);
        t.set_u32(gs(&t, 0x1), hist);
        let hb = gs(&t, 0x53) - 1;
        junk(&mut t, hb, 8, k);
        t.set_u32(lock, 0x5a5a_0000 + k);
        let n = u32::from(free);
        for i in 0..n {
            t.write(node(i), &[0x40 + i as u8; 0x1c]);
            t.set_u32(node(i), node((i + n - 1) % n));
            t.set_u32(node(i) + 4, node((i + 1) % n));
        }
        t.set_u32(gs(&t, 0x238), node(0));
        t.set_u16(gs(&t, 0x221), free);
        let tail = SCRATCH + 0x1400;
        t.write(tail, &[0x66; 0x1c]);
        t.set_u32(gs(&t, 0x29a), if listed { tail } else { 0 });
        t.set_u32(gs(&t, 0x2ad), tail);
        t.set_u32(gs(&t, 0x398), (0.25 * k as f32).to_bits());
        // the words the start-up clears or sets, junk first
        junk_ops(
            &mut t,
            su,
            &[0x7, 0xd8, 0xe2, 0xec, 0xf6, 0x137, 0x1d8, 0x1fc],
        );
        for o in [0x151u32, 0x1e3, 0x1eb, 0x207, 0x20f] {
            let at = gs(&t, o);
            t.set_u32(at, 0x5a5a_5a5a ^ o);
        }
        let recs = gs(&t, 0x10e);
        for r in 0..0x64u32 {
            t.set_u32(recs + 0x140 * r, 0x7e7e_0000 + r);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(719)) >= 3, "the start-ups");
}
