//! The shared-5 list's ports (`ffx_magic::s5_misc`) against the DLLs
//! they were ported from. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::{Ret, Xmm, s5_misc};
use magic::{Call, Eax, SCRATCH, STACK_LEN, STACK_TOP, StubMem, TABLE, TRAPS, Twin};
use sa::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// Each slot recorded with `n` words, answering a number of its own.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// The word at `f + off` of the code.
fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

/// The trap of table word `s` (a callback the harness answers as slot `s`).
fn trap(s: u32) -> u32 {
    TRAPS + 16 * s
}

/// `n` bytes that differ from their neighbours (`seed` picks the pattern).
fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// Whether `a` is in the stack or the scratch area (a pointer a stub may
/// follow).
fn followable(a: u32) -> bool {
    (STACK_TOP - STACK_LEN..STACK_TOP - 0x40).contains(&a)
        || (SCRATCH..SCRATCH + 0x1_f000).contains(&a)
}

/// Records each call of `slot` (a row of its own per call, 8 rows): its
/// first `n` words and, for each that points into the stack or the
/// scratch area, the `words` words there. With `fill = Some(i)`, the
/// memory argument `i` points at gets `words` new words (a callee's
/// result: floats near 1.0 told apart by slot, call and word). Answers
/// the call's number. Uses `SCRATCH + 0x2_0000..`: don't mix with
/// [`rec`] in one test.
fn rec_mem(t: &mut Twin, slot: u32, n: usize, words: u32, fill: Option<usize>) {
    let base = SCRATCH + 0x2_0000 + 0x800 * (slot % 0x40);
    t.stub(slot, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(base);
        m.w32(base, k + 1);
        let row = base + 4 + 0x100 * (k % 8);
        let mut at = row;
        for &x in &a[..n] {
            m.w32(at, x);
            at += 4;
            if followable(x) {
                for i in 0..words {
                    let v = m.r32(x + 4 * i);
                    m.w32(at, v);
                    at += 4;
                }
            }
        }
        if let Some(j) = fill.filter(|&j| followable(a[j])) {
            for i in 0..words {
                m.w32(
                    a[j] + 4 * i,
                    0x3f80_0000 + (slot << 12) + (k << 8) + i * 0x11,
                );
            }
        }
        (slot << 16) + k
    });
}

/// Answers of slot 261 (`AtelGetEffectFlag`): the word at `FLAGS + 4 * flag`.
const FLAGS: u32 = SCRATCH + 0x3000;

