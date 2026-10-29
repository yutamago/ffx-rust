//! Shared helpers without a family (`ffx_magic::sa_misc`) against the
//! DLLs they were ported from. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, SCRATCH, TRAPS, Twin};
use sa::rec;

#[test]
fn set_four_floors_and_sends() {
    let Some(mut t) = sa::twin(72) else { return };
    rec(&mut t, 933, 4, |_, _| 0x933);
    rec(&mut t, 785, 1, |_, a| 0x7850_0000 ^ a[0]);
    let f = t.at(0x1000);
    let v = SCRATCH + 0x100;
    for case in [
        [1.0f32, 2.0, 3.0, 4.0],
        [0.0, -0.0, -1.0, f32::NAN],
        [-5.0, 0.5, f32::NAN, 0.0],
        [1.0e-30, -1.0e-30, 1.0e30, -1.0e30],
    ] {
        for (i, x) in case.iter().enumerate() {
            t.set_u32(v + 4 * i as u32, x.to_bits());
        }
        t.check(f, Call::this(v), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::set_four(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn reset_transform_fills() {
    let Some(mut t) = sa::twin(9) else { return };
    let f = t.at(0x31a0);
    let obj = SCRATCH + 0x200;
    t.write(obj, &[0xa5; 0x140]);
    let c = Call {
        eax: 0x1234,
        ecx: obj,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    t.check(
        f,
        c,
        &[0x0a00_0000, 0x0a11_0000],
        Eax::Checked,
        |g, _, fr| {
            let a1 = fr.arg(g, 1);
            ffx_magic::Ret::eax(ffx_magic::sa_misc::reset_transform(g, fr.ecx, fr.edx, a1))
        },
    );
}

#[test]
fn unit_block_and_leave() {
    let Some(mut t) = sa::twin(42) else { return };
    let f = t.at(0x29b0);
    let blk = t.img.word(f + 0x1b);
    t.write(blk, &[0x5a; 0x34]);
    rec(&mut t, 790, 0, |_, _| 0x790);
    for v in [0.5f64, -1.0e-40, 1.0e300] {
        t.stub_st0(790, move |_, _| v);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::unit_block(g, img, fr.at))
        });
    }
    let f = t.at(0x8100);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (parent, prev, next) in [
        (0u32, 1u32, 2u32),
        (3, 3, 3),
        (3, 1, 3),
        (3, 3, 2),
        (3, 1, 2),
    ] {
        for i in 0..5 {
            t.write(node(i), &[0x10 + i as u8; 0x40]);
        }
        let me = node(0);
        t.set_u32(me + 0x14, if parent == 0 { 0 } else { node(parent) });
        t.set_u32(me + 4, node(prev));
        t.set_u32(me + 8, node(next));
        let c = Call {
            eax: 0xeeee,
            ecx: me,
            edx: 0,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, _, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::leave(g, fr.ecx, fr.eax))
        });
    }
}

#[test]
fn register_parts_runs_both_loops() {
    let Some(mut t) = sa::twin(53) else { return };
    let f = t.at(0x2a00);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (count, table, lock, banks) = (w(&t, 0x0e), w(&t, 0x16), w(&t, 0x43), w(&t, 0x76));
    let recs = SCRATCH + 0x1000;
    t.set_u32(table, recs);
    for i in 0..4 {
        t.set_u32(recs + 0x74 * i + 0x30, 0x0300_0000 + i);
    }
    t.set_u32(lock, SCRATCH + 0x800);
    rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[0] ^ a[1] << 8);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 779, 4, |m, a| {
        // the lock is held while slot 779 runs
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        a[0]
    });
    for (n, b) in [(0u32, 0u32), (3, 0), (0, 1), (2, 2)] {
        t.set_u32(count, n);
        t.set_u32(banks, b);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::register_parts(g, img, fr.at, fr.eax))
        });
    }
}

#[test]
fn records_init_takes_a_node() {
    let Some(mut t) = sa::twin(132) else { return };
    let f = t.at(0x31d0);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (count, free, head, tail) = (w(&t, 0x20), w(&t, 0x3e), w(&t, 0xa6), w(&t, 0xb7));
    let block = SCRATCH + 0x1_0000;
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (n, empty) in [(1u32, true), (2, false), (3, true), (3, false)] {
        t.write(block, &vec![0x77; 0x200 * 0x74]);
        for i in 1..=n {
            t.write(node(i), &[0x33; 0x40]);
            let next = if i == n { 1 } else { i + 1 };
            let prev = if i == 1 { n } else { i - 1 };
            t.set_u32(node(i), node(prev));
            t.set_u32(node(i) + 4, node(next));
        }
        t.set_u32(free, node(1));
        t.set_u16(count, n as u16);
        t.write(node(9), &[0x99; 0x40]);
        t.set_u32(head, if empty { 0 } else { node(9) });
        t.set_u32(tail, node(9));
        t.check(f, Call::this(block), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::records_init(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn small_entries() {
    let Some(mut t) = sa::twin(187) else { return };
    let p = SCRATCH + 0x100;
    for v in [0u32, 4, 0xffff_fffb] {
        t.set_u32(p, v);
        t.check_entry(t.at(0x1250), Call::default(), &[p], Eax::Checked);
    }
    rec(&mut t, 435, 1, |_, _| 0x435);
    rec(&mut t, 437, 1, |_, _| 0x437);
    let h = t.img.word(t.at(0x1480) + 0x07);
    t.set_u32(h, 0x00ab_cdef);
    t.check_entry(t.at(0x1480), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x14e0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(45) else { return };
    rec(&mut t, 804, 1, |_, _| 0x804);
    let h = t.img.word(t.at(0x1e60) + 0x07);
    t.set_u32(h, 0x0012_3456);
    t.check_entry(t.at(0x1e60), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(9) else { return };
    let f = t.at(0x1900);
    let b = t.img.word(f + 0x05);
    t.write(b, &[0xa5; 0x40]);
    let src = t.img.word(f + 0x1b);
    t.set_u32(src, 0x0a0b_0c0d);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn me_block_fills() {
    let Some(mut t) = sa::twin(52) else { return };
    let f = t.at(0x2040);
    let battle = t.img.word(f + 0x05);
    let me = t.img.word(f + 0x13);
    t.set_u32(me, 0x15);
    rec(&mut t, 413, 1, |_, a| 0x4130_0000 ^ a[0]);
    rec(&mut t, 215, 1, |_, a| 0x2150_0000 ^ a[0]);
    let blk = t.img.word(f + 0x0e);
    for b in [0u32, 1, 0x100] {
        t.write(blk, &[0x5a; 0x20]);
        t.set_u32(battle, b);
        t.check(
            f,
            Call::default(),
            &[0x00a1_0000],
            Eax::Checked,
            |g, img, fr| {
                let a = fr.arg(g, 0);
                ffx_magic::Ret::eax(ffx_magic::sa_misc::me_block(g, img, fr.at, a))
            },
        );
    }
}

#[test]
fn script_run_by_mode() {
    let Some(mut t) = sa::twin(253) else { return };
    let f = t.at(0x42c0);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (s, table, step) = (w(&t, 0x1c), w(&t, 0x17), w(&t, 0x68));
    for i in 0..8 {
        t.set_u32(table + 4 * i, TRAPS);
    }
    // the command: logs its call, moves on 4 and sets the mode from the
    // cells at SCRATCH + 0x200 (one per call) when it runs out
    rec(&mut t, 0, 2, move |m, a| {
        let k = m.r32(SCRATCH + 0x2_0000);
        let mode = m.r32(SCRATCH + 0x200 + 4 * (k - 1));
        let v = m.r32(s + 0x54);
        m.w32(s + 0x54, v & 0xff00_ffff | (mode & 0xff) << 16);
        a[1] + 4
    });
    rec(&mut t, 2, 1, |_, _| 0x22);
    rec(&mut t, 32, 1, |_, _| 0x33);
    t.set_u32(SCRATCH + 0x300, 3);
    t.set_u32(step, SCRATCH + 0x300);
    let prog = SCRATCH + 0x400;
    for i in 0..8u32 {
        t.set_u32(prog + 4 * i, i % 3);
    }
    for (start, modes, wait, cb) in [
        (0u8, [1u32, 1, 0], 0u16, false),
        (1, [1, 1, 0], 0, true),
        (1, [2, 0, 0], 10, true),
        (2, [0, 0, 0], 3, true),
        (2, [0, 0, 0], 2, false),
        (2, [0, 0, 0], 0xffff, true),
        (1, [3, 0, 0], 0, true),
        (3, [0, 0, 0], 0, false),
        (1, [1, 7, 0], 0, true),
    ] {
        t.set_u32(SCRATCH + 0x2_0000, 0);
        for (i, m) in (0u32..).zip(modes) {
            t.set_u32(SCRATCH + 0x200 + 4 * i, m);
        }
        t.write(s, &[0; 0x60]);
        t.set_u32(s, prog);
        t.set_u8(s + 0x56, start);
        t.set_u16(s + 0x54, wait);
        t.set_u32(s + 0x24, TRAPS + 16 * 2);
        t.set_u32(s + 0x28, if cb { TRAPS + 16 * 32 } else { 0 });
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_script::run(g, img, fr.at))
        });
    }
}

#[test]
fn link_part_both_ways() {
    let Some(mut t) = sa::twin(9) else { return };
    let f = t.at(0x2af0);
    let lock = t.img.word(f + 0x4f);
    t.set_u32(lock, SCRATCH + 0x800);
    let recd = SCRATCH + 0x100;
    let s = SCRATCH + 0x200;
    let owner = SCRATCH + 0x300;
    t.set_u32(recd, 0x0123_4567);
    t.set_u16(recd + 8, 0x9abc);
    t.set_u32(owner + 0x1c, 0x00c0_ffee);
    // slot 941 answers the cell; v = i16 + it points at SCRATCH + 0x3000
    rec(&mut t, 941, 1, |m, _| m.r32(SCRATCH + 0x900));
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(&mut t, 773, 4, |m, _| m.r32(SCRATCH + 0x904));
    for s in [774u32, 775, 776, 777, 778] {
        let n = [5usize, 2, 1, 1, 6][(s - 774) as usize];
        rec(&mut t, s, n, move |m, a| {
            // the lock as the callee sees it
            m.w32(SCRATCH + 0x908 + 4 * (s - 774), m.r32(SCRATCH + 0x800));
            s << 16 ^ a[0]
        });
    }
    let v = SCRATCH + 0x3000;
    t.set_u32(v + 4, 0x10);
    t.write(v + 0x14, &[0x5d]);
    for (off, known) in [(8i16, 0u32), (-0x20, 0), (8, 1), (0, 7)] {
        t.set_u16(s, off as u16);
        t.set_u32(SCRATCH + 0x900, v.wrapping_sub(i32::from(off) as u32));
        t.set_u32(SCRATCH + 0x904, known);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        let c = Call {
            eax: 0,
            ecx: recd,
            edx: s,
            ..Call::default()
        };
        t.check(f, c, &[owner, 0x0000_0042], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::link_part(g, img, fr))
        });
    }
}

#[test]
fn host_start_calls() {
    let Some(mut t) = sa::twin(84) else { return };
    for (s, n) in [
        (353u32, 1usize),
        (397, 1),
        (332, 1),
        (267, 1),
        (933, 4),
        (785, 1),
        (543, 1),
        (250, 2),
        (550, 1),
    ] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    rec(&mut t, 549, 1, |m, _| m.r32(SCRATCH + 0x900));
    let f = t.at(0x11f0);
    let k = t.img.word(f + 0x7a);
    t.set_u32(k, 0.75f32.to_bits());
    for answer in [0u32, 1] {
        t.set_u32(SCRATCH + 0x900, answer);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn timeline_frames() {
    let Some(mut t) = sa::twin(188) else { return };
    let f = t.at(0x3b30);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (live, events, flags, count, now) = (
        w(&t, 0x06),
        w(&t, 0x17),
        w(&t, 0x1f),
        w(&t, 0x25),
        w(&t, 0x41),
    );
    let ev = SCRATCH + 0x1000;
    t.set_u32(events, ev);
    // the callbacks: three unused function slots; on-frame answers the cell
    rec(&mut t, 0, 1, |_, a| 0x0100_0000 ^ a[0]);
    rec(&mut t, 2, 1, |m, _| m.r32(SCRATCH + 0x900));
    rec(&mut t, 32, 1, |_, a| 0x0300_0000 ^ a[0]);
    rec(&mut t, 253, 0, |m, _| m.r32(SCRATCH + 0x904));
    // (start, length, which callbacks) per event
    let table: [(u32, u32, u32); 6] = [
        (5, 3, 0b111),
        (7, 1, 0b101),
        (5, u32::MAX, 0b111),
        (5, u32::MAX, 0b001),
        (9, 2, 0b111),
        (3, 3, 0b110),
    ];
    for (i, &(start, len, cbs)) in (0u32..).zip(&table) {
        let e = ev + 0x18 * i;
        t.set_u32(e, start);
        t.set_u32(e + 4, len);
        for (k, slot) in [(0u32, 0u32), (1, 2), (2, 32)] {
            let v = if cbs & 1 << k != 0 {
                TRAPS + 16 * slot
            } else {
                0
            };
            t.set_u32(e + 8 + 4 * k, v);
        }
        t.set_u32(e + 0x14, 0x00a0_0000 + i);
    }
    for (frame, alive, on_frame, step) in [
        (4u32, 6u32, 1u32, 1u32),
        (5, 6, 0, 1),
        (5, 6, 1, 0),
        (7, 6, 1, 1),
        (7, 1, 1, 1),
        (10, 6, 0, 1),
        (5, 0, 1, 1),
    ] {
        for i in 0..6 {
            t.set_u32(flags + 8 * i, 1);
        }
        t.set_u32(flags + 8, 0);
        t.set_u32(live, alive);
        t.set_u32(count, 6);
        t.set_u32(now, frame);
        t.set_u32(SCRATCH + 0x900, on_frame);
        t.set_u32(SCRATCH + 0x904, step);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::timeline(g, img, fr.at))
        });
    }
}

#[test]
fn drift_and_reset() {
    let Some(mut t) = sa::twin(162) else { return };
    let f = t.at(0x5ea0);
    let obj = SCRATCH + 0x100;
    rec(&mut t, 314, 1, |_, a| 0x3140_0000 ^ a[0]);
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x2_0000 + 0x80 * 314);
        f64::from(f32::from_bits(a[0])) * (f64::from(k) * 0.37 - 1.0)
    });
    for (n1, n2) in [(0i32, 0i32), (1, 0), (0, 2), (3, 2), (-1, 1)] {
        t.write(obj, &[0x3f; 0x140]);
        t.set_u32(obj + 0x128, 0.5f32.to_bits());
        t.set_u32(obj + 0x12c, (-2.0f32).to_bits());
        t.set_u32(obj + 0x130, n1 as u32);
        t.set_u32(obj + 0x134, n2 as u32);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: obj,
            edx: 0,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::drift(g, img, fr.at, fr.ecx, fr.eax))
        });
    }
    t.write(obj, &[0xa5; 0x140]);
    t.check(
        t.at(0x9590),
        Call::this(obj),
        &[],
        Eax::Checked,
        |g, _, fr| ffx_magic::Ret::eax(ffx_magic::sa_misc::reset_transform_0(g, fr.ecx)),
    );
}