#[test]
fn wot_frames() {
    let Some(mut t) = twin(662) else { return };
    let f = t.at(0x16c0);
    recs(
        &mut t,
        &[(723, 1), (724, 2), (377, 4), (381, 2), (376, 1), (214, 3)],
    );
    rec(&mut t, 302, 3, |_, _| 0x1234_5678);
    rec(&mut t, 261, 1, |m, a| m.r32(FLAGS + 4 * (a[0] & 0xff)));
    rec(&mut t, 1001, 1, |_, a| a[0]);
    t.set_u32(TABLE + 4 * 1001, trap(1001));
    // the timeline [`timeline_stop_global`] stops
    let stop = t.img.rel32_target(f + 0xfc);
    let ev = SCRATCH + 0x600;
    let (live_n, count, evp, live) = (
        op(&t, stop, 0x02),
        op(&t, stop, 0x0d),
        op(&t, stop, 0x18),
        op(&t, stop, 0x1d),
    );
    for (on, f0, once, f1) in [
        (0u32, 1u32, 0u32, 1u32),
        (1, 0, 0, 0),
        (1, 1, 0, 0),
        (1, 1, 3, 1),
        (1, 1, 0, 1),
        (1, 0, 0, 1),
    ] {
        t.write(SCRATCH + 0x100, &varied(0x100, on + f0 + once));
        for o in [0x21u32, 0x4c, 0x5f, 0xe0, 0x107, 0x29, 0x9c, 0x11e] {
            t.set_u32(op(&t, f, o), 0x0100_0000 + o * 0x11);
        }
        t.set_u32(op(&t, f, 0x2e), SCRATCH + 0x110);
        t.set_u32(op(&t, f, 0x5a), SCRATCH + 0x124);
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(op(&t, f, 0x7d), once);
        t.set_u32(FLAGS, f0);
        t.set_u32(FLAGS + 4, f1);
        t.set_u32(live_n, 2);
        t.set_u32(count, 3);
        t.set_u32(evp, ev);
        for i in 0..3u32 {
            t.set_u32(live + 8 * i, u32::from(i != 1));
            t.set_u32(ev + 0x18 * i + 0x10, if i == 2 { 0 } else { trap(1001) });
            t.set_u32(ev + 0x18 * i + 0x14, 0xb0 + i);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn facing_scales() {
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x7e90);
    rec_mem(&mut t, 723, 1, 16, Some(0));
    rec_mem(&mut t, 724, 2, 16, Some(0));
    rec_mem(&mut t, 327, 2, 16, Some(0));
    rec_mem(&mut t, 229, 3, 4, Some(0));
    rec_mem(&mut t, 235, 1, 16, Some(0));
    rec_mem(&mut t, 330, 3, 16, Some(0));
    rec_mem(&mut t, 231, 3, 16, Some(0));
    rec_mem(&mut t, 282, 0, 0, None);
    let obj = SCRATCH + 0x400;
    for depth in [2.5f64, 0.1, -3.0, 0.0] {
        t.stub_st0(282, move |_, _| depth);
        t.write(obj, &varied(0x100, depth.to_bits() as u32));
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn motions_towards() {
    let Some(mut t) = twin(201) else { return };
    let f = t.at(0x25b0);
    let this = SCRATCH + 0x300;
    let obj = SCRATCH + 0x400;
    rec_mem(&mut t, 217, 2, 4, Some(1));
    rec_mem(&mut t, 234, 3, 4, Some(0));
    rec_mem(&mut t, 233, 3, 4, Some(0));
    rec_mem(&mut t, 248, 3, 4, Some(0));
    for (flags, m, len) in [
        (0x0009u16, 7u32, 0.5f32),
        (0xfffe, 0x8000_0005, 2.0),
        (0x0008, 0x0012_3456, -1.0),
        (0x0f0f, 3, 1.0e6),
    ] {
        t.stub(228, move |_, a| m ^ (a[0] & 0xf) ^ (a[1] & 0xf0));
        t.write(obj, &varied(0x100, m));
        t.set_u16(obj + 0x90, flags);
        t.set_u32(this, obj);
        t.set_u32(this + 4, 0x0228_0228);
        t.set_u32(this + 8, SCRATCH + 0x500);
        t.set_u32(this + 0xc, len.to_bits());
        let eax = if flags & 1 == 0 {
            Eax::Checked
        } else {
            // the cookie word: the original's DllMain set the cookie
            Eax::Junk
        };
        t.check_entry(f, Call::this(this), &[], eax);
    }
}

#[test]
fn chara_setups() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x26d0);
    recs(
        &mut t,
        &[(26, 2), (387, 2), (385, 2), (386, 2), (922, 2), (268, 3)],
    );
    let ch = SCRATCH + 0x1000;
    rec(&mut t, 0, 3, move |_, _| ch);
    for have in [0u32, SCRATCH + 0x1800] {
        t.write(SCRATCH + 0x1000, &varied(0x1000, have));
        for o in [
            0x54u32, 0x69, 0xb5, 0x9e, 0xa3, 0xbb, 0xc5, 0xcf, 0xd9, 0xe3, 0xed, 0xf7, 0x101,
            0x10d, 0x113,
        ] {
            t.set_u32(op(&t, f, o), 0x0100_0000 + o * 0x11);
        }
        t.set_u32(op(&t, f, 0x7), 5);
        t.set_u32(op(&t, f, 0x1), have);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn targets_starts() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x1330);
    recs(&mut t, &[(268, 3), (714, 5), (325, 2)]);
    rec(&mut t, 367, 3, |m, a| {
        m.w32(a[2], a[1].wrapping_mul(0x0101_0103));
        0x367
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    let table = SCRATCH + 0x700;
    rec(&mut t, 941, 1, move |_, _| table);
    let start = t.img.rel32_target(f + 0xc2);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f0);
    let ids = SCRATCH + 0x7c0;
    let recs_at = SCRATCH + 0x1000;
    for n in [0u32, 1, 3, 0xffff_fff0] {
        t.write(recs_at, &varied(0x300, n));
        t.write(SCRATCH + 0x700, &varied(0x100, n + 1));
        for o in [0x4au32, 0x93, 0x9e] {
            t.set_u32(op(&t, f, o), SCRATCH + 0x780 + o / 4 * 4 % 0x40);
        }
        t.set_u32(op(&t, f, 0x31), ids);
        t.set_u32(op(&t, f, 0x5), n);
        let c = Call {
            ecx: 0x0ecc_0ecc,
            ..Call::default()
        };
        t.check_entry(f, c, &[recs_at], Eax::Checked);
    }
}

#[test]
fn particles_rethrown() {
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x7470);
    recs(&mut t, &[(531, 2)]);
    let calls = SCRATCH + 0x7f0;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(calls);
        m.w32(calls, k + 1);
        [0x1234_5ffe, 0x0000_0002, 0x7fff_fffd, 0x8000_1001][(k % 4) as usize]
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| f64::from(m.r32(SCRATCH + 0x7f0)) * 0.37 - 0.2);
    let this = SCRATCH + 0x300;
    let obj = SCRATCH + 0x4000;
    for n in [0u32, 1, 4, 0x8000_0000] {
        t.write(obj, &varied(0x1700, n));
        for k in 0..4u32 {
            let p = obj + 0x13c + 0x278 * k;
            t.set_u32(p + 0x1a0, (1.5f32 + k as f32).to_bits());
            t.set_u8(p, (k as u8) ^ (n as u8));
        }
        t.set_u32(obj + 0x163c, n);
        t.set_u32(this, obj);
        t.set_u32(this + 4, 0x0aaa_0004);
        t.set_u32(this + 8, 0x0bbb_0008);
        t.check_entry(f, Call::this(this), &[], Eax::Checked);
    }
}

#[test]
fn camera_rect_tasks() {
    let Some(mut t) = twin(152) else { return };
    let f = t.at(0x8440);
    recs(&mut t, &[(284, 3), (693, 2), (692, 3), (686, 1)]);
    let task = SCRATCH + 0x300;
    let rec_at = SCRATCH + 0x400;
    let node = SCRATCH + 0x600;
    for (state, count) in [
        (0u32, 5u32),
        (1, 3),
        (1, 1),
        (1, 0),
        (0, 0),
        (2, 0),
        (0x2710, 0),
        (0x2711, 2),
        (0x2711, 0),
        (0x2711, 0xffff_ffff),
        (0x2712, 0),
        (0xffff_fffe, 0),
    ] {
        t.write(rec_at, &varied(0x200, state ^ count));
        t.write(node, &varied(0x40, state));
        t.set_u32(task, node);
        t.set_u32(task + 4, rec_at);
        t.set_u32(node + 4, SCRATCH + 0x700);
        t.set_u32(node + 8, SCRATCH + 0x740);
        t.set_u32(rec_at + 0x24, SCRATCH + 0x780);
        t.set_u32(rec_at + 0x28, SCRATCH + 0x7c0);
        t.set_u32(rec_at + 0x9c, 2.5f32.to_bits());
        t.set_u32(rec_at + 0x18, count);
        t.set_u32(rec_at + 0x1c, state);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The object block's slots (`l5_objhost::obj_block_init`): `n` entries.
fn block_stubs(t: &mut Twin, n: u32) {
    const BLOCK: u32 = SCRATCH + 0x1000;
    const BYTES: u32 = SCRATCH + 0x1800;
    t.set_u32(BLOCK + 4, n);
    for i in 0..8 {
        t.set_u8(BYTES + i, (3 * i + 1) as u8);
    }
    rec(t, 168, 1, |_, _| BLOCK);
    rec(t, 218, 2, |_, _| BYTES);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    rec(t, 291, 5, |_, a| a[1] + 0x10);
    rec(t, 367, 8, |_, a| a[2] ^ 0x367);
}

#[test]
fn host_starts_chara() {
    let Some(mut t) = twin(349) else { return };
    let f = t.at(0x4360);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (4, 3),
            (27, 5),
            (718, 6),
            (222, 10),
            (221, 12),
        ],
    );
    rec(&mut t, 790, 0, |_, _| 0x790);
    let payloads = op(&t, f, 0x81);
    let done = op(&t, f, 0x133);
    for (n, scale) in [(0u32, None), (3, Some(1.25f64)), (8, Some(-0.1))] {
        block_stubs(&mut t, n);
        match scale {
            Some(s) => t.stub_st0(790, move |_, _| s),
            None => t.stub_st0(790, |_, _| f64::NAN),
        }
        t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
        t.set_u32(done, 0x77);
        for o in [0x57, 0x61, 0x6b, 0xa3, 0xab] {
            t.set_u32(op(&t, f, o), 0x0101_0101);
        }
        t.check_entry(f, Call::default(), &[0x00ab_c000 + n], Eax::Checked);
    }
}

#[test]
fn node_setups() {
    let Some(mut t) = twin(573) else { return };
    let f = t.at(0x3660);
    recs(&mut t, &[(235, 1), (230, 2), (325, 2)]);
    let args = SCRATCH + 0x300;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1400;
    let last = SCRATCH + 0x1800;
    for (p, l) in [(0u32, 0u32), (parent, parent), (parent, last)] {
        t.write(node, &varied(0x200, p));
        t.write(parent, &varied(0x40, l + 1));
        t.write(last, &varied(0x40, l + 2));
        t.write(args, &varied(0x18, l + 3));
        t.set_u32(args, node);
        t.set_u32(args + 4, p);
        t.set_u32(parent + 0x10, l);
        t.check_entry(f, Call::this(args), &[], Eax::Checked);
    }
}

#[test]
fn rising_parts() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x1630);
    recs(
        &mut t,
        &[(325, 2), (258, 1), (231, 3), (230, 2), (713, 1), (714, 5)],
    );
    let answer = SCRATCH + 0x7e0;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let table = SCRATCH + 0x700;
    rec(&mut t, 941, 2, move |_, _| table);
    let start = t.img.rel32_target(f + 0x111);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f0);
    let obj = SCRATCH + 0x1000;
    for (ended, fade, life) in [
        (0u32, 0u32, 0x100u16),
        (1, 0, 0x100),
        (0, 1, 0x100),
        (0, 1, 0x89),
        (5, 2, 0x88),
        (0, 1, 0x8000),
    ] {
        t.write(obj, &varied(0x100, ended + fade));
        t.write(table, &varied(0x40, life.into()));
        t.set_u16(obj + 0x76, life);
        t.set_u32(answer, ended);
        for o in [0x5cu32, 0x67, 0x73, 0x86] {
            t.set_u32(op(&t, f, o), 0x0100_0000 + o);
        }
        t.set_u32(op(&t, f, 0x93), SCRATCH + 0x7c0);
        t.set_u32(op(&t, f, 0xa0), SCRATCH + 0x7c4);
        t.set_u32(SCRATCH + 0x7c4, 0x0bad_0000 + fade);
        t.set_u32(op(&t, f, 0xee), SCRATCH + 0x7c8);
        t.set_u32(op(&t, f, 0x11a), fade);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn host_starts_bank() {
    let Some(mut t) = twin(393) else { return };
    let f = t.at(0x10a0);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (523, 4),
            (718, 5),
            (222, 9),
            (221, 12),
        ],
    );
    rec(&mut t, 790, 0, |_, _| 0x790);
    let payloads = op(&t, f, 0x81);
    let done = op(&t, f, 0x138);
    for (n, bank) in [(0u32, 0u32), (3, 0x8000_0000), (8, 7), (1, 0xffff_ffff)] {
        block_stubs(&mut t, n);
        t.stub_st0(790, move |_, _| f64::from(n) * 0.75);
        rec(&mut t, 627, 2, move |_, _| bank);
        t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
        t.set_u32(done, 0x77);
        for o in [0x57, 0x61, 0x6b, 0xa3, 0xab] {
            t.set_u32(op(&t, f, o), 0x0101_0101);
        }
        t.check_entry(f, Call::default(), &[0x00ab_c000 + n], Eax::Checked);
    }
}