#[test]
fn node_append_links() {
    let Some(mut t) = sa::twin(42) else { return };
    let f = t.at(0x7de0);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (parent, kids) in [(false, 0u32), (true, 0), (true, 1), (true, 3)] {
        for i in 0..6 {
            t.write(node(i), &[0x20 + i as u8; 0x40]);
        }
        let p = node(1);
        let last = if kids == 0 { p } else { node(1 + kids) };
        t.set_u32(p + 0x10, last);
        t.set_u16(p + 0x18, 0xfffe);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: node(0),
            edx: if parent { p } else { 0 },
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::node_append(
                g, img, fr.at, fr.ecx, fr.edx, fr.eax,
            ))
        });
    }
}

#[test]
fn fade_step_counts_down() {
    let Some(mut t) = sa::twin(353) else { return };
    let f = t.at(0x23f0);
    let obj = SCRATCH + 0x100;
    for n in [1u32, 2, 31, 60, 0, 0x8000_0000, 0x7fff_ffff, 0xffff_fff0] {
        t.write(obj, &[0x5a; 0x130]);
        t.set_u32(obj + 0x120, n);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn host_start_b_calls() {
    let Some(mut t) = sa::twin(197) else { return };
    let f = t.at(0x1000);
    for (s, n) in [
        (556u32, 1usize),
        (555, 1),
        (331, 1),
        (720, 2),
        (718, 1),
        (222, 4),
        (221, 4),
    ] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.125);
    // pool A holds old payloads the start-up clears
    let pool = t.img.word(f + 0x7b);
    t.write(pool, &vec![0x77; 12 * 0x3ff]);
    t.check_entry(f, Call::default(), &[0x00a1_0000], Eax::Checked);
}

#[test]
fn reset_transform_3_fills() {
    let Some(mut t) = sa::twin(92) else { return };
    let obj = SCRATCH + 0x200;
    t.write(obj, &[0xa5; 0x140]);
    let c = Call {
        eax: 0,
        ecx: obj,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    t.check(
        t.at(0x26e0),
        c,
        &[0x0a00_0000, 0x0a11_0000],
        Eax::Checked,
        |g, _, fr| {
            let (a, b) = (fr.arg(g, 0), fr.arg(g, 1));
            ffx_magic::Ret::eax(ffx_magic::sa_misc::reset_transform_3(
                g, fr.ecx, fr.edx, a, b,
            ))
        },
    );
}

/// The part refresh's stubs and globals (matrices as words, the part
/// pointer and the lock in scratch).
fn refresh_setup(t: &mut Twin, f: u32, ops: [u32; 7]) {
    let [x68, x74, x70, x60, part_p, lock, _] = ops;
    for (i, op) in [x68, x74, x70, x60].into_iter().enumerate() {
        let g = t.img.word(f + op);
        t.set_u32(g, 0x0600_0000 + i as u32);
    }
    let p = t.img.word(f + part_p);
    t.set_u32(p, SCRATCH + 0x700);
    let l = t.img.word(f + lock);
    t.set_u32(l, SCRATCH + 0x800);
    rec(t, 258, 1, |_, a| 0x2580_0000 ^ a[0]);
    rec(t, 231, 4, |_, a| 0x2310_0000 ^ a[0]);
    rec(t, 230, 6, |_, a| 0x2300_0000 ^ a[0]);
    rec(t, 712, 8, |m, _| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        m.r32(SCRATCH + 0x900)
    });
}

#[test]
fn part_refresh_both() {
    let Some(mut t) = sa::twin(45) else { return };
    let f = t.at(0x1510);
    refresh_setup(&mut t, f, [0x16, 0x21, 0x2d, 0x40, 0x4d, 0x5d, 0]);
    for r in [0u32, 1] {
        t.set_u32(SCRATCH + 0x900, r);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        t.check_entry(f, Call::default(), &[SCRATCH + 0x600], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = sa::twin(341) else { return };
    let f = t.at(0x10d0);
    refresh_setup(&mut t, f, [0x1f, 0x2a, 0x36, 0x49, 0x56, 0x63, 0]);
    let obj = SCRATCH + 0x600;
    for (flag, r) in [(0u16, 1u32), (1, 0), (0x8000, 1)] {
        t.set_u16(obj + 0x1e, flag);
        t.set_u32(SCRATCH + 0x900, r);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: obj,
            edx: 0,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::part_refresh_b(
                g, img, fr.at, fr.ecx, fr.eax,
            ))
        });
    }
}

#[test]
fn hook_callback_passes_the_dll_callback() {
    let Some(mut t) = sa::twin(195) else { return };
    rec(&mut t, 432, 0, |_, _| 0x432);
    rec(&mut t, 450, 2, |_, a| 0x4500_0000 ^ a[0]);
    t.check_entry(t.at(0x13f0), Call::default(), &[0x00a1_0000], Eax::Checked);
}

#[test]
fn script_extras() {
    let Some(mut t) = sa::twin(162) else { return };
    let f = t.at(0x61e0);
    let list = t.img.word(f + 0x10);
    t.set_u32(list, SCRATCH + 0x1800);
    t.write(SCRATCH + 0x1800, &[0x05, 0x25]);
    let objs = t.img.word(f + 0x1b);
    t.set_u32(objs + 4 * 5, 0x0b00_0005);
    t.set_u32(objs + 4 * 0x25, 0x0b00_0025);
    let me = t.img.word(f + 0x09);
    t.set_u32(me, 0x0bad_0b1e);
    rec(&mut t, 237, 4, |_, a| 0x2370_0000 ^ a[0]);
    let out = SCRATCH + 0x200;
    for i in [u32::MAX, 0, 1] {
        t.write(out, &[0x66; 0x10]);
        let c = Call {
            eax: 0,
            ecx: i,
            edx: 0x00ed_0000,
            ..Call::default()
        };
        t.check(f, c, &[0x00a0_0000, out], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_script::actor_vector(g, img, fr))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(446) else { return };
    for (rva, off) in [(0x4050u32, 0xd74cu32), (0x21a0, 0x5c08)] {
        let f = t.at(rva);
        let fx_g = t.img.word(f + 0x05);
        let fx = SCRATCH + 0x1_0000;
        t.set_u32(fx_g, fx);
        let obj = SCRATCH + 0x100;
        for (a, b) in [(3u32, 3u32), (3, 4)] {
            t.write(obj, &[0x11; 0x60]);
            t.set_u32(fx + off, a);
            t.set_u32(fx + off + 4, b);
            t.set_u32(fx + off + 8, 7);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}

#[test]
fn magic_0042_node_helpers() {
    let Some(mut t) = sa::twin(42) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0x4ef0);
    for (k, &(c, p, cp)) in sa::RINGS.iter().enumerate() {
        sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
        let parent = sa::node(6);
        t.write(parent, &[0x44; 0x40]);
        t.set_u32(parent + 0x20, 2.5f32.to_bits());
        t.set_u32(sa::node(0) + 0x14, parent);
        t.set_u32(sa::node(0) + 0x1c, (-0.75f32).to_bits());
        sa::check_this_thunk(
            &mut t,
            f,
            0x4f50,
            ffx_magic::sa_misc::add_to_parent,
            Eax::Checked,
        );
    }
    let f = t.at(0x7e20);
    let node = |i: u32| SCRATCH + 0x8000 + 0x200 * i;
    for (parent, kids) in [(false, 0u32), (true, 0), (true, 2)] {
        for i in 0..5 {
            t.write(node(i), &[0x20 + i as u8; 0x200]);
        }
        let p = node(1);
        let last = if kids == 0 { p } else { node(1 + kids) };
        t.set_u32(p + 0x10, last);
        let c = Call {
            eax: 0,
            ecx: node(0),
            edx: if parent { p } else { 0 },
            ..Call::default()
        };
        t.check(
            f,
            c,
            &[0x0a00_0000, 0x0a11_0000, 0x0a22_0000, 0x0a33_0000],
            Eax::Checked,
            |g, img, fr| ffx_magic::Ret::eax(ffx_magic::sa_misc::node_append_owned(g, img, fr)),
        );
    }
    let f = t.at(0x2a60);
    let args = SCRATCH + 0x100;
    let n = SCRATCH + 0x200;
    for mode in [0u32, 1, 0x100] {
        t.write(n, &[0x3f; 0xb0]);
        for i in 0..0x10 {
            t.set_u32(n + 0x1c + 4 * i, (1.5f32 * i as f32 - 3.0).to_bits());
            t.set_u32(n + 0x4c + 4 * i, (0.25f32 * i as f32 + 1.0).to_bits());
        }
        t.set_u32(n + 0xac, 0xffff_0f0a);
        t.set_u32(args, n);
        t.set_u32(args + 4, mode);
        t.set_u32(args + 8, 0x00a8_0000);
        t.set_u32(args + 0xc, 0x00ac_0000);
        t.check(f, Call::this(args), &[], Eax::Checked, |g, _, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::move_start(g, fr.ecx))
        });
    }
}

#[test]
fn reset_transform_14_fills() {
    let Some(mut t) = sa::twin(10) else { return };
    let obj = SCRATCH + 0x200;
    t.write(obj, &[0xa5; 0x140]);
    let c = Call {
        eax: 0,
        ecx: obj,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    t.check(
        t.at(0x1bc0),
        c,
        &[0x0a00_0000, 0x0a11_0000],
        Eax::Checked,
        |g, _, fr| {
            let b = fr.arg(g, 1);
            ffx_magic::Ret::eax(ffx_magic::sa_misc::reset_transform_14(g, fr.ecx, fr.edx, b))
        },
    );
}

#[test]
fn insert_with_group() {
    let Some(mut t) = sa::twin(159) else { return };
    let f = t.at(0x6270);
    let node = |i: u32| SCRATCH + 0x1000 + 0x200 * i;
    for (parent, kids) in [(false, 0u32), (true, 0), (true, 1)] {
        for i in 0..4 {
            t.write(node(i), &[0x20 + i as u8; 0x200]);
        }
        let p = node(1);
        let first = if kids == 0 { p } else { node(2) };
        t.set_u32(p + 0x10, first);
        let c = Call {
            eax: 0,
            ecx: node(0),
            edx: if parent { p } else { 0 },
            ..Call::default()
        };
        t.check(
            f,
            c,
            &[0x0a00_0000, 0x0a11_0000, 0x0a22_0000, 0x0a33_0000],
            Eax::Checked,
            |g, img, fr| ffx_magic::Ret::eax(ffx_magic::sa_misc::insert_with(g, img, fr)),
        );
    }
}

#[test]
fn records_init_400_takes() {
    let Some(mut t) = sa::twin(162) else { return };
    let f = t.at(0x81f0);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (count, free, head, tail) = (w(&t, 0x20), w(&t, 0x3e), w(&t, 0xa6), w(&t, 0xb7));
    let block = SCRATCH + 0x1_0000;
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (n, empty) in [(1u32, true), (2, false)] {
        t.write(block, &vec![0x77; 0x400 * 0x74]);
        for i in 1..=n {
            let next = if i == n { 1 } else { i + 1 };
            let prev = if i == 1 { n } else { i - 1 };
            t.set_u32(node(i), node(prev));
            t.set_u32(node(i) + 4, node(next));
        }
        t.set_u32(free, node(1));
        t.set_u16(count, n as u16);
        t.set_u32(head, if empty { 0 } else { node(9) });
        t.set_u32(tail, node(9));
        t.check(f, Call::this(block), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::records_init_400(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn ring_take_finds_a_free_slot() {
    let Some(mut t) = sa::twin(242) else { return };
    let f = t.at(0x1a60);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (rover, count, base) = (w(&t, 0x01), w(&t, 0x09), w(&t, 0x16));
    let ring = SCRATCH + 0x1000;
    t.set_u32(base, ring);
    for (n, start, used) in [
        (0i32, 0u32, 0u32),
        (4, 0, 0),
        (4, 2, 0b0100),
        (4, 3, 0b1000),
        (4, 1, 0b1111),
        (3, 2, 0b011),
    ] {
        t.write(ring, &vec![0x55; 0x50 * 5]);
        for i in 0..5 {
            t.set_u32(ring + 0x50 * i + 0x48, (used >> i) & 1);
        }
        t.set_u32(count, n as u32);
        t.set_u32(rover, start);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::ring_take(g, img, fr.at))
        });
    }
}

#[test]
fn release_part_entry() {
    let Some(mut t) = sa::twin(292) else { return };
    let f = t.at(0x1210);
    let p = t.img.word(f + 0x01);
    t.set_u32(p, SCRATCH + 0x100);
    rec(&mut t, 713, 1, |_, a| 0x7130_0000 ^ a[0]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn sprite_init_and_fade() {
    let Some(mut t) = sa::twin(206) else { return };
    let out = SCRATCH + 0x100;
    let hdr = SCRATCH + 0x400;
    t.write(hdr, &[0x99; 0x20]);
    t.set_u16(hdr + 0x10, 0xfff0);
    t.set_u16(hdr + 0x12, 0x1234);
    rec(&mut t, 942, 1, |_, a| 0x9420_0000 ^ a[0]);
    // slot 941: the header for every call, after a counter
    rec(&mut t, 941, 1, move |_, _| hdr);
    for res in [0x00a1_0000u32, 0] {
        t.write(out, &[0xa5; 0x40]);
        let c = Call {
            eax: 0,
            ecx: out,
            edx: res,
            ..Call::default()
        };
        t.check(t.at(0x2670), c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::sprite_init(
                g, img, fr.at, fr.ecx, fr.edx,
            ))
        });
    }
    rec(&mut t, 248, 3, |_, a| 0x2480_0000 ^ a[2]);
    let f = t.at(0x2b20);
    let o = SCRATCH + 0x800;
    for flags in [0u32, 1, 2, 3, 4, 5, 8, 9, 0xf, 0xe] {
        for (time, len) in [(5i32, 10i32), (1, 4), (10, 10), (0, 3), (7, 0)] {
            t.write(o, &[0x3c; 0xa0]);
            t.set_u32(o + 0x2c, flags);
            t.set_u32(o + 0x28, time as u32);
            t.set_u32(o + 0x24, len as u32);
            for (i, v) in [
                (0x6c, 10i32),
                (0x70, -20),
                (0x74, 300),
                (0x78, 0),
                (0x7c, 250),
                (0x80, 40),
                (0x84, -7),
                (0x88, 255),
            ] {
                t.set_u32(o + i, v as u32);
            }
            t.set_u32(o + 0x8c, 0.5f32.to_bits());
            t.set_u32(o + 0x90, 2.0f32.to_bits());
            t.check(f, Call::this(o), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::Ret::eax(ffx_magic::sa_misc::sprite_fade(g, img, fr.at, fr.ecx))
            });
        }
    }
}

#[test]
fn magic_0201_and_0565_entries() {
    let Some(mut t) = sa::twin(201) else { return };
    rec(&mut t, 250, 2, |_, _| 0x250);
    t.check_entry(t.at(0x48e0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(565) else { return };
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[0]);
    rec(&mut t, 663, 2, |_, a| 0x6630_0000 ^ a[0]);
    rec(&mut t, 628, 1, |_, _| 0x628);
    rec(&mut t, 311, 2, |_, a| 0x3110_0000 ^ a[0]);
    rec(&mut t, 219, 2, |m, a| {
        let k = m.r32(SCRATCH + 0x2_0000 + 0x80 * 219);
        0x2190_0000 ^ a[0] ^ a[1] << 4 ^ k << 8
    });
    rec(&mut t, 257, 2, |_, a| 0x2570_0000 ^ a[0]);
    rec(&mut t, 38, 3, |_, a| 0x0380_0000 ^ a[0]);
    rec(&mut t, 357, 3, |_, a| 0x3570_0000 ^ a[2]);
    rec(&mut t, 356, 3, |_, a| 0x3560_0000 ^ a[2]);
    t.check_entry(t.at(0x6110), Call::default(), &[], Eax::Checked);
    let h = t.img.word(t.at(0x17b0) + 0x16);
    t.set_u32(h, 0x0000_0042);
    t.check_entry(t.at(0x17b0), Call::default(), &[], Eax::Checked);
    let o = t.img.word(t.at(0x78b0) + 0x09);
    t.set_u32(o, 0x00b0_0000);
    t.check_entry(t.at(0x78b0), Call::default(), &[], Eax::Checked);
    t.check(
        t.at(0x6c90),
        Call::default(),
        &[],
        Eax::Checked,
        |g, img, fr| ffx_magic::Ret::eax(ffx_magic::sa_misc::part_ranges(g, img, fr.at)),
    );
    let tm = SCRATCH + 0x100;
    t.write(tm, &[0x5a; 0x60]);
    let c = Call {
        eax: 0x00ea_0000,
        ecx: tm,
        edx: 0,
        ..Call::default()
    };
    t.check(t.at(0x19b0), c, &[], Eax::Checked, |g, _, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::timers_reset(g, fr.ecx, fr.eax))
    });
}