/// Runs an object host's start (`host_start_head` first) for a few
/// object counts and slot 790 floats; `done` is the operand of its
/// `*G_DONE = 0` store, `vary` sets the case's own words.
fn host_runs(t: &mut Twin, f: u32, done: u32, vary: impl Fn(&mut Twin, u32)) {
    rec(t, 790, 0, |_, _| 0x790);
    let payloads = op(t, f, 0x81);
    let done = op(t, f, done);
    for n in [0u32, 3, 8] {
        block_stubs(t, n);
        t.stub_st0(790, move |_, _| f64::from(n) * 0.75 - 1.0);
        vary(t, n);
        t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
        t.set_u32(done, 0x77);
        for o in [0x57, 0x61, 0x6b, 0xa3, 0xab] {
            t.set_u32(op(t, f, o), 0x0101_0101);
        }
        t.check_entry(f, Call::default(), &[0x00ab_c000 + n], Eax::Checked);
    }
}

#[test]
fn host_starts_523() {
    for (dll, shift) in [(53u16, 10u32), (52, 11)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x1000);
        recs(
            &mut t,
            &[
                (556, 1),
                (555, 2),
                (331, 3),
                (720, 2),
                (523, 4),
                (718, 7),
                (222, 11),
                (221, 12),
            ],
        );
        let table = op(&t, f, 0xe9);
        host_runs(&mut t, f, 0x149, move |t, n| {
            t.set_u32(op(t, f, 0xc7), (n << shift) ^ (1 << (shift + 1)));
            t.set_u32(op(t, f, 0xe2), n % 3);
            for i in 0..3 {
                t.set_u32(table + 4 * i, 0x0523_0000 + i + n);
            }
        });
    }
}

#[test]
fn host_starts_streams() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x10a0);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (718, 3),
            (222, 7),
            (221, 11),
            (551, 6),
        ],
    );
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    host_runs(&mut t, f, 0x175, |_, _| {});
}

#[test]
fn host_starts_stream_and_two() {
    let Some(mut t) = twin(102) else { return };
    let f = t.at(0x10a0);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (718, 3),
            (222, 7),
            (221, 11),
            (551, 12),
        ],
    );
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    host_runs(&mut t, f, 0x13b, |_, _| {});
    drop(t);
    let Some(mut t) = twin(511) else { return };
    let f = t.at(0x10a0);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (718, 3),
            (222, 7),
            (221, 11),
        ],
    );
    host_runs(&mut t, f, 0x140, |_, _| {});
}

/// Slots 262 (`Ch_GetPos`: three float pointers) and 292
/// (`MsGetMapCenter`: a vector) answering from the words at `POS`.
const POS: u32 = SCRATCH + 0x3100;

fn position_stubs(t: &mut Twin) {
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = m.r32(POS + 4 * i);
            m.w32(a[1 + i as usize], v);
        }
        0x262
    });
    rec(t, 292, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(POS + 0x10 + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x292
    });
    rec(t, 576, 3, |_, _| 0x576);
}

/// Positions and centres whose yaws reach every wrap of
/// `face_map_centre`.
const FACINGS: &[([f32; 3], [f32; 4])] = &[
    ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0, 9.0]),
    ([0.0, 1.0, 0.0], [-1.0, 0.0, 0.5, 9.0]),
    ([1.0, 2.0, 3.0], [1.0, 0.0, -4.0, 9.0]),
    ([1.0, 2.0, 3.0], [-2.0, 0.0, 3.5, 9.0]),
    ([0.5, 0.0, 0.5], [0.5, 0.0, 0.5, 9.0]),
    ([2.0, 0.0, 0.0], [-3.0, 0.0, -0.1, 9.0]),
];