#[test]
fn magic_0446_camera_and_start() {
    let Some(mut t) = sa::twin(446) else { return };
    let f = t.at(0x4c70);
    let fx_g = t.img.word(f + 0x08);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(fx_g, fx);
    let m = t.img.word(f + 0x22);
    t.set_u32(m, 0x00ab_0000);
    rec(&mut t, 262, 4, |m, a| {
        m.w32(a[1], 0x3f00_0000);
        0x262
    });
    rec(&mut t, 333, 1, |_, _| 0x333);
    let kf = |t: &Twin, off: u32| f32::from_bits(t.u32(t.img.word(f + off)));
    let edge = f64::from(kf(&t, 0x4f) - kf(&t, 0x5e));
    let low = f64::from(kf(&t, 0x4f) - kf(&t, 0x72));
    for angle in [0.0f64, 1.0, -3.0, 7.0, 100.0, -100.0, edge, low] {
        t.stub_st0(333, move |_, _| angle);
        t.write(fx + 0x2f80, &[0x5a; 0x10]);
        t.write(fx + 0xebf0, &[0x5a; 0x10]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::camera_target(g, img, fr.at))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(446) else { return };
    let f = t.at(0x1000);
    for (s, n) in [
        (556u32, 1usize),
        (555, 1),
        (331, 1),
        (720, 2),
        (718, 1),
        (222, 4),
        (221, 4),
        (215, 1),
        (413, 1),
    ] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    rec(&mut t, 168, 1, |_, _| SCRATCH + 0x4000);
    // the object block's byte list: none
    rec(&mut t, 218, 2, |m, a| {
        m.w32(a[1], 0);
        SCRATCH + 0x3000
    });
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.25);
    t.check_entry(f, Call::default(), &[0x00a1_0000], Eax::Checked);
}

#[test]
fn magic_0070_colour_and_follow() {
    let Some(mut t) = sa::twin(70) else { return };
    let f = t.at(0x8640);
    rec(&mut t, 678, 1, |_, a| 0x6780_0000 ^ a[0]);
    let node = |i: u32| SCRATCH + 0x1000 + 0x80 * i;
    let args = SCRATCH + 0x100;
    for (parent, kids) in [(false, 0u32), (true, 0), (true, 2)] {
        for i in 0..5 {
            t.write(node(i), &[0x20 + i as u8; 0x80]);
        }
        let p = node(1);
        let last = if kids == 0 { p } else { node(1 + kids) };
        t.set_u32(p + 0x10, last);
        t.set_u32(args, node(0));
        t.set_u32(args + 4, if parent { p } else { 0 });
        for i in 2..7 {
            t.set_u32(args + 4 * i, 0x0a00_0000 + i);
        }
        t.check(f, Call::this(args), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::colour_node_init(g, img, fr.at, fr.ecx))
        });
    }
    sa::handler_stubs(&mut t);
    rec(&mut t, 942, 1, |_, a| 0x9420_0000 ^ a[0]);
    let f = t.at(0x88d0);
    for (k, &(c, p, cp)) in sa::RINGS.iter().enumerate() {
        for second in [false, true] {
            sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
            for (off, i) in [(0x3c, 6), (0x40, 7)] {
                t.write(sa::node(i), &[0x60 + i as u8; 0x40]);
                for j in 1..0x10 {
                    t.set_u32(sa::node(i) + 4 * j, 0x0600_0000 + 0x100 * i + j);
                }
                t.set_u32(sa::node(i), sa::VTABLE_2);
                let v = if off == 0x40 && !second {
                    0
                } else {
                    sa::node(i)
                };
                t.set_u32(sa::node(0) + off, v);
            }
            t.check_entry(
                f,
                Call {
                    eax: 0x1111_2222,
                    ecx: 0x3333,
                    edx: 0,
                    ..Call::default()
                },
                &[sa::CURSOR],
                Eax::Checked,
            );
        }
    }
}

#[test]
fn register_16_handles() {
    let Some(mut t) = sa::twin(543) else { return };
    let f = t.at(0x5840);
    let fx_g = t.img.word(f + 0x03);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(fx_g, fx);
    t.set_u32(fx + 0x59b0, 0x0059_b000);
    let lock = t.img.word(f + 0x37);
    t.set_u32(lock, SCRATCH + 0x800);
    t.set_u32(SCRATCH + 0x800, 0x5555);
    rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 779, 4, |m, a| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        a[0]
    });
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::register_16(g, img, fr.at))
    });
}

#[test]
fn facing_int_both_ways() {
    let Some(mut t) = sa::twin(6) else { return };
    let f = t.at(0x65b0);
    let obj = SCRATCH + 0x100;
    let out = SCRATCH + 0x200;
    rec(&mut t, 333, 1, |_, _| 0x333);
    rec(&mut t, 601, 2, |m, a| {
        let v = m.r32(SCRATCH + 0x900);
        if v != 0 {
            m.w32(a[1], v);
        }
        0x601
    });
    for (flag, angle, local) in [
        (1u8, 0.5f64, 0u32),
        (1, -2.0, 0),
        (0, 0.0, 1.25f32.to_bits()),
        (0, 0.0, (-3.5f32).to_bits()),
        // slot 601 writes nothing: the local still holds the object
        (0, 0.0, 0),
    ] {
        t.stub_st0(333, move |_, _| angle);
        t.set_u32(SCRATCH + 0x900, local);
        t.write(obj, &[0x11; 0x100]);
        t.set_u8(obj + 0x90, flag);
        t.write(out, &[0x22; 0x10]);
        let c = Call {
            eax: 0,
            ecx: obj,
            edx: out,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::facing_int(
                g, img, fr.at, fr.ecx, fr.edx, fr.esp,
            ))
        });
    }
}

#[test]
fn model_id_and_kind_check() {
    let Some(mut t) = sa::twin(155) else { return };
    let f = t.at(0x3ec0);
    rec(&mut t, 20, 1, |_, a| 0x0200_0000 ^ a[0]);
    rec(&mut t, 411, 1, |_, a| 0x4110_0000 ^ a[0]);
    let this = SCRATCH + 0x100;
    let o = SCRATCH + 0x200;
    for flag in [0u8, 1] {
        t.write(o, &[0x33; 0x100]);
        t.set_u8(o + 0x90, flag);
        t.set_u32(this, o);
        t.set_u32(this + 4, 0x4444);
        t.check(f, Call::this(this), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::model_id(g, img, fr.at, fr.ecx))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(126) else { return };
    let f = t.at(0x1a60);
    let me = t.img.word(f + 0x09);
    let list = t.img.word(f + 0x10);
    t.set_u32(me, 0x11);
    t.set_u32(list, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x05, 0x25, 0x07]);
    let kinds = t.img.word(f + 0x30);
    // slot 390 answers the cell; slot 534 logs
    rec(&mut t, 390, 1, |m, _| m.r32(SCRATCH + 0x900));
    rec(&mut t, 534, 1, |_, a| 0x5340_0000 ^ a[0]);
    let first = t.u32(kinds);
    let second = t.u32(kinds + 4);
    for i in [u32::MAX, 0, 1] {
        for kind in [first, second, 0x7777_7777] {
            t.set_u32(SCRATCH + 0x900, kind);
            t.check(f, Call::this(i), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::Ret::eax(ffx_magic::sa_misc::actor_kind_check(g, img, fr.at, fr.ecx))
            });
        }
    }
}

#[test]
fn scale_rotate_matrix() {
    let Some(mut t) = sa::twin(165) else { return };
    let f = t.at(0x6b70);
    let m = SCRATCH + 0x100;
    let v = SCRATCH + 0x200;
    let h = t.img.word(f + 0x1d);
    t.set_u32(h, 0x00ab_0000);
    rec(&mut t, 235, 1, |_, _| 0x235);
    for (s, k) in [(277u32, 2.0f64), (15, 0.5), (16, -1.5), (17, 3.0)] {
        rec(&mut t, s, 1, move |_, a| s << 16 ^ a[0]);
        t.stub_st0(s, move |_, _| k);
    }
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |_, a| s << 16 ^ a[2]);
    }
    t.write(m, &[0x3c; 0x40]);
    for (i, x) in [0.1f32, -0.2, 0.3].iter().enumerate() {
        t.set_u32(v + 4 * i as u32, x.to_bits());
    }
    let c = Call {
        eax: 0,
        ecx: 0,
        edx: m,
        ..Call::default()
    };
    t.check(f, c, &[v], Eax::Checked, |g, img, fr| {
        let v = fr.arg(g, 0);
        ffx_magic::Ret::eax(ffx_magic::sa_misc::scale_rotate(g, img, fr.at, fr.edx, v))
    });
}

#[test]
fn sound_start_record() {
    let Some(mut t) = sa::twin(515) else { return };
    let f = t.at(0x26b0);
    rec(&mut t, 228, 2, |_, a| 0x2280_0000 ^ a[0] ^ a[1] << 4);
    let p = SCRATCH + 0x100;
    for id in [0xffu32, 0, 7, 0x1ff] {
        t.write(p, &[0x5a; 0x20]);
        t.set_u32(p, id);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: p,
            edx: 0x0042,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::sound_start(
                g, img, fr.at, fr.ecx, fr.edx,
            ))
        });
    }
}

#[test]
fn actor_mode_and_scale() {
    let Some(mut t) = sa::twin(266) else { return };
    let f = t.at(0x28d0);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (list, me_obj, objs, battle, me) = (
        w(&t, 0x15),
        w(&t, 0x21),
        w(&t, 0x2e),
        w(&t, 0x4c),
        w(&t, 0x56),
    );
    t.set_u32(list, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x05, 0x25, 0x20, 0x1f]);
    t.set_u32(me_obj, 0x0bad_0b1e);
    for n in [5u32, 0x25, 0x20, 0x1f] {
        t.set_u32(objs + 4 * n, 0x0b00_0000 + n);
    }
    t.set_u32(me, 0x11);
    for (s, n) in [(210u32, 2usize), (211, 2), (217, 1), (255, 1)] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    for b in [0u32, 1] {
        t.set_u32(battle, b);
        for i in [u32::MAX, 0, 1, 2, 3] {
            for mode in [0u32, 3] {
                let c = Call {
                    eax: 0,
                    ecx: i,
                    edx: mode,
                    ..Call::default()
                };
                t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                    ffx_magic::Ret::eax(ffx_magic::sa_script::actor_mode(g, img, fr))
                });
            }
        }
    }
    drop(t);
    let Some(mut t) = sa::twin(251) else { return };
    let f = t.at(0x1480);
    let list = t.img.word(f + 0x28);
    t.set_u32(list, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x05, 0x25]);
    let me_obj = t.img.word(f + 0x21);
    t.set_u32(me_obj, 0x0bad_0b1e);
    let objs = t.img.word(f + 0x33);
    t.set_u32(objs + 4 * 5, 0x0b00_0005);
    t.set_u32(objs + 4 * 0x25, 0x0b00_0025);
    rec(&mut t, 235, 1, |_, _| 0x235);
    for (s, k) in [(277u32, 2.0f64), (15, 0.5), (16, -1.5), (17, 3.0)] {
        rec(&mut t, s, 1, move |_, a| s << 16 ^ a[0]);
        t.stub_st0(s, move |_, _| k);
    }
    let m = SCRATCH + 0x100;
    for i in [u32::MAX, 0, 1] {
        t.write(m, &[0x3c; 0x40]);
        let c = Call {
            eax: 0,
            ecx: i,
            edx: m,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::actor_scale(
                g, img, fr.at, fr.ecx, fr.edx,
            ))
        });
    }
}

#[test]
fn blur_setup_by_count() {
    let Some(mut t) = sa::twin(70) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0x1000);
    rec(&mut t, 245, 1, |_, _| 0x245);
    rec(&mut t, 821, 1, |_, _| 0x821);
    rec(&mut t, 822, 4, |_, a| 0x8220_0000 ^ a[3]);
    rec(&mut t, 823, 2, |_, a| 0x8230_0000 ^ a[0]);
    rec(&mut t, 824, 4, |_, _| 0x824);
    for (k, &(c, p, cp)) in sa::RINGS.iter().enumerate() {
        for (count, time) in [
            (0u8, 1.0f32),
            (1, 1.0),
            (2, 0.0),
            (3, -1.0),
            (4, 2.5),
            (0xff, 0.125),
        ] {
            sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
            t.set_u8(sa::node(0) + 0x1c, count);
            t.set_u32(sa::node(0) + 0x20, time.to_bits());
            sa::check_this_thunk(
                &mut t,
                f,
                0x1150,
                ffx_magic::sa_misc::blur_setup,
                Eax::Checked,
            );
        }
    }
}

#[test]
fn small_entries_b() {
    let Some(mut t) = sa::twin(446) else { return };
    let f = t.at(0x49b0);
    let fx_g = t.img.word(f + 0x01);
    t.set_u32(fx_g, SCRATCH + 0x1_0000);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(155) else { return };
    rec(&mut t, 262, 4, |m, a| {
        m.w32(a[1], 0x0bad);
        0x262
    });
    rec(&mut t, 12, 1, |_, _| 0x12);
    t.stub_st0(12, |_, _| 1.75);
    let obj = SCRATCH + 0x100;
    let v = SCRATCH + 0x200;
    t.set_u32(obj + 0x54, 0x0054_0000);
    let c = Call {
        eax: 0,
        ecx: obj,
        edx: v,
        ..Call::default()
    };
    t.check(t.at(0x8e70), c, &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::model_position(
            g, img, fr.at, fr.ecx, fr.edx,
        ))
    });
    drop(t);
    let Some(mut t) = sa::twin(98) else { return };
    for (s, n) in [(384u32, 1usize), (25, 2), (804, 1)] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    t.check_entry(t.at(0x3260), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(393) else { return };
    for (s, n) in [
        (628u32, 1usize),
        (311, 2),
        (622, 2),
        (630, 2),
        (629, 1),
        (802, 1),
    ] {
        rec(&mut t, s, n, move |m, a| match s {
            629 => m.r32(SCRATCH + 0x900),
            _ => s << 16 ^ a[0],
        });
    }
    let f = t.at(0x15a0);
    let p = t.img.word(f + 0x23);
    t.set_u32(p, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x9c]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    let f = t.at(0x14f0);
    let (state, count) = (t.img.word(f + 0x04), t.img.word(f + 0x1e));
    let obj = SCRATCH + 0x400;
    for (s, c, answer) in [
        (0u32, 5u32, 0u32),
        (0, 1, 0),
        (0, 0, 0),
        (1, 0, 0),
        (1, 0, 0xffff_ffff),
        (2, 3, 0),
        (2, 9, 0),
        (2, 20, 0),
        (3, 0, 0),
        (7, 0, 0),
    ] {
        t.set_u32(state, s);
        t.set_u32(count, c);
        t.set_u32(SCRATCH + 0x900, answer);
        t.write(obj, &[0x11; 0x60]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn load_wait_steps() {
    let Some(mut t) = sa::twin(376) else { return };
    let f = t.at(0x2670);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (a, started, b, file) = (w(&t, 0x02), w(&t, 0x2a), w(&t, 0x55), w(&t, 0x3a));
    t.set_u32(file, 0x00f1_1e00);
    for (s, n) in [(225u32, 1usize), (226, 2), (227, 1)] {
        rec(&mut t, s, n, move |m, _| {
            m.r32(SCRATCH + 0x900 + 4 * (s - 225))
        });
    }
    for (sa_, st, sb) in [(0u32, 0u32, 0u32), (1, 0, 0), (1, 1, 0), (1, 1, 1)] {
        for answers in [
            [0u32, 0x55, 1],
            [1, 0, 0],
            [0, 0, 0],
            [0, 7, 0xffff_ffff],
            [0, 0, 3],
        ] {
            for (k, v) in (0u32..).zip(answers) {
                t.set_u32(SCRATCH + 0x900 + 4 * k, v);
            }
            t.set_u32(a, sa_);
            t.set_u32(started, st);
            t.set_u32(b, sb);
            t.check_entry(
                f,
                Call {
                    eax: 0x00ea_0000,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
            );
        }
    }
}

#[test]
fn actor_slot256_with_xmm() {
    let Some(mut t) = sa::twin(53) else { return };
    let f = t.at(0x1c30);
    let list = t.img.word(f + 0x0d);
    t.set_u32(list, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x05, 0x25]);
    let me_obj = t.img.word(f + 0x06);
    t.set_u32(me_obj, 0x0bad_0b1e);
    let objs = t.img.word(f + 0x18);
    t.set_u32(objs + 4 * 5, 0x0b00_0005);
    t.set_u32(objs + 4 * 0x25, 0x0b00_0025);
    rec(&mut t, 256, 5, |_, a| 0x2560_0000 ^ a[0]);
    for i in [u32::MAX, 0, 1] {
        let c = Call {
            eax: 0,
            ecx: i,
            edx: 0x00ed_0000,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(9.0),
            ffx_magic::Xmm::f32(1.5),
            ffx_magic::Xmm::f32(-2.5),
            ffx_magic::Xmm::f32(0.125),
        ]);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_script::actor_slot256(g, img, fr))
        });
    }
}