fn set_facing(t: &mut Twin, (pos, centre): ([f32; 3], [f32; 4])) {
    for (i, v) in (0u32..).zip(pos) {
        t.set_u32(POS + 4 * i, v.to_bits());
    }
    for (i, v) in (0u32..).zip(centre) {
        t.set_u32(POS + 0x10 + 4 * i, v.to_bits());
    }
}

#[test]
fn faces_map_centre() {
    let Some(mut t) = twin(307) else { return };
    let f = t.at(0x13f0);
    position_stubs(&mut t);
    for &c in FACINGS {
        set_facing(&mut t, c);
        t.set_u32(op(&t, f, 0x2a), 0x0262_0001);
        t.set_u32(op(&t, f, 0x107), 0x0576_0001);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn host_starts_facing() {
    let Some(mut t) = twin(307) else { return };
    let f = t.at(0x10a0);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (718, 3),
            (222, 7),
            (221, 11),
        ],
    );
    position_stubs(&mut t);
    host_runs(&mut t, f, 0x145, |t, n| {
        set_facing(t, FACINGS[n as usize % 6]);
    });
}

#[test]
fn model_draws() {
    let Some(mut t) = twin(563) else { return };
    let f = t.at(0x2060);
    recs(
        &mut t,
        &[(723, 3), (724, 1), (231, 3), (819, 4), (870, 6), (871, 5)],
    );
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    let ready = SCRATCH + 0x7e0;
    rec(&mut t, 773, 2, move |m, _| m.r32(ready));
    let lock = SCRATCH + 0x7f0;
    let b = SCRATCH + 0x4000;
    for r in [0u32, 1, 0x100] {
        t.write(b, &varied(0x4000, r));
        t.set_u32(op(&t, f, 0x7b), b);
        t.set_u32(op(&t, f, 0x8f), lock);
        t.set_u32(lock, 0x5a5a_0000 + r);
        t.set_u32(ready, r);
        let c = Call {
            ecx: 0x0123_4560 + r,
            edx: SCRATCH + 0x900,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s5_misc::model_draw(g, img, fr.at, fr.esp, fr.ecx, fr.edx))
        });
    }
}

#[test]
fn four_part_tasks() {
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x9970);
    let task = SCRATCH + 0x300;
    let rec_at = SCRATCH + 0x400;
    let node = SCRATCH + 0x600;
    let parts = SCRATCH + 0x1000;
    for (state, count) in [
        (0u32, 0u32),
        (5, 0),
        (10, 0),
        (11, 0),
        (12, 3),
        (13, 4),
        (13, 5),
        (13, 0xffff_ffff),
        (16, 0),
        (0x14, 0),
        (0x15, 0),
        (0x1e, 0),
        (0x1f, 0),
        (0x2710, 0),
        (0x2711, 0),
        (0x8000_0000, 0),
    ] {
        t.write(rec_at, &varied(0x60, state));
        t.write(parts, &varied(0x500, state + count));
        t.write(node, &varied(0x40, state));
        t.set_u32(task, node);
        t.set_u32(task + 4, rec_at);
        for i in 0..5 {
            t.set_u32(rec_at + 0x1c + 4 * i, parts + 0x100 * i);
        }
        t.set_u32(parts + 0x400 + 0x30, 2.5f32.to_bits());
        t.set_u32(node + 4, SCRATCH + 0x700);
        t.set_u32(node + 8, SCRATCH + 0x740);
        t.set_u32(rec_at + 0x18, state);
        t.set_u32(rec_at + 0x40, count);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn moving_parts() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x1860);
    recs(
        &mut t,
        &[
            (229, 3),
            (325, 2),
            (258, 1),
            (231, 3),
            (230, 2),
            (713, 1),
            (714, 5),
        ],
    );
    let answer = SCRATCH + 0x7e0;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let table = SCRATCH + 0x700;
    rec(&mut t, 941, 2, move |_, _| table);
    let start = t.img.rel32_target(f + 0x10a);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f0);
    let obj = SCRATCH + 0x1000;
    for (ended, fade, life) in [
        (0u32, 0u32, 0x100u16),
        (1, 0, 0x100),
        (0, 1, 0x89),
        (5, 2, 0x88),
    ] {
        t.write(obj, &varied(0x100, ended + fade));
        t.write(table, &varied(0x40, life.into()));
        t.set_u16(obj + 0x76, life);
        t.set_u32(answer, ended);
        for o in [0x55u32, 0x60, 0x6c, 0x7f] {
            t.set_u32(op(&t, f, o), 0x0100_0000 + o);
        }
        t.set_u32(op(&t, f, 0x8c), SCRATCH + 0x7c0);
        t.set_u32(op(&t, f, 0x99), SCRATCH + 0x7c4);
        t.set_u32(SCRATCH + 0x7c4, 0x0bad_0000 + fade);
        t.set_u32(op(&t, f, 0xe7), SCRATCH + 0x7c8);
        t.set_u32(op(&t, f, 0x113), fade);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn thickness_margins() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x14b0);
    let model = SCRATCH + 0x7e0;
    rec(&mut t, 390, 1, move |m, _| m.r32(model));
    rec(&mut t, 215, 2, |_, a| 0x0215_0000 ^ a[0]);
    rec(&mut t, 19, 1, |_, a| 0x0019_0000 ^ a[0]);
    t.stub_st0(19, |m, _| f64::from(m.r32(SCRATCH + 0x7e0) & 0xff) * 0.125);
    for id in [
        0x108cu32, 0x102d, 0x1015, 0x111a, 0x112c, 0x1127, 0x1119, 0x2000, 0,
    ] {
        t.set_u32(model, id);
        t.set_u32(op(&t, f, 0xc), 0x0188_0000 + id);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            s5_misc::thickness_margin(g, img, fr.at)
        });
    }
}

#[test]
fn ppp_effect_setups() {
    let Some(mut t) = twin(292) else { return };
    let f = t.at(0x1240);
    recs(&mut t, &[(718, 1), (222, 5), (711, 11), (710, 12)]);
    let data = SCRATCH + 0x800;
    rec(&mut t, 719, 8, move |_, _| data);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    let lock = SCRATCH + 0x7f0;
    for k in 0..2u32 {
        t.write(data, &varied(0x40, k));
        t.write(op(&t, f, 0x118), &varied(0x110, k + 5));
        t.set_u32(op(&t, f, 0x74), lock);
        t.set_u32(lock, 0x0bad_0000 + k);
        t.set_u32(op(&t, f, 0xa6), 0x0a6a_6a00 + k);
        t.check_entry(f, Call::default(), &[0x0292_0000 + k], Eax::Checked);
    }
}

#[test]
fn ppp_effect_setups_6() {
    let Some(mut t) = twin(662) else { return };
    let f = t.at(0x14b0);
    recs(&mut t, &[(718, 1), (711, 7), (710, 11)]);
    let data = SCRATCH + 0x800;
    rec(&mut t, 719, 4, move |_, _| data);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    let lock = SCRATCH + 0x7f0;
    for k in 0..2u32 {
        t.write(data, &varied(0x40, k));
        t.write(op(&t, f, 0x11e), &varied(0x110, k + 5));
        t.set_u32(op(&t, f, 0x7d), lock);
        t.set_u32(lock, 0x0bad_0000 + k);
        t.check_entry(f, Call::default(), &[0x0662_0000 + k], Eax::Checked);
    }
}

#[test]
fn packed_textures() {
    let Some(mut t) = twin(348) else { return };
    let f = t.at(0x51f0);
    let answer = SCRATCH + 0x7e0;
    rec_mem(&mut t, 776, 1, 0, None);
    rec_mem(&mut t, 905, 9, 4, None);
    let row = SCRATCH + 0x7a0;
    t.stub(885, move |m, a| {
        for (i, x) in (0u32..).zip(&a[..7]) {
            m.w32(row + 4 * i, *x);
        }
        m.r32(answer)
    });
    let model = SCRATCH + 0x1000;
    let lock = SCRATCH + 0x7f0;
    t.set_u32(op(&t, f, 0x3c), lock);
    for (filled, lo, hi, n) in [
        (0u32, 0x1234_5678u32, 0x9abc_def0u32, 3u32),
        (1, 0xffff_ffff, 0xffff_ffff, 0xffff_fff0),
        (7, 0x4321_8765, 0x0000_0002, 0x0100_0001),
        (1, 0, 0, 0),
    ] {
        t.set_u32(answer, filled);
        t.set_u32(lock, 0x5a5a_0000 + n);
        t.set_u32(model + 0x830, 0x0830_0830 + n);
        let c = Call::this(model);
        let args = [
            7,
            0x0a0a_0000 + n,
            lo,
            hi,
            1.5f32.to_bits(),
            (-2.0f32).to_bits(),
            n,
        ];
        t.check_entry(f, c, &args, Eax::Checked);
    }
}

/// A free ring of `n` nodes for the pool's take at `take` (its count and
/// head operands), 0x40 bytes apart from `at`.
fn free_ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 1..=n {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
    }
    t.set_u32(t.img.word(take + 0x17), node(1));
    t.set_u16(t.img.word(take + 0x04), n as u16);
}

#[test]
fn two_lights_starts() {
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x9450);
    let take = t.img.rel32_target(f + 0x67);
    let base = SCRATCH + 0x4000;
    let (head, tail) = (op(&t, f, 0x6d), op(&t, f, 0x83));
    for k in 0..3u32 {
        free_ring(&mut t, take, SCRATCH + 0x1000, 4);
        t.write(base, &varied(0x1900, k));
        t.set_u32(op(&t, f, 0x4), base);
        if k == 0 {
            t.set_u32(head, 0);
            t.set_u32(tail, 0);
        } else {
            t.write(SCRATCH + 0x1400, &varied(0x1c, k));
            t.set_u32(head, SCRATCH + 0x1400);
            t.set_u32(tail, SCRATCH + 0x1400);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn targets_mean_offsets() {
    let Some(mut t) = twin(150) else { return };
    let f = t.at(0xafc0);
    let live = SCRATCH + 0x7e0;
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, (0.5f32 + i as f32).to_bits());
        }
        0x325
    });
    rec(&mut t, 684, 1, move |m, a| m.r32(live) >> (a[0] & 7) & 1);
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, (a[0] as f32 * 1.25 - i as f32).to_bits());
        }
        0x531 ^ a[0]
    });
    let out = SCRATCH + 0x300;
    let ids = op(&t, f, 0x63);
    for (n, mask) in [
        (0u32, 0u32),
        (3, 0b101),
        (5, 0),
        (4, 0xff),
        (0x8000_0000, 1),
    ] {
        for i in 0..8u32 {
            t.set_u8(ids + i, (i * 3 % 8) as u8);
        }
        t.set_u32(op(&t, f, 0x4c), n);
        t.set_u32(live, mask);
        t.write(out, &varied(0x10, n));
        t.check_entry(f, Call::this(out), &[], Eax::Checked);
    }
}