#[test]
fn records_init_100_takes() {
    let Some(mut t) = sa::twin(171) else { return };
    let f = t.at(0x3510);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (count, free, head, tail) = (w(&t, 0x20), w(&t, 0x3e), w(&t, 0xa6), w(&t, 0xb7));
    let block = SCRATCH + 0x1_0000;
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (n, empty) in [(1u32, true), (2, false)] {
        t.write(block, &vec![0x77; 0x100 * 0x50]);
        for i in 1..=n {
            let next = if i == n { 1 } else { i + 1 };
            let prev = if i == 1 { n } else { i - 1 };
            t.set_u32(node(i), node(prev));
            t.set_u32(node(i) + 4, node(next));
        }
        t.set_u32(free, node(1));
        t.set_u16(count, n as u16);
        t.set_u32(head, if empty { 0 } else { node(9) });
        t.set_u32(tail, node(9));
        t.check(f, Call::this(block), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::records_init_100(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn ring_reserve_cases() {
    let Some(mut t) = sa::twin(42) else { return };
    let f = t.at(0x19c0);
    let req = SCRATCH + 0x100;
    let ring = SCRATCH + 0x200;
    for (cap, head, used, start, n) in [
        (16u16, 0u16, 0u16, 0u16, 4u16),
        (16, 14, 5, 3, 4),
        (16, 12, 16, 7, 4),
        (16, 3, 10, 9, 20),
        (16, 8, 8, 4, 8),
        (16, 10, 15, 12, 6),
        (8, 0, 8, 0, 8),
    ] {
        t.write(ring, &[0x5a; 0x40]);
        t.write(req, &[0xa5; 0x14]);
        t.set_u32(req, ring);
        t.set_u16(req + 4, n);
        t.set_u16(ring + 0x1c, cap);
        t.set_u16(ring + 0x1e, 0x24);
        t.set_u32(ring + 0x20, 0x0800_0000);
        t.set_u16(ring + 0x30, head);
        t.set_u16(ring + 0x32, used);
        t.set_u16(ring + 0x34, start);
        t.check(f, Call::this(req), &[], Eax::Checked, |g, _, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::ring_reserve(g, fr.ecx))
        });
    }
}

#[test]
fn part_matrix_builds() {
    let Some(mut t) = sa::twin(544) else { return };
    let f = t.at(0x5280);
    let o = t.img.word(f + 0x32);
    t.set_u32(o, 0x00b0_0000);
    rec(&mut t, 562, 0, |_, _| 0x562);
    rec(&mut t, 219, 2, |_, a| 0x2190_0000 ^ a[0]);
    rec(&mut t, 235, 1, |m, a| {
        // the callee resets the matrix
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for (s, k) in [(277u32, 2.0f64), (15, 0.5), (16, -1.5), (17, 3.0)] {
        rec(&mut t, s, 1, move |_, a| s << 16 ^ a[0]);
        t.stub_st0(s, move |_, _| k);
    }
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            // log the matrix's diagonal as the callee sees it
            m.w32(SCRATCH + 0x900 + 4 * (s - 328), m.r32(a[0] + 0x14));
            s << 16 ^ a[2]
        });
    }
    rec(&mut t, 825, 2, |m, a| {
        m.w32(SCRATCH + 0x910, m.r32(a[1] + 0x30));
        0x8250_0000 ^ a[0]
    });
    rec(&mut t, 843, 2, |m, a| {
        m.w32(SCRATCH + 0x914, m.r32(a[1] + 0x3c));
        0x8430_0000 ^ a[0]
    });
    let v = SCRATCH + 0x100;
    let p = SCRATCH + 0x200;
    for (i, x) in [0.1f32, -0.2, 0.3].iter().enumerate() {
        t.set_u32(v + 4 * i as u32, x.to_bits());
    }
    for i in 0..4 {
        t.set_u32(p + 4 * i, 0x4100_0000 + i);
    }
    let c = Call {
        eax: 0,
        ecx: v,
        edx: p,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::part_matrix(g, img, fr))
    });
}

#[test]
fn magic_0644_small() {
    let Some(mut t) = sa::twin(644) else { return };
    let f = t.at(0xb7b0);
    let fx_g = t.img.word(f + 0x01);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(fx_g, fx);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    for (rva, off) in [(0x8d50u32, 0x83f8u32), (0xa940, 0xeab0)] {
        let f = t.at(rva);
        let obj = SCRATCH + 0x100;
        for (a, b) in [(3u32, 3u32), (3, 4)] {
            t.write(obj, &[0x11; 0x60]);
            t.set_u32(fx + off, a);
            t.set_u32(fx + off + 4, b);
            t.set_u32(fx + off + 8, 7);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
    // the release: the effect's object in a two-node pool-A list
    let f = t.at(0x9f70);
    let list = t.img.word(f + 0x10);
    let p = t.img.word(f + 0x1e);
    t.set_u32(p, SCRATCH + 0x800);
    rec(&mut t, 713, 1, |_, a| 0x7130_0000 ^ a[0]);
    let o = fx + 0x11d0;
    let n = |i: u32| SCRATCH + 0x2c00 + 12 * i;
    for present in [false, true] {
        t.set_u32(list, n(0));
        t.set_u32(list + 4, n(1));
        t.set_u32(list + 8, n(0));
        t.set_u32(list + 0xc, 2);
        t.set_u32(n(0), 0);
        t.set_u32(n(0) + 4, n(1));
        t.set_u32(n(0) + 8, 0x0123);
        t.set_u32(n(1), n(0));
        t.set_u32(n(1) + 4, 0);
        t.set_u32(n(1) + 8, if present { o } else { 0x0456 });
        t.set_u16(o + 0x124, 0xffff);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    // three reels: equal or not
    let f = t.at(0x5d20);
    let r = SCRATCH + 0x400;
    let strip = SCRATCH + 0x600;
    t.set_u32(strip, 4);
    t.set_u32(strip + 4, strip + 8);
    t.write(strip + 8, &[7, 7, 9, 7]);
    for pos in [[0i16, 0, 0], [0, 1, 0], [3, 3, 0], [2, 2, 2], [1, 0, 3]] {
        t.write(r, &[0x5a; 0x80]);
        for (k, p) in (0u32..).zip(pos) {
            t.set_u32(r + 0x1c * k, strip);
            t.set_u16(r + 0x1c * k + 6, p as u16);
        }
        t.check(f, Call::this(r), &[], Eax::Checked, |g, _, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::reels_match(g, fr.ecx))
        });
    }
}

#[test]
fn mem_open_and_fx_reset() {
    let Some(mut t) = sa::twin(384) else { return };
    rec(&mut t, 436, 9, |_, a| 0x4360_0000 ^ a[1]);
    rec(&mut t, 434, 2, |_, a| 0x4340_0000 ^ a[1]);
    t.check_entry(t.at(0x23b0), Call::default(), &[0x00a1_0000], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(565) else { return };
    let f = t.at(0x5b00);
    let fx_g = t.img.word(f + 0x03);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(fx_g, fx);
    t.set_u32(fx + 0xec94, SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x304, 0x0042_4242);
    rec(&mut t, 219, 2, |_, a| 0x2190_0000 ^ a[0]);
    rec(&mut t, 257, 2, |_, a| 0x2570_0000 ^ a[0]);
    for flag in [0u32, 1, 2] {
        t.write(fx + 0xd740, &[0x77; 0x14]);
        t.set_u32(fx + 0xd748, flag);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::fx_reset_d74c(g, img, fr.at))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(173) else { return };
    let f = t.at(0x6a10);
    let p = t.img.word(f + 0x0f);
    let node = SCRATCH + 0x100;
    for kids in [0u32, 1] {
        t.write(p, &[0x44; 0x20]);
        t.set_u32(p + 0x10, if kids == 0 { p } else { SCRATCH + 0x400 });
        t.write(node, &[0x55; 0x40]);
        t.check(f, Call::this(node), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::static_child(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn camera_target_b_folds() {
    let Some(mut t) = sa::twin(644) else { return };
    let f = t.at(0xab30);
    let fx_g = t.img.word(f + 0x08);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(fx_g, fx);
    let m = t.img.word(f + 0x22);
    t.set_u32(m, 0x00ab_0000);
    rec(&mut t, 262, 4, |m, a| {
        m.w32(a[1], 0x3f00_0000);
        0x262
    });
    rec(&mut t, 333, 1, |_, _| 0x333);
    for angle in [0.0f64, 1.0, -3.0, 7.0, 100.0, -100.0] {
        t.stub_st0(333, move |_, _| angle);
        t.write(fx + 0x2e50, &[0x5a; 0x10]);
        t.write(fx + 0xeac0, &[0x5a; 0xc0]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::camera_target_b(g, img, fr.at))
        });
    }
}

#[test]
fn register_grid_loops() {
    let Some(mut t) = sa::twin(250) else { return };
    let f = t.at(0x2300);
    let (n, table, lock) = (
        t.img.word(f + 0x05),
        t.img.word(f + 0x0c),
        t.img.word(f + 0x53),
    );
    let recs = SCRATCH + 0x1000;
    t.set_u32(table, recs);
    for k in 0..4 {
        t.set_u32(recs + 0x68 * k, 0x0300_0000 + k);
    }
    t.set_u32(lock, SCRATCH + 0x800);
    rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(&mut t, 779, 4, |m, a| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        a[0]
    });
    for count in [0u32, 1, 3, 0xffff_ffff] {
        t.set_u32(n, count);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::register_grid(g, img, fr.at))
        });
    }
}