#[test]
fn three_charas_reset() {
    let Some(mut t) = twin(349) else { return };
    let f = t.at(0x4980);
    recs(
        &mut t,
        &[
            (26, 3),
            (10, 2),
            (210, 2),
            (211, 2),
            (217, 2),
            (255, 2),
            (257, 2),
        ],
    );
    rec(&mut t, 0, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x7e0);
        m.w32(SCRATCH + 0x7e0, k + 1);
        0x0100_0000 + a[0] + k
    });
    let s = SCRATCH + 0x4000;
    t.write(s, &varied(0x1a40, 3));
    t.set_u32(op(&t, f, 0xb), s);
    let sel = SCRATCH + 0x700;
    for i in 0..3u32 {
        t.set_u8(sel + i, (2 - i) as u8);
    }
    t.set_u32(op(&t, f, 0x146), sel);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn fading_model_tasks() {
    let Some(mut t) = twin(578) else { return };
    let f = t.at(0x7e30);
    recs(&mut t, &[(256, 5)]);
    let task = SCRATCH + 0x300;
    let rec_at = SCRATCH + 0x400;
    let node = SCRATCH + 0x600;
    let model = SCRATCH + 0x800;
    for (state, timer, fade) in [
        (0u32, 5u32, 0x3cu8),
        (0, 5, 0x10),
        (1, 0, 0x3c),
        (1, 0, 0x3b),
        (2, 1, 0),
        (2, 0, 0),
        (2, 0xffff_fff0, 0),
        (3, 0, 4),
        (3, 0, 8),
        (4, 0, 0),
        (10, 9, 0),
        (11, 1, 0),
        (11, 0, 0),
        (12, 0, 0),
        (0x8000_0000, 0, 0),
    ] {
        t.write(rec_at, &varied(0x100, state + timer));
        t.write(node, &varied(0x40, state));
        t.set_u32(task, node);
        t.set_u32(task + 4, rec_at);
        t.set_u32(node + 4, SCRATCH + 0x700);
        t.set_u32(node + 8, SCRATCH + 0x740);
        t.set_u32(rec_at + 0x44, model);
        t.set_u32(rec_at + 0x18, state);
        t.set_u32(rec_at + 0x34, timer);
        t.set_u8(rec_at + 0xc3, fade);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn target_measures() {
    let Some(mut t) = twin(667) else { return };
    let f = t.at(0x29a0);
    rec_mem(&mut t, 215, 1, 0, None);
    rec_mem(&mut t, 217, 2, 4, Some(1));
    rec_mem(&mut t, 210, 2, 16, Some(1));
    rec_mem(&mut t, 230, 2, 16, Some(0));
    rec_mem(&mut t, 290, 2, 4, Some(1));
    for s in [169u32, 251, 216, 277] {
        rec_mem(&mut t, s, 1, 0, None);
    }
    let r = SCRATCH + 0x1000;
    for (id, dir, wid, hgt) in [
        (0xffu32, 0.0f64, 0.0f64, 0.0f64),
        (3, 1.5, 2.0, 1.0),
        (4, -2.5, 0.5, 3.0),
        (5, 100.0, 0.0001, 0.00001),
        (6, 1.0e10, 1.0, 1.0),
    ] {
        t.stub_st0(169, move |_, _| dir);
        t.stub_st0(251, move |_, _| wid);
        t.stub_st0(216, move |_, _| hgt);
        t.stub_st0(277, move |_, _| wid + 0.5);
        t.write(r, &varied(0x100, id));
        t.set_u32(r, id);
        t.check_entry(f, Call::this(r), &[], Eax::Checked);
    }
}

#[test]
fn weapon_swings() {
    let Some(mut t) = twin(555) else { return };
    let f = t.at(0x2a90);
    rec_mem(&mut t, 562, 0, 0, None);
    rec_mem(&mut t, 219, 2, 0, None);
    rec_mem(&mut t, 235, 1, 16, Some(0));
    for s in [277u32, 15, 16, 17] {
        rec_mem(&mut t, s, 1, 0, None);
        t.stub_st0(s, move |_, _| f64::from(s) / 64.0);
    }
    for s in [330u32, 328, 329] {
        rec_mem(&mut t, s, 3, 16, None);
    }
    rec_mem(&mut t, 825, 1, 0, None);
    rec_mem(&mut t, 913, 3, 16, None);
    rec_mem(&mut t, 713, 1, 0, None);
    // the child start (`start_child_b`) and the object start it calls
    let child = t.img.rel32_target(f + 0x12a);
    let soa = t.img.rel32_target(child + 0x2e);
    t.set_u32(op(&t, soa, 0x04), SCRATCH + 0x200);
    t.set_u32(op(&t, soa, 0x5e), SCRATCH + 0x280);
    for i in 0..0x30 {
        t.set_u32(SCRATCH + 0x3400 + 4 * i, 0x1111 * (i + 1));
    }
    rec_mem(&mut t, 941, 1, 0, None);
    t.stub(941, |_, _| SCRATCH + 0x3400);
    rec_mem(&mut t, 714, 5, 0, None);
    rec_mem(&mut t, 641, 1, 0, None);
    let map = SCRATCH + 0x5000;
    t.write(map, &[0x21, 0x22, 0x23, 0x24]);
    t.set_u32(op(&t, child, 0x82), 0x00a0_29e0);
    t.set_u32(op(&t, child, 0x92), map);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = op(&t, pa, 0x04);
    let list = op(&t, f, 0x134);
    let task = SCRATCH + 0x300;
    let obj = SCRATCH + 0x1000;
    let s = SCRATCH + 0x8000;
    t.set_u32(op(&t, f, 0x21), s);
    t.set_u32(op(&t, f, 0x142), SCRATCH + 0x2f0);
    for (count, total, listed) in [
        (5u32, 10u32, false),
        (1, 10, false),
        (0, 7, false),
        (0, 7, true),
    ] {
        t.write(obj, &varied(0x600, count));
        t.write(task, &varied(0x40, total));
        t.write(s + 0x8520, &varied(0x10, count));
        t.set_u32(task, 0);
        t.set_u32(task + 4, 0);
        // the node's payload: no children to give back first
        t.set_u32(task + 8, SCRATCH + 0x340);
        t.set_u32(SCRATCH + 0x34c, 0);
        t.set_u32(task + 0x14, obj);
        t.set_u32(obj + 0x50c, count);
        t.set_u32(obj + 0x510, total);
        t.set_u32(obj + 0x514, u32::MAX);
        t.set_u16(free, 3);
        // the DLL's object list: empty, or one node holding the object
        let node = SCRATCH + 0x380;
        t.set_u32(list, if listed { node } else { 0 });
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, obj);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn shade_reset_colour_tasks() {
    let Some(mut t) = twin(165) else { return };
    let f = t.at(0x3410);
    recs(&mut t, &[(256, 5)]);
    let s = SCRATCH + 0x4000;
    let sel = SCRATCH + 0x700;
    let free = op(&t, f, 0xe3);
    let (head, tail) = (op(&t, f, 0x14a), op(&t, f, 0x162));
    // (a count of 0 takes no node: the original then writes through 0)
    for (n, count, listed) in [
        (0u32, 1u16, false),
        (2, 1, false),
        (3, 3, true),
        (0, 2, true),
    ] {
        t.write(s + 0x5a00, &varied(0x80, n));
        t.set_u32(op(&t, f, 0x6), s);
        t.set_u32(op(&t, f, 0xc), n);
        t.set_u32(op(&t, f, 0x1f), sel);
        for i in 0..4u32 {
            t.set_u8(sel + i, (3 - i) as u8);
        }
        t.set_u16(op(&t, f, 0x6d), count);
        // a ring of three free nodes
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &varied(0x1c, i + n));
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(free, node(0));
        t.write(SCRATCH + 0x1400, &varied(0x1c, 9));
        let last = if listed { SCRATCH + 0x1400 } else { 0 };
        t.set_u32(head, last);
        t.set_u32(tail, last);
        t.check_entry(
            f,
            Call {
                eax: 0x0eaa_0000,
                ..Call::default()
            },
            &[],
            Eax::Checked,
        );
    }
}

#[test]
fn effect_ends_shown() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x1cc0);
    recs(
        &mut t,
        &[
            (6, 2),
            (547, 1),
            (295, 2),
            (541, 3),
            (38, 3),
            (568, 1),
            (566, 1),
            (546, 1),
            (271, 1),
        ],
    );
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 779, 4, |_, _| 0x779);
    rec(&mut t, 574, 0, |_, _| 0x574);
    let s = SCRATCH + 0x4000;
    let lock = SCRATCH + 0x7f0;
    let held = SCRATCH + 0x600;
    for (alt, id, counts, hold) in [
        (0u32, 3u32, [0u32, 0, 0, 0], false),
        (0, 0x20, [2, 0, 1, 3], true),
        (1, 3, [1, 1, 1, 1], false),
    ] {
        t.write(s, &varied(0x3200, id));
        t.set_u32(op(&t, f, 0x102), s);
        for (k, c) in (0u32..).zip(counts) {
            t.set_u32(s + 0xea8 + 0x8a8 * k + 0x824, c);
        }
        t.set_u32(op(&t, f, 0x48), alt);
        t.set_u32(op(&t, f, 0x50), id);
        t.set_u32(op(&t, f, 0x146), lock);
        t.set_u32(lock, 0x5a5a_0000 + id);
        if hold {
            // a node with no children, unlinked
            t.write(held, &varied(0x20, 1));
            t.set_u32(held, 0);
            t.set_u32(held + 4, 0);
            t.set_u32(held + 8, SCRATCH + 0x640);
            t.set_u32(SCRATCH + 0x64c, 0);
            t.set_u32(op(&t, f, 0x90), held);
        } else {
            t.set_u32(op(&t, f, 0x90), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn screen_fade_steps() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x23b0);
    recs(&mut t, &[(801, 6), (785, 1)]);
    rec(&mut t, 353, 1, |m, a| m.r32(a[0]) ^ m.r32(a[0] + 12));
    rec(&mut t, 397, 2, |m, a| m.r32(a[0]) ^ m.r32(a[1] + 4));
    let k = |t: &Twin, o: u32| f32::from_bits(t.u32(op(t, f, o)));
    let (lo, hi, up, dn) = (k(&t, 0x26), k(&t, 0x45), k(&t, 0x30), k(&t, 0x85));
    for (timer, x, z) in [
        (0xb0u32, lo, 2.0f32),
        (0xb0, hi - up, 2.0),
        (0xb0, hi, 2.0),
        (0xb0, lo - 2.0 * up, 2.0),
        (0xb0, f32::NAN, 2.0),
        (0xaf, hi + 1.0, 0.5),
        (0xaf, lo + dn * 0.5, 0.5),
        (0xaf, lo.midpoint(hi), 0.0),
        (0, f32::NAN, 1.0),
    ] {
        t.set_u32(op(&t, f, 0x12), timer);
        t.set_u32(op(&t, f, 0x1e), x.to_bits());
        t.set_u32(op(&t, f, 0x12c), z.to_bits());
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn angle_keys_sampled() {
    let Some(mut t) = twin(55) else { return };
    let f = t.at(0x31a0);
    let keys = SCRATCH + 0x400;
    let angles = [
        [0.0f32, 1.0, 2.0, 3.0, -3.0, -2.0],
        [3.0, -3.0, 3.1, -3.1, 0.5, 2.9],
        [-1.0, 3.1, -3.1, 1.5, -1.5, 0.0],
    ];
    for set in angles {
        t.write(keys, &varied(0x60, 1));
        for (i, a) in (0u32..).zip(set) {
            t.set_u32(keys + 16 * i, a.to_bits());
        }
        for time in [
            0.0f32, 0.25, 0.5, 0.75, 1.0, 1.3, 2.6, 3.0, 3.9, 4.2, 4.8, 5.0,
        ] {
            let c = Call::this(keys).with_xmm(&[Xmm::f32(time)]);
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                let (r, eax) = s5_misc::angle_keys(g, img, fr.at, fr.ecx, fr.xmm_f32(0));
                Ret::xmm0(Xmm::f32(r), eax)
            });
        }
    }
}

#[test]
fn summon_ppp_starts() {
    let Some(mut t) = twin(342) else { return };
    let f = t.at(0x71b0);
    let ti = t.img.rel32_target(f + 0x8);
    let statics = t.img.rel32_target(ti + 0xe0);
    let root = op(&t, statics, 0x10d);
    let list = SCRATCH + 0x6000;
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
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
    recs(
        &mut t,
        &[(718, 1), (711, 7), (710, 11), (222, 12), (221, 12)],
    );
    let data = SCRATCH + 0x800;
    rec(&mut t, 719, 4, move |_, _| data);
    let lock = SCRATCH + 0x7f0;
    for count in [0u32, 2] {
        t.write(list, &[0x80, 0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]);
        t.set_u32(list + 0x7c, count);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, 0);
        t.set_u32(op(&t, f, 0xd4), lock);
        t.set_u32(lock, 0x0bad_0000 + count);
        // the ring's tail before: a record of its own
        t.write(SCRATCH + 0x7a0, &varied(0x10, count));
        t.set_u32(op(&t, f, 0x15a), SCRATCH + 0x7a0);
        t.set_u32(op(&t, f, 0x197), 0xffff_ff0f + count);
        t.check_entry(f, Call::default(), &[0x0342_0040 + count], Eax::Checked);
    }
}

#[test]
fn images_stored_and_loaded() {
    let Some(mut t) = twin(662) else { return };
    let f = t.at(0x1000);
    recs(
        &mut t,
        &[
            (426, 8),
            (421, 1),
            (424, 2),
            (222, 4),
            (378, 1),
            (380, 2),
            (379, 3),
            (425, 8),
            (382, 2),
            (423, 2),
            (253, 0),
        ],
    );
    let tl = t.img.rel32_target(f + 0x10c);
    let live = op(&t, tl, 0x06);
    for n in [0u32, 0x12d, 0x12e, 0x8000_0001] {
        t.set_u32(op(&t, f, 0x9d), n);
        // a timeline with no events (live or not): only its clock moves
        t.set_u32(live, n & 1);
        t.set_u32(op(&t, tl, 0x25), 0);
        t.set_u32(op(&t, f, 0x100), SCRATCH + 0x7c0);
        t.set_u32(op(&t, f, 0x105), 0x0105_0000 + n);
        for o in [0x32u32, 0x7d, 0xbf, 0x14b, 0x197] {
            t.set_u32(op(&t, f, o), 0x0100_0000 + o + n);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn target_measures_modes() {
    let Some(mut t) = twin(515) else { return };
    let f = t.at(0x29a0);
    rec_mem(&mut t, 215, 1, 0, None);
    rec_mem(&mut t, 217, 2, 4, Some(1));
    rec_mem(&mut t, 210, 2, 16, Some(1));
    rec_mem(&mut t, 230, 2, 16, Some(0));
    rec_mem(&mut t, 290, 2, 4, Some(1));
    for s in [169u32, 251, 216, 277] {
        rec_mem(&mut t, s, 1, 0, None);
    }
    let r = SCRATCH + 0x1000;
    for (id, wid, hgt, modes) in [
        (0xffu32, 0.0f64, 0.0f64, [0u32, 0, 0]),
        (3, 2.0, 1.0, [1, 0, 0]),
        (4, 0.5, 3.0, [0, 1, 0]),
        (5, 2.5, 1.5, [0, 0, 1]),
        (6, 1.5, 4.0, [0, 0, 0]),
        (7, 0.0001, 0.00001, [0, 0, 0]),
    ] {
        t.stub_st0(169, move |_, _| wid - hgt);
        t.stub_st0(251, move |_, _| wid);
        t.stub_st0(216, move |_, _| hgt);
        t.stub_st0(277, move |_, _| wid + 0.5);
        for (o, v) in [0x10cu32, 0x115, 0x138].into_iter().zip(modes) {
            t.set_u32(op(&t, f, o), v);
        }
        t.write(r, &varied(0x100, id));
        t.set_u32(r, id);
        t.check_entry(f, Call::this(r), &[], Eax::Checked);
    }
}

#[test]
fn model_parts_drawn() {
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x3ce0);
    rec_mem(&mut t, 842, 3, 16, None);
    rec_mem(&mut t, 843, 2, 16, None);
    rec_mem(&mut t, 231, 3, 16, Some(0));
    let table = SCRATCH + 0x3000;
    t.stub(941, move |_, _| table);
    let obj = SCRATCH + 0x1000;
    let d = SCRATCH + 0x2000;
    let ptrs = SCRATCH + 0x2100;
    let mtx = SCRATCH + 0x2800;
    // (parts, the entries' part numbers)
    for (parts, entries) in [
        (0u16, &[][..]),
        (2, &[1u16, 0][..]),
        (3, &[2, 0][..]),
        (3, &[][..]),
        (1, &[5, 6][..]),
    ] {
        t.write(obj, &varied(0x900, u32::from(parts)));
        t.write(mtx, &varied(0x40, 3));
        t.set_u32(obj + 0x1c0, d);
        t.set_u16(d + 6, parts);
        t.set_u16(d + 8, entries.len() as u16);
        t.set_u32(d + 0x14, 0x0941_0014);
        for (k, e) in (0u32..).zip(entries) {
            t.set_u16(table + 12 * k + 2, *e);
        }
        t.set_u32(obj + 0x1bc, ptrs);
        for i in 0..4u32 {
            t.set_u32(ptrs + 0x38 * i, SCRATCH + 0x2400 + 4 * i);
            t.set_u16(SCRATCH + 0x2400 + 4 * i, (i * 3) as u16);
        }
        t.set_u32(obj + 0x328, SCRATCH + 0x8000);
        let c = Call {
            ecx: obj,
            edx: mtx,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s5_misc::model_parts_draw(
                g, img, fr.at, fr.esp, fr.ecx, fr.edx,
            ))
        });
    }
}