#[test]
fn place_me_ways() {
    let Some(mut t) = sa::twin(303) else { return };
    let f = t.at(0x2cb0);
    let (battle, me, obj) = (
        t.img.word(f + 0x02),
        t.img.word(f + 0x0d),
        t.img.word(f + 0x32),
    );
    t.set_u32(obj, 0x0bad_0b1e);
    rec(&mut t, 255, 2, |_, a| 0x2550_0000 ^ a[0]);
    rec(&mut t, 11, 1, |m, _| m.r32(SCRATCH + 0x900));
    rec(&mut t, 32, 3, |_, a| 0x0320_0000 ^ a[1]);
    rec(&mut t, 33, 4, |_, a| 0x0330_0000 ^ a[2]);
    let v = SCRATCH + 0x100;
    for (i, x) in [1.5f32, -2.0, 3.25].iter().enumerate() {
        t.set_u32(v + 4 * i as u32, x.to_bits());
    }
    for (b, m, grounded) in [
        (0u32, 5u32, 0u32),
        (0, 0x20, 0),
        (0, 0x25, 1),
        (1, 5, 0),
        (1, 5, 1),
    ] {
        t.set_u32(battle, b);
        t.set_u32(me, m);
        t.set_u32(SCRATCH + 0x900, grounded);
        let c = Call {
            eax: 0,
            ecx: 0x77,
            edx: v,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::place_me(g, img, fr.at, fr.edx))
        });
    }
}

#[test]
fn magic_0644_tables() {
    let Some(mut t) = sa::twin(644) else { return };
    let f = t.at(0xb2b0);
    let (step, end, obj) = (
        t.img.word(f + 0x04),
        t.img.word(f + 0x6e),
        t.img.word(f + 0x2c),
    );
    t.set_u32(obj, 0x00b0_0000);
    rec(&mut t, 843, 2, |_, a| 0x8430_0000 ^ a[1]);
    rec(&mut t, 219, 2, |_, a| 0x2190_0000 ^ a[0]);
    rec(&mut t, 833, 3, |_, a| 0x8330_0000 ^ a[2]);
    rec(&mut t, 847, 2, |_, _| 0x847);
    for (s, e) in [
        (0u32, 10u32),
        (8, 10),
        (9, 10),
        (0x13, 0x30),
        (0x14, 0x30),
        (3, 0),
    ] {
        t.set_u32(step, s);
        t.set_u32(end, e);
        t.check_entry(f, Call::default(), &[0x00a1_0000], Eax::Checked);
    }
    let f = t.at(0xa9d0);
    let fx_g = t.img.word(f + 0x07);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(fx_g, fx);
    let n = t.img.word(f + 0x18);
    t.set_u32(n, 3);
    let list = t.img.word(f + 0x8b);
    t.set_u32(list, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x05, 0x25, 0x07]);
    // slot 645: per kind a byte list, its length and a word from cells
    rec(&mut t, 645, 4, |m, a| {
        let k = a[2];
        m.w32(a[1], m.r32(SCRATCH + 0x900 + 8 * k));
        m.w32(a[3], m.r32(SCRATCH + 0x904 + 8 * k));
        SCRATCH + 0x400
    });
    t.write(SCRATCH + 0x400, &[0x07, 0x05, 0x99, 0x25]);
    for (k, (len, word)) in [(4u32, 0x11u32), (0, 0), (2, 0x33), (3, 0)]
        .into_iter()
        .enumerate()
    {
        t.set_u32(SCRATCH + 0x900 + 8 * k as u32, len);
        t.set_u32(SCRATCH + 0x904 + 8 * k as u32, word);
    }
    t.write(fx + 0xeac4, &[0x66; 0xb0]);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::kind_records(g, img, fr.at, fr.esp))
    });
}

#[test]
fn jitter_adds() {
    let Some(mut t) = sa::twin(515) else { return };
    let f = t.at(0x20c0);
    let o = SCRATCH + 0x100;
    rec(&mut t, 314, 1, |_, a| 0x3140_0000 ^ a[0]);
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x2_0000 + 0x80 * 314);
        f64::from(f32::from_bits(a[0])) * (f64::from(k) * 0.37 - 1.0)
    });
    for (n1, n2) in [(0i32, 0i32), (1, 0), (0, 2), (2, 1), (-1, 1)] {
        t.write(o, &[0x3f; 0x180]);
        t.set_u32(o + 0x16c, 0.5f32.to_bits());
        t.set_u32(o + 0x170, (-2.0f32).to_bits());
        t.set_u32(o + 0x174, n1 as u32);
        t.set_u32(o + 0x178, n2 as u32);
        t.check(f, Call::this(o), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::jitter(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn host_start_d_and_rotate_add() {
    let Some(mut t) = sa::twin(165) else { return };
    for (s, n) in [
        (353u32, 1usize),
        (397, 1),
        (332, 1),
        (267, 1),
        (933, 4),
        (785, 1),
        (543, 1),
        (250, 2),
        (550, 1),
    ] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    // slot 549 answers the cells in turn
    rec(&mut t, 549, 1, |m, _| {
        let k = m.r32(SCRATCH + 0x2_0000 + 0x80 * 549);
        m.r32(SCRATCH + 0x900 + 4 * (k - 1))
    });
    let f = t.at(0x1260);
    for answers in [[0u32, 0], [1, 0], [1, 1], [0, 1]] {
        t.set_u32(SCRATCH + 0x2_0000 + 0x80 * 549, 0);
        t.set_u32(SCRATCH + 0x900, answers[0]);
        t.set_u32(SCRATCH + 0x904, answers[1]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = sa::twin(644) else { return };
    let f = t.at(0x35a0);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m, a| {
            m.w32(a[0] + 4, a[2]);
            s << 16 ^ a[2]
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 248, 3, |m, a| {
        m.w32(SCRATCH + 0x910, m.r32(a[2]));
        0x2480_0000 ^ a[0]
    });
    let rot = SCRATCH + 0x100;
    let out = SCRATCH + 0x200;
    let v = SCRATCH + 0x300;
    t.set_u32(rot, 0.5f32.to_bits());
    t.set_u32(rot + 4, (-1.25f32).to_bits());
    t.write(out, &[0x44; 0x10]);
    t.write(v, &[0x33; 0x10]);
    let c = Call {
        eax: 0,
        ecx: rot,
        edx: out,
        ..Call::default()
    };
    t.check(f, c, &[v], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sb_misc::rotate_offset(
            g, img, fr.at, fr.esp, fr.ecx, fr.edx,
        ))
    });
}

#[test]
fn magic_0515_small() {
    let Some(mut t) = sa::twin(515) else { return };
    rec(&mut t, 331, 1, |_, a| 0x3310_0000 ^ a[0]);
    rec(&mut t, 306, 3, |_, a| 0x3060_0000 ^ a[1]);
    rec(&mut t, 296, 2, |_, a| 0x2960_0000 ^ a[0]);
    t.check(
        t.at(0x9ed0),
        Call::default(),
        &[],
        Eax::Checked,
        |g, img, fr| ffx_magic::Ret::eax(ffx_magic::sa_misc::block_331(g, img, fr.at)),
    );
    let f = t.at(0x9d90);
    let x = t.img.word(f + 0x02);
    t.set_u32(x, 0x0042_0000);
    let c = Call {
        eax: 0,
        ecx: 0,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::slot306(g, img, fr.at, fr.edx))
    });
    let p = SCRATCH + 0x100;
    for id in [0xffu32, 3, 0x1ff] {
        t.set_u32(p, id);
        let c = Call {
            eax: 0,
            ecx: p,
            edx: 0x00ed_0000,
            ..Call::default()
        };
        t.check(t.at(0x2640), c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::record_296(
                g, img, fr.at, fr.ecx, fr.edx,
            ))
        });
    }
    let f = t.at(0x3d50);
    let recs = t.img.word(f + 0x06);
    t.write(recs, &vec![0x77; 0x100 * 0x4c]);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::records_clear(g, img, fr.at))
    });
    let f = t.at(0x2da0);
    let (a, b) = (t.img.word(f + 0x01), t.img.word(f + 0x14));
    let n = (b - a) / 0xa8;
    for free in [Some(0u32), Some(3), Some(n - 1), None] {
        for i in 0..n {
            t.set_u32(a + 0xa8 * i + 0x80, u32::from(Some(i) != free));
        }
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::record_free(g, img, fr.at))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(98) else { return };
    rec(&mut t, 239, 3, |_, a| 0x2390_0000 ^ a[0]);
    let f = t.at(0x20f0);
    t.set_u32(t.img.word(f + 0x15), 0x00a0_0000);
    t.set_u32(t.img.word(f + 0x09), 0x00b0_0000);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(351) else { return };
    rec(&mut t, 399, 2, |_, a| 0x3990_0000 ^ a[0]);
    let f = t.at(0x15c0);
    t.set_u32(t.img.word(f + 0x09), 0x00c0_0000);
    t.set_u32(t.img.word(f + 0x1a), 0x5555);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn small_internal_c() {
    let Some(mut t) = sa::twin(515) else { return };
    let o = SCRATCH + 0x100;
    t.write(o, &[0x5a; 0x190]);
    let c = Call {
        eax: 0x00ea_0000,
        ecx: o,
        edx: 0,
        ..Call::default()
    }
    .with_xmm(&[
        ffx_magic::Xmm::f32(9.0),
        ffx_magic::Xmm::f32(1.5),
        ffx_magic::Xmm::f32(-2.5),
    ]);
    t.check(t.at(0x1b40), c, &[], Eax::Checked, |g, _, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::spread_set(g, fr))
    });
    let f = t.at(0x2490);
    let (n, id, x) = (
        t.img.word(f + 0x07),
        t.img.word(f + 0x13),
        t.img.word(f + 0x24),
    );
    t.set_u32(x, 0x0042_0000);
    rec(&mut t, 309, 1, |_, a| 0x3090_0000 ^ a[0]);
    for (count, sid) in [(0u32, 3u32), (2, 3), (3, 0xff)] {
        t.set_u32(n, count);
        t.set_u32(id, sid);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::repeat_309(g, img, fr.at, fr.eax))
        });
    }
    let f = t.at(0x3940);
    let anim = SCRATCH + 0x300;
    let recd = SCRATCH + 0x400;
    for (sub, len, fno, frames) in [
        (0u16, 0x400u16, 0u16, 3i16),
        (0x300, 0x400, 0, 3),
        (0x300, 0x400, 2, 3),
        (0x7f00, 0x8100, 5, 2),
        (0xfe00, 0x100, 0x7fff, -1),
        (0x7f00, 0x100, 0, 3),
    ] {
        t.write(o, &[0x5a; 0x60]);
        t.set_u16(o + 0x48, sub);
        t.set_u16(o + 0x4a, fno);
        t.set_u32(o + 0x28, recd);
        t.set_u16(anim + 2, len);
        t.set_u16(recd + 6, frames as u16);
        t.check_entry(f, Call::default(), &[o, anim], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = sa::twin(356) else { return };
    rec(&mut t, 450, 2, |_, a| 0x4500_0000 ^ a[1]);
    rec(&mut t, 449, 2, |_, a| 0x4490_0000 ^ a[1]);
    let flag = t.img.word(t.at(0x1000) + 0x18);
    t.set_u32(flag, 0x5555);
    t.check_entry(t.at(0x1000), Call::default(), &[0x00a1_0000], Eax::Checked);
    drop(t);
    let Some(mut t) = sa::twin(155) else { return };
    let f = t.at(0x8cc0);
    let lock = t.img.word(f + 0x18);
    t.set_u32(lock, SCRATCH + 0x800);
    t.set_u32(SCRATCH + 0x800, 0x5555);
    rec(&mut t, 771, 1, |_, a| 0x7710_0000 ^ a[0]);
    t.stub_edx(771, |_, a| 0x0edd_0000 ^ a[0]);
    rec(&mut t, 779, 2, |m, a| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        a[0]
    });
    t.check(f, Call::this(0x0042), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::register_one(g, img, fr.at, fr.ecx))
    });
    drop(t);
    let Some(mut t) = sa::twin(262) else { return };
    rec(&mut t, 211, 2, |_, a| 0x2110_0000 ^ a[0]);
    rec(&mut t, 255, 2, |_, a| 0x2550_0000 ^ a[0]);
    for flags in [0u16, 1, 0x10, 0xffff, 0xfffe] {
        t.write(o, &[0x5a; 0x100]);
        t.set_u16(o + 0x90, flags);
        t.set_u32(o + 0x4c, SCRATCH + 0x300);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: o,
            edx: 0,
            ..Call::default()
        };
        t.check(t.at(0x6aa0), c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::object_to_node(
                g, img, fr.at, fr.ecx, fr.eax,
            ))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(70) else { return };
    rec(&mut t, 234, 3, |_, a| 0x2340_0000 ^ a[0]);
    t.write(o, &[0x5a; 0x60]);
    let c = Call::this(o).with_xmm(&[ffx_magic::Xmm::f32(9.0), ffx_magic::Xmm::f32(0.75)]);
    t.check(t.at(0x99a0), c, &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::node_start(g, img, fr))
    });
}