#[test]
fn root_point_parts() {
    let Some(mut t) = twin(530) else { return };
    let f = t.at(0x14e0);
    rec_mem(&mut t, 210, 2, 16, Some(1));
    rec_mem(&mut t, 235, 1, 16, Some(0));
    rec_mem(&mut t, 231, 3, 16, Some(0));
    rec_mem(&mut t, 229, 3, 4, Some(0));
    rec_mem(&mut t, 325, 2, 4, Some(0));
    rec_mem(&mut t, 230, 2, 16, Some(0));
    let answer = SCRATCH + 0x7e0;
    t.stub(712, move |m, _| m.r32(answer));
    let obj = SCRATCH + 0x1000;
    let lock = SCRATCH + 0x7f0;
    for (alt, ran) in [(0u32, 0u32), (1, 1), (2, 0)] {
        t.write(obj, &varied(0x80, alt));
        t.set_u32(op(&t, f, 0x76), alt);
        t.set_u32(answer, ran);
        for o in [0x26u32, 0x127, 0x140, 0x14c, 0x17b] {
            t.set_u32(op(&t, f, o), SCRATCH + 0x4000 + 0x100 * (o % 8));
        }
        t.set_u32(op(&t, f, 0x188), SCRATCH + 0x7c0);
        t.set_u32(op(&t, f, 0x196), lock);
        t.set_u32(lock, 0x5a5a_0000 + alt);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn summon_ppp_starts_stream() {
    let Some(mut t) = twin(186) else { return };
    let f = t.at(0x8230);
    let ti = t.img.rel32_target(f + 0x8);
    let statics = t.img.rel32_target(ti + 0xe0);
    let root = op(&t, statics, 0x10d);
    let list = SCRATCH + 0x6000;
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
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
    recs(
        &mut t,
        &[
            (718, 1),
            (711, 7),
            (710, 11),
            (222, 12),
            (221, 12),
            (551, 6),
        ],
    );
    let data = SCRATCH + 0x800;
    rec(&mut t, 719, 3, move |_, a| data ^ a[2]);
    let lock = SCRATCH + 0x7f0;
    for count in [0u32, 2] {
        t.write(list, &[0x80, 0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]);
        t.set_u32(list + 0x7c, count);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, 0);
        t.set_u32(op(&t, f, 0xd4), lock);
        t.set_u32(lock, 0x0bad_0000 + count);
        t.write(SCRATCH + 0x7a0, &varied(0x10, count));
        t.set_u32(op(&t, f, 0x185), SCRATCH + 0x7a0);
        t.set_u32(op(&t, f, 0x1c2), 0xffff_ff0f + count);
        t.check_entry(f, Call::default(), &[0x0186_0040 + count], Eax::Checked);
    }
}

#[test]
fn spark_pairs_started() {
    let Some(mut t) = twin(349) else { return };
    let f = t.at(0x54c0);
    let (head, tail) = (op(&t, f, 0xc7), op(&t, f, 0xd2));
    for (n, listed) in [(3u32, false), (4, false), (6, true), (0xffff_fff0, false)] {
        // a ring of eight free nodes
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        for i in 0..8u32 {
            t.write(node(i), &varied(0x1c, i + n));
            t.set_u32(node(i), node((i + 7) % 8));
            t.set_u32(node(i) + 4, node((i + 1) % 8));
        }
        t.set_u32(op(&t, f, 0x32), node(0));
        t.set_u16(op(&t, f, 0x23), 8);
        t.set_u32(op(&t, f, 0x8), n);
        t.write(SCRATCH + 0x1400, &varied(0x1c, 9));
        let last = if listed { SCRATCH + 0x1400 } else { 0 };
        t.set_u32(head, last);
        t.set_u32(tail, last);
        t.check_entry(
            f,
            Call {
                eax: 0x0eaa_0000,
                ..Call::default()
            },
            &[],
            Eax::Checked,
        );
    }
}