/// The handle stubs: slot 772 answers `edx:eax` from its words, slot 779
/// logs the lock as it sees it.
fn handle_stubs(t: &mut Twin, lock: u32) {
    t.set_u32(lock, SCRATCH + 0x800);
    t.set_u32(SCRATCH + 0x800, 0x5555);
    rec(t, 772, 2, |_, a| 0x7720_0000 ^ a[0] ^ a[1] << 4);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 779, 4, |m, a| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        a[0]
    });
}

#[test]
fn handle_registers() {
    let Some(mut t) = sa::twin(284) else { return };
    let f = t.at(0x2a90);
    let lock = t.img.word(f + 0x21);
    handle_stubs(&mut t, lock);
    t.check(f, Call::this(0x0042), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::register_8(g, img, fr.at, fr.ecx))
    });
    drop(t);
    let Some(mut t) = sa::twin(515) else { return };
    let f = t.at(0x3e90);
    let lock = t.img.word(f + 0x22);
    handle_stubs(&mut t, lock);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::sa_misc::register_records(g, img, fr.at))
    });
    let f = t.at(0x34b0);
    let (first, end) = (t.img.word(f + 0x04), t.img.word(f + 0x5d));
    let n = (end - first) / 0xa8;
    for counts in [[-1i32, -1, -1], [0, 2, -1], [-1, -1, 1]] {
        for i in 0..n {
            let c = if i < 3 { counts[i as usize] } else { -1 };
            t.set_u32(first + 0xa8 * i, c.cast_unsigned());
        }
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::register_counted(g, img, fr.at, fr.eax))
        });
    }
}

#[test]
fn magic_0515_records() {
    let Some(mut t) = sa::twin(515) else { return };
    rec(&mut t, 255, 2, |_, a| 0x2550_0000 ^ a[0]);
    rec(&mut t, 170, 2, |_, a| 0x1700_0000 ^ a[1]);
    let recs = SCRATCH + 0x1000;
    for i in 0..4u32 {
        t.write(recs + 0xcc * i, &[0x11; 0xcc]);
        t.set_u32(recs + 0xcc * i, 0x0b00_0000 + i);
        t.set_u16(recs + 0xcc * i + 0x14, (i as u16).wrapping_mul(0x5234));
    }
    for n in [0u32, 1, 4] {
        let c = Call {
            eax: 0x00ea_0000,
            ecx: recs,
            edx: n,
            ..Call::default()
        };
        t.check(t.at(0x28f0), c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::place_records(
                g, img, fr.at, fr.ecx, fr.edx, fr.eax,
            ))
        });
    }
    rec(&mut t, 295, 2, |_, a| 0x2950_0000 ^ a[0]);
    let f = t.at(0x2560);
    let (n, a, b) = (
        t.img.word(f + 0x05),
        t.img.word(f + 0x0d),
        t.img.word(f + 0x36),
    );
    for i in 0..4 {
        t.set_u32(a + 0xcc * i, 0x0a00_0000 + i);
    }
    let _ = b;
    for count in [0u32, 2, 0xffff_ffff] {
        t.set_u32(n, count);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::records_295(g, img, fr.at))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(284) else { return };
    let f = t.at(0x3510);
    let (rover, count, base) = (
        t.img.word(f + 0x02),
        t.img.word(f + 0x0a),
        t.img.word(f + 0x17),
    );
    let ring = SCRATCH + 0x1000;
    t.set_u32(base, ring);
    for (n, start, used) in [
        (0i32, 0u32, 0u32),
        (4, 0, 0),
        (4, 2, 0b0100),
        (4, 3, 0b1000),
        (4, 1, 0b1111),
    ] {
        t.write(ring, &vec![0x55; 0x68 * 5]);
        for i in 0..5 {
            t.set_u32(ring + 0x68 * i + 0x60, (used >> i) & 1);
        }
        t.set_u32(count, n as u32);
        t.set_u32(rover, start);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::ring_take_68(g, img, fr.at))
        });
    }
}

#[test]
fn turn_matrix_and_static_node() {
    let Some(mut t) = sa::twin(515) else { return };
    let f = t.at(0x3ef0);
    rec(&mut t, 247, 2, |m, a| {
        m.w32(a[0], a[1]);
        0x247
    });
    rec(&mut t, 327, 2, |m, a| {
        m.w32(SCRATCH + 0x900, m.r32(a[1] + 0x30));
        0x3270_0000 ^ a[0]
    });
    for (i, op) in [0x25u32, 0x3a, 0x4a, 0x5a].into_iter().enumerate() {
        let src = t.img.word(f + op);
        t.set_u32(src, 0x4100_0000 + i as u32);
    }
    for angle in [0u32, 0x1234, 0xffff_c000] {
        let c = Call {
            eax: 0,
            ecx: 0,
            edx: angle,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::turn_matrix(g, img, fr.at, fr.edx))
        });
    }
    drop(t);
    let Some(mut t) = sa::twin(42) else { return };
    let f = t.at(0x8470);
    let p = t.img.word(f + 0x46);
    for kids in [0u32, 1] {
        t.write(p, &[0x44; 0x20]);
        t.set_u32(p + 0x10, if kids == 0 { p } else { SCRATCH + 0x400 });
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_misc::static_node(g, img, fr.at))
        });
    }
}
