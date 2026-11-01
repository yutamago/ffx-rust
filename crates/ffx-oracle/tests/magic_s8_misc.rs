//! The shared-8 list's ports (`ffx_magic::s8_misc`) against the DLLs
//! they were ported from and the other DLLs that carry them. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

#[path = "magic/l5.rs"]
mod l5;
mod magic;

use ffx_magic::{Ret, Xmm, s8_misc};
use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The word at `f + off` of the code.
fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

/// `n` bytes that differ from their neighbours (`seed` picks the pattern).
fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// `n` words of small floats that differ from their neighbours.
fn floats(n: u32, seed: u32) -> Vec<u8> {
    (0..n)
        .flat_map(|i| {
            let x = ((i * 37 + seed * 11) % 41) as f32 - 20.0;
            (x * 0.125).to_bits().to_le_bytes()
        })
        .collect()
}

fn set_f(t: &mut Twin, a: u32, x: f32) {
    t.set_u32(a, x.to_bits());
}

/// The entry's stack pointer (at the return address).
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// The frame area below the entry's stack pointer filled with a pattern
/// before a run (the stores of the run before, or zeros, would hide a
/// port's dropped store from the stubs).
fn scrub(t: &mut Twin) {
    t.write(ENTRY_ESP - 0x3000, &varied(0x3000, 0x5b));
}

/// The call log: a count at `LOG`, then rows of 0x40 bytes.
const LOG: u32 = SCRATCH + 0x6000;
/// The log's rows.
const ROWS: u32 = 128;

/// Logs each call of `slot` into a row of the shared log (in call order):
/// the slot, its first `n` words, and 4 words at each argument `ptrs`
/// names. Answers `ret(m, args, k)`, `k` the call's number.
fn log_calls(
    t: &mut Twin,
    slot: u32,
    n: usize,
    ptrs: &'static [usize],
    ret: impl Fn(&mut dyn StubMem, &[u32], u32) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let k = m.r32(LOG);
        m.w32(LOG, k + 1);
        let row = LOG + 4 + 0x40 * (k % ROWS);
        m.w32(row, slot);
        for (i, w) in (1u32..).zip(&a[..n.min(a.len())]) {
            m.w32(row + 4 * i, *w);
        }
        for (j, &p) in (0u32..).zip(ptrs) {
            for i in 0..4 {
                m.w32(row + 0x20 + 0x10 * j + 4 * i, m.r32(a[p] + 4 * i));
            }
        }
        ret(m, a, k)
    });
}

/// The camera rigs' step in eight of the 25 DLLs: both point tracks in
/// every state (the blend before, at and past its end, the drift's last
/// and other steps, the key and angle steps, nothing), the roll's blend
/// (wrapped up and down) and eased keys, the screen depth's keys at each
/// branch (first, middle and last keys, the fraction 0, before and after
/// the half), the drift, the camera set with and without the turn.
#[test]
fn camera_rigs_steps() {
    for (dll, rva) in [
        (162u16, 0x5370u32),
        (165, 0x5790),
        (253, 0x1f20),
        (434, 0x9f90),
        (563, 0x5c20),
        (588, 0x30b0),
        (596, 0x12a0),
        (612, 0x12a0),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let rigs = op(&t, f, 0x14);
        let s_global = op(&t, f, 0x539);
        let (keys, base, ctl) = (SCRATCH + 0x1000, SCRATCH + 0x1400, SCRATCH + 0x1500);
        let (roll_keys, depth_keys) = (SCRATCH + 0x1600, SCRATCH + 0x1700);
        for i in 0..30u32 {
            let x = (i as f32 * 0.7).sin() * 4.0 + i as f32 * 0.9;
            set_f(&mut t, keys + 4 * i, x);
        }
        for (i, c) in [0.5f32, 1.0, -2.0, 3.5, 4.0].iter().enumerate() {
            set_f(&mut t, ctl + 4 * i as u32, *c);
        }
        for (i, c) in [10.0f32, -20.0, 0.125, 7.0].iter().enumerate() {
            set_f(&mut t, base + 4 * i as u32, *c);
        }
        for i in 0..8u32 {
            set_f(&mut t, roll_keys + 4 * i, (i as f32 * 1.3).sin() * 3.0);
            set_f(
                &mut t,
                depth_keys + 4 * i,
                2.0 + (i as f32 * 0.9).cos() * 1.5,
            );
        }
        log_calls(&mut t, 326, 4, &[1, 2], |m, a, k| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(i) ^ a[3];
                m.w32(a[0] + 4 * i, (v & 0x807f_ffff) | 0x3f00_0000);
            }
            0x0326_0000 + k
        });
        log_calls(&mut t, 229, 3, &[1, 2], |m, a, k| {
            for i in 0..4 {
                let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 0x10 * i);
                m.w32(a[0] + 4 * i, (v & 0x807f_ffff) | 0x4000_0000);
            }
            0x0229_0000 + k
        });
        log_calls(&mut t, 284, 3, &[2], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 285, 2, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 286, 2, &[], |_, a, k| a[1].rotate_left(k));
        for s in [235u32, 330, 328, 329] {
            log_calls(&mut t, s, 1, &[], move |m, a, _| {
                let v = m.r32(a[0] + 4 * (s % 16)) ^ s;
                m.w32(a[0] + 4 * (s % 16), v);
                s
            });
        }
        log_calls(&mut t, 314, 1, &[], |_, a, k| a[0] ^ k);
        t.stub_st0(314, |_, a| f64::from(a[0] & 0xff) * 0.0625 - 7.0);
        // (time, step): the blend before, at and past its end; key times
        let times = [
            (0.0f32, 0.25f32),
            (0.5, 0.25),
            (0.75, 0.25),
            (0.9, 0.25),
            (7.6, 0.5),
            (1.5, 0.3),
        ];
        // the roll's (from, to)
        // (past pi, near -pi, below -pi, between -pi and 0, small)
        let rolls = [
            (3.0f32, 4.0f32),
            (-3.5, -3.0),
            (-3.5, -3.4),
            (-1.0, -2.0),
            (0.5, 1.0),
        ];
        // the depth's (time, step)
        let depths = [
            (0.0f32, 0.25f32),
            (0.5, 0.25),
            (1.75, 0.25),
            (1.0, 0.3),
            (3.6, 0.2),
            (4.4, 0.3),
            (5.0, 0.25),
            (1.25, 0.25),
        ];
        // from 48 on the second track blends below `K1` after each of the
        // first's states (the `K1` the first one leaves)
        for k in 0..64u32 {
            t.set_u32(LOG, 0);
            t.write(rigs - 0xdc, &floats(3 * 0x148 / 4, k));
            for r in 0..3u32 {
                let q = 3 * k + r;
                let e = rigs + 0x148 * r;
                // the point tracks
                for (st, tr, qq) in [
                    (0x10u32, 0x2cu32, q),
                    (0x30u32.wrapping_neg(), 0x6c, q / 2 + 3),
                ] {
                    let tr = e - tr;
                    let late = k >= 48 && st != 0x10;
                    t.set_u32(e.wrapping_add(st), if late { 1 } else { qq % 8 });
                    t.set_u32(tr + 0x20, keys + 4 * (qq % 3));
                    t.set_u32(tr + 0x24, base);
                    t.set_u32(tr + 0x28, ctl);
                    let (tm, step) = if late {
                        (0.25, 0.25)
                    } else {
                        times[(qq / 8 % 6) as usize]
                    };
                    set_f(&mut t, tr + 0x2c, tm);
                    set_f(&mut t, tr + 0x30, step);
                    t.set_u32(tr + 0x34, if (qq / 3) % 3 == 0 { 1 } else { 4 });
                    t.set_u32(tr + 0x38, 9);
                }
                // the roll
                t.set_u32(e + 0x2c, q % 4);
                t.set_u32(e + 0x24, if (q / 4) % 3 == 0 { 1 } else { 5 });
                t.set_u32(e + 0x28, 6 + q % 2);
                t.set_u32(e + 0x14, roll_keys);
                let (from, to) = rolls[(q % 5) as usize];
                set_f(
                    &mut t,
                    e + 0x18,
                    if q % 4 == 2 { from.abs() * 0.5 } else { from },
                );
                set_f(&mut t, e + 0x1c, if q % 4 == 2 { 0.375 } else { to });
                // the screen depth
                t.set_u32(e + 0x48, if q % 5 != 0 { 1 } else { (q / 5) % 2 * 2 });
                t.set_u32(e + 0x40, if q % 11 == 0 { 1 } else { 3 });
                t.set_u32(e + 0x44, 6);
                t.set_u32(e + 0x30, depth_keys);
                let (tm, step) = depths[(q / 2 % 8) as usize];
                set_f(&mut t, e + 0x34, tm);
                set_f(&mut t, e + 0x38, step);
                // the drift and the camera
                t.set_u32(e + 0x54, (q / 3) % 3);
                t.set_u32(e + 0x58, (q / 5) % 2);
                t.set_u32(
                    e + 0x64,
                    match q % 8 {
                        3 => 0,
                        7 => 2,
                        _ => 1,
                    },
                );
                t.set_u8(e + 0x68, (q % 3) as u8);
                t.set_u8(e + 0x6a, ((q / 2) % 2) as u8);
            }
            t.set_u32(s_global, 0x5150_0000 + k);
            scrub(&mut t);
            let c = Call {
                eax: 0x1234_0000 + k,
                ecx: 0x4321_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// A cell the stubs answer from (set per case).
const ANSWER: u32 = SCRATCH + 0x5f00;

/// The reel game's phases in both DLL kinds: phase 0 counting down
/// (above, at and below 1), phase 1 with slot 652 not and done, phase 2
/// with the reels' phases answering 0 (their phase 0 and 3) and 1 (their
/// phase 2), phase 3 counting up (below, at and past 10), unknown phases.
#[test]
fn reel_game_phases() {
    for (dll, rva) in [(565u16, 0x16c0u32), (644, 0x5850), (691, 0x16c0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (s, n) in [
            (655u32, 0usize),
            (656, 0),
            (649, 0),
            (646, 1),
            (311, 1),
            (808, 4),
        ] {
            log_calls(&mut t, s, n, &[], move |_, a, k| {
                a.first().copied().unwrap_or(0) ^ (s << 16) ^ k
            });
        }
        log_calls(&mut t, 652, 1, &[], |m, _, _| m.r32(ANSWER));
        log_calls(&mut t, 312, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 651, 0, &[], |_, _, _| 1);
        t.stub_st0(651, |m, _| f64::from(m.r32(LOG)) * 0.5);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // the reels' phases and their callees (`reel_phases`' test)
        let rp = t.img.rel32_target(f + 0x7f);
        let reel = t.img.rel32_target(rp + 0x41);
        let base = t.img.word(reel + 0x1a);
        let pc = t.img.rel32_target(rp + 0x11);
        let strip = SCRATCH + 0x700;
        t.set_u32(strip, 4);
        t.set_u32(strip + 4, SCRATCH + 0x720);
        t.write(SCRATCH + 0x720, &[3, 1, 2, 3]);
        let o = SCRATCH + 0x100;
        // (phase, count, slot 652's answer, the reels' phase)
        for (k, (phase, count, ans, rph)) in [
            (0u32, 5i32, 0u32, 0u32),
            (0, 2, 0, 0),
            (0, 1, 0, 0),
            (0, 0, 0, 0),
            (0, -3, 0, 0),
            (1, 0, 0, 0),
            (1, 0, 2, 0),
            (1, 7, 1, 0),
            (2, 0, 0, 3),
            (2, 4, 0, 0),
            (2, 4, 0, 2),
            (3, 3, 0, 0),
            (3, 9, 0, 0),
            (3, 12, 0, 0),
            (4, 1, 1, 2),
            (0x8000_0000, 0, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(ANSWER, ans);
            t.set_u32(w(&t, 0x4), phase);
            t.set_u32(w(&t, 0x19), count.cast_unsigned());
            t.set_u32(w(&t, 0x47), 0x0047_0000 + k);
            t.write(o, &varied(0x80, k));
            t.set_u32(t.img.word(rp + 0x1), rph);
            t.set_u32(t.img.word(rp + 0x16), 0x2311 + k);
            let i = k % 2;
            t.set_u32(t.img.word(reel + 0x2), i);
            let r = base + 0x78 * i;
            t.write(r, &varied(0x78, k));
            for c in 0..3u32 {
                let row = r + 0x1c * c;
                t.set_u32(row, strip);
                t.set_u16(row + 6, (c + k) as u16 % 4);
            }
            t.set_u32(r + 0x68, 4);
            t.set_u32(r + 0x60, 2);
            t.set_u32(t.img.word(pc + 0x13), 1);
            t.set_u32(t.img.word(pc + 0x38), k % 3);
            scrub(&mut t);
            let c = Call {
                eax: 0x0eee_0000 + k,
                ecx: 0x0ccc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[o], Eax::Checked);
        }
    }
}

/// A free ring of `n` nodes at `node(1..=n)` (head on node 1) for the
/// pool take at `take`, its count `count`.
fn free_ring(t: &mut Twin, take: u32, node: impl Fn(u32) -> u32, n: u32, count: u16) {
    t.write(node(0), &varied(0x40, 0x77));
    for i in 1..=n {
        t.write(node(i), &varied(0x40, i));
        let next = if i == n { 1 } else { i + 1 };
        let prev = if i == 1 { n } else { i - 1 };
        t.set_u32(node(i), node(prev));
        t.set_u32(node(i) + 4, node(next));
    }
    t.set_u32(t.img.word(take + 0x17), node(1));
    t.set_u16(t.img.word(take + 0x04), count);
}

/// The object's pool node in the three DLLs: the DLL's list empty and
/// not, two takes in a row, the last free node.
#[test]
fn object_nodes() {
    for (dll, rva) in [(304u16, 0x7070u32), (568, 0x6aa0), (596, 0x4500)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let take = t.img.rel32_target(f + 0x30);
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        let (first, last) = (op(&t, f, 0x36), op(&t, f, 0x4a));
        for (k, (n, listed)) in [(3u32, false), (3, true), (1, true), (2, false)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            free_ring(&mut t, take, node, n, n as u16);
            let o = SCRATCH + 0x2000;
            t.write(o, &varied(0x140, k));
            let tail = SCRATCH + 0x3000;
            t.write(tail, &varied(0x20, k + 1));
            t.set_u32(first, if listed { SCRATCH + 0x3100 } else { 0 });
            t.set_u32(last, tail);
            for _ in 0..n.min(2) {
                let c = Call {
                    ecx: o,
                    eax: 0x0eee_0000 + k,
                    ..Call::default()
                };
                t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                    Ret::eax(s8_misc::object_node(g, img, fr))
                });
            }
        }
    }
}

/// The table flight steps (both tables) in both DLLs: the first and a
/// later step, three record kinds.
#[test]
fn table_flight_steps() {
    for (dll, rva) in [
        (304u16, 0x26c0u32),
        (304, 0x2700),
        (568, 0x20f0),
        (568, 0x2130),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let fl = t.img.rel32_target(f + 0x2a);
        log_calls(&mut t, 235, 1, &[], |m, a, k| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { k + i });
            }
            0x235
        });
        for s in [328u32, 329] {
            log_calls(&mut t, s, 3, &[], move |m, a, k| {
                let v = m.r32(a[1] + 4 * (k % 16)) ^ a[2];
                m.w32(a[0] + 4 * (k % 16), v);
                s
            });
        }
        log_calls(&mut t, 314, 1, &[], |_, a, _| a[0]);
        t.stub_st0(314, |m, _| 0.375 + f64::from(m.r32(LOG) % 5));
        log_calls(&mut t, 229, 3, &[], |m, a, _| {
            for i in 0..4u32 {
                let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
                m.w32(a[0] + 4 * i, v & 0x7fff_ffff);
            }
            0x229
        });
        for (o, x) in [
            (0xe9u32, 0.0f32),
            (0xf8, 1.0),
            (0x108, 2.5),
            (0x112, -1.0),
            (0x11f, 2.5),
            (0x13b, 0.5),
            (0x1b7, 0.25),
            (0x1f7, 0.125),
        ] {
            t.set_u32(op(&t, fl, o), x.to_bits());
        }
        let table = SCRATCH + 0x3000;
        t.set_u32(op(&t, f, 0x1b), table.wrapping_sub(op(&t, f, 0x23)));
        let o = SCRATCH + 0x2000;
        let model = SCRATCH + 0x1800;
        for (k, (kind, first, dx)) in [
            (0u8, true, 1.0f32),
            (1, true, -2.0),
            (2, false, 0.5),
            (1, false, 3.0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(model, &varied(0x20, k));
            t.set_u8(model + 0xe, kind);
            t.write(o, &floats(0x40, k));
            t.set_u32(o + 8, model);
            let e = table + 0x2c * u32::from(kind);
            t.write(table, &floats(0x30, k + 1));
            t.set_u32(e + 0x28, u32::from(!first));
            for (i, x) in (0u32..).zip([dx, 0.5, -1.0]) {
                set_f(&mut t, o + 0xb0 + 4 * i, x);
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[o], Eax::Checked);
        }
    }
}

/// The summon's start task in both DLLs: slot 225 busy, the effect
/// ready once (`*G_F` 0 and 1), the motion level below 1, at 1 and above
/// (the objects started).
#[test]
fn summon_start_tasks() {
    for (dll, rva) in [(248u16, 0x1210u32), (660, 0x3070)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        log_calls(&mut t, 225, 1, &[], |m, _, _| m.r32(ANSWER) & 1);
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], |m, _, _| m.r32(ANSWER) >> 4);
        // the objects' start (`summon_objects_started`' set-up)
        let soa_f = t.img.rel32_target(f + 0x5b);
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 262, 6, &[], |m, a, k| {
            for (i, &x) in (1u32..).zip(&a[1..4]) {
                if (SCRATCH..SCRATCH + 0x4_0000).contains(&x) {
                    m.w32(x, (0.5 * i as f32 + k as f32).to_bits());
                }
            }
            0x262
        });
        log_calls(&mut t, 333, 7, &[], |_, a, _| a[0]);
        t.stub_st0(333, |m, _| {
            [0.25, 7.5, -6.0, 1.0][(m.r32(LOG) % 4) as usize]
        });
        for s in [413u32, 215] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| a[0] ^ (s << 16));
        }
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        log_calls(&mut t, 941, 1, &[], move |_, _, _| table);
        log_calls(&mut t, 714, 5, &[], |_, a, _| 0x0714_0000 ^ a[2]);
        for call in [0x72u32, 0x123] {
            let soa = t.img.rel32_target(soa_f + call);
            t.set_u32(op(&t, soa, 0x04), SCRATCH + 0x200);
            t.set_u32(op(&t, soa, 0x5e), SCRATCH + 0x280);
            let start = t.img.rel32_target(soa + 0x84);
            t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
            let pa = t.img.rel32_target(soa + 0x99);
            t.set_u16(op(&t, pa, 0x04), 9);
        }
        let pi = t.img.rel32_target(soa_f + 0x3c);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let st = t.img.rel32_target(soa_f + 0x1b5);
        let take = t.img.rel32_target(st + 0x54);
        let w = |t: &Twin, o: u32| t.img.word(soa_f + o);
        for (o, x) in [
            (0xb9u32, 0.0f32),
            (0xc8, 1.0),
            (0xd2, 2.5),
            (0xdc, -1.0),
            (0xe9, 2.5),
            (0x16d, 0.0),
            (0x17c, 1.0),
            (0x186, 2.5),
            (0x190, -1.0),
            (0x19d, 2.5),
        ] {
            t.set_u32(w(&t, o), x.to_bits());
        }
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        // (225's answer bit 0, 227's answer << 4, *G_F)
        for (k, (ans, ready)) in [
            (1u32, 0u32),
            (0, 0),
            (0x10, 1),
            (0x20, 0),
            (0xffff_fff0, 0),
            (0x50, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(ANSWER, ans);
            t.set_u32(op(&t, f, 0x18), ready);
            t.set_u32(op(&t, f, 0x26), 0x0a00_0000 + k);
            for o in [0x61u32, 0x6b] {
                t.set_u32(op(&t, f, o), 0x0dd0_0000 + k);
            }
            t.set_u32(w(&t, 0x1c3), k % 2);
            t.set_u32(w(&t, 0x208), k % 2);
            t.set_u32(w(&t, 0x1ba), 0x01ba_0000 + k);
            t.set_u32(w(&t, 0x8c), 0x0ac7_0000 + k);
            t.set_u32(w(&t, 0x143), 0x0ac8_0000 + k);
            t.write(SCRATCH + 0x1800, &varied(0x100, k));
            for i in 1..=3u32 {
                let n = SCRATCH + 0x1800 + 0x40 * i;
                t.set_u32(n, SCRATCH + 0x1800 + 0x40 * if i == 1 { 3 } else { i - 1 });
                t.set_u32(
                    n + 4,
                    SCRATCH + 0x1800 + 0x40 * if i == 3 { 1 } else { i + 1 },
                );
            }
            t.set_u32(op(&t, take, 0x17), SCRATCH + 0x1840);
            t.set_u16(op(&t, take, 0x04), 3);
            t.set_u32(op(&t, st, 0x5a), 0);
            t.write(node(0) - 0x40, &varied(0x140, k + 1));
            t.set_u32(node(0), node(2));
            t.set_u32(node(0) + 4, node(1));
            t.set_u16(w(&t, 0x247), 2);
            t.set_u32(w(&t, 0x256), node(0));
            t.set_u32(w(&t, 0x2bd), if k.is_multiple_of(2) { node(3) } else { 0 });
            t.set_u32(w(&t, 0x2cf), node(3));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The OPU start-up with its 6 MiB buffer in both DLLs: the battle flag,
/// the object's mode bit and its data (`l5::boot_cases`' cases).
#[test]
fn opu_boots_6m() {
    for dll in [124u16, 125] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x17a0);
        l5::boot_stubs(&mut t);
        let buffer = 0x5000_0000;
        log_calls(&mut t, 803, 2, &[], move |_, _, _| buffer);
        let setup = t.at(0x1000);
        for (o, len) in [
            (0x71u32, 0x100usize),
            (0x76, 0x2000),
            (0xa8, 0x100),
            (0xad, 0x30),
        ] {
            let at = t.img.word(setup + o);
            t.write(at, &vec![0xa5; len]);
        }
        let e = SCRATCH + 0x2000;
        t.set_u32(op(&t, f, 0x13), e);
        for (k, (flag, mode, data)) in [
            (0u8, 0u8, 0u32),
            (0, 0x10, 0x00da_7a00),
            (1, 0x10, 0x00da_7a00),
            (1, 0x0f, 0x00da_7a00),
            (1, 0xff, 0),
            (1, 0x30, 0x00d0_0000),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(buffer, &varied(0x100, k));
            t.write(buffer + 0x5f_ff00, &varied(0x100, k + 1));
            let fl = t.u32(op(&t, f, 0x31));
            t.set_u8(fl + 0x10, flag);
            t.write(e, &varied(0x100, k + 2));
            t.set_u8(e + 0x40, mode);
            t.set_u32(e + 0xa8, data);
            let s = t.u32(op(&t, f, 0x37));
            t.set_u32(s + 4, 0);
            let c = Call {
                ecx: 0x77,
                ..Call::default()
            };
            t.check_entry(f, c, &[0x00ab_cdef, 0x1234], Eax::Checked);
        }
    }
}

/// The parts placed in both DLLs: the ring (`*G_F` 0: one part and
/// three) and the line (no parts, a negative count, one and four parts).
#[test]
fn parts_placed() {
    for (dll, rva) in [(515u16, 0x2c70u32), (611, 0x2c90)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let ring = t.img.rel32_target(f + 0xed);
        let tab = SCRATCH + 0x8000;
        for i in 0..0x1000u32 {
            set_f(&mut t, tab + 4 * i, ((i as f32) * 0.0015).sin());
        }
        t.set_u32(op(&t, ring, 0x67), tab);
        for (o, v) in [(0x15u32, 1.5f32), (0x26, -2.0), (0x33, 0.75)] {
            t.set_u32(op(&t, ring, o), v.to_bits());
        }
        for (o, v) in [
            (0x24u32, 1.25f32),
            (0x2c, -0.5),
            (0x34, 3.0),
            (0x3c, 0.75),
            (0x5e, 2.0),
            (0x66, 8.0),
            (0x73, 0.25),
            (0x90, -1.0),
            (0xaa, 0.5),
        ] {
            t.set_u32(op(&t, f, o), v.to_bits());
        }
        let parts = op(&t, f, 0x41);
        let base = op(&t, f, 0xe7);
        for (k, (on, n)) in [(0u32, 1i32), (0, 3), (1, 0), (1, -1), (1, 1), (1, 4)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.set_u32(op(&t, f, 0x5), on);
            t.set_u32(op(&t, f, 0x12), n.cast_unsigned());
            // the ring's base comes first: the parts lie inside it
            t.write(base, &varied(0x300, k));
            t.write(parts - 0x40, &floats(0x150, k));
            let c = Call {
                eax: 0x0eaa_0000 + k,
                ..Call::default()
            }
            .with_xmm(&[
                Xmm::f32(9.0),
                Xmm::f32(8.0),
                Xmm::f32(7.0),
                Xmm::f32(2.5 + k as f32),
            ]);
            scrub(&mut t);
            t.check_entry(
                f,
                c,
                &[0x0bad_0001, 0x1234 * k, 0x0007_0000 + k],
                Eax::Checked,
            );
        }
    }
}

/// A child object linked and set up in both DLLs: no parent, a parent
/// with no children and with one; model ids that call slot 362 and ids
/// beside them.
#[test]
fn child_objects_set_up() {
    for dll in [150u16, 603] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x6fc0);
        log_calls(&mut t, 390, 1, &[], |m, _, _| m.r32(ANSWER));
        log_calls(&mut t, 362, 4, &[], |_, a, k| a[0] ^ a[1] ^ k);
        let (p, o, r, c, m) = (
            SCRATCH + 0x100,
            SCRATCH + 0x1000,
            SCRATCH + 0x2000,
            SCRATCH + 0x2100,
            SCRATCH + 0x3000,
        );
        for (k, id) in [
            0x1071u32, 0x1072, 0x1088, 0x1089, 0x108a, 0x108c, 0x108d, 0x1070, 0x2000,
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(ANSWER, id);
            t.write(p, &varied(0x20, k));
            t.write(o, &varied(0x100, k + 1));
            t.write(r, &varied(0x40, k + 2));
            t.write(c, &varied(0x40, k + 3));
            t.write(m, &varied(0x60, k + 4));
            t.set_u32(p, o);
            t.set_u32(
                p + 4,
                match k % 3 {
                    0 => 0,
                    _ => r,
                },
            );
            t.set_u32(r + 0x10, if k % 3 == 1 { r } else { c });
            t.set_u32(p + 0x18, m);
            let call = Call {
                ecx: p,
                eax: 0x0eee_0000 + k,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s8_misc::child_object_setup(g, img, fr))
            });
        }
    }
}

/// The weapon's point in both DLLs: scales above and below 1 (the slots
/// see the scaled matrix and the point).
#[test]
fn weapon_points() {
    for (dll, rva) in [(392u16, 0x1ff0u32), (449, 0x5b10)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        log_calls(&mut t, 219, 2, &[], |_, a, k| 0x0219_0000 ^ a[0] ^ k);
        log_calls(&mut t, 236, 3, &[], |m, a, k| {
            for i in 0..16u32 {
                m.w32(a[2] + 4 * i, ((i as f32 - 7.5) * 0.25 + k as f32).to_bits());
            }
            0x236
        });
        log_calls(&mut t, 277, 1, &[], |_, a, _| a[0]);
        t.stub_st0(277, |m, _| [0.5, 3.0, 1.0 / 3.0][(m.r32(LOG) % 3) as usize]);
        log_calls(&mut t, 229, 3, &[1, 2], |m, a, k| {
            for i in 0..4u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i) ^ k);
            }
            0x229
        });
        t.set_u32(op(&t, f, 0x56), f32::to_bits(1.25));
        let o = SCRATCH + 0x1000;
        for k in 0..4u32 {
            t.set_u32(LOG, k);
            t.set_u32(op(&t, f, 0x24), 0x0a00_0000 + k);
            t.write(o, &varied(0x100, k));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[o], Eax::Checked);
        }
    }
}

/// The staged script in both DLLs: every stage it knows, stage 13 before
/// and once its count reaches 5, stages beside them and far out.
#[test]
fn staged_scripts() {
    for (dll, rva) in [(609u16, 0x9a70u32), (613, 0x9580)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        t.set_u32(op(&t, f, 0xca), f32::to_bits(0.375));
        let (tk, s, n) = (SCRATCH + 0x100, SCRATCH + 0x300, SCRATCH + 0x200);
        let part = |i: u32| SCRATCH + 0x400 + 0x80 * i;
        let ring = [SCRATCH + 0x240, SCRATCH + 0x280];
        let stages = [
            0u32,
            1,
            9,
            10,
            11,
            12,
            13,
            13,
            13,
            14,
            19,
            20,
            21,
            29,
            30,
            31,
            32,
            9999,
            10000,
            10001,
            0xffff_ffff,
        ];
        for (k, stage) in stages.into_iter().enumerate() {
            let k = k as u32;
            t.write(tk, &varied(0x10, k));
            t.set_u32(tk, n);
            t.set_u32(tk + 4, s);
            t.write(s, &varied(0x48, k + 1));
            t.set_u32(s + 0x18, stage);
            t.set_u32(s + 0x40, [4u32, 5, 7][(k % 3) as usize]);
            for i in 0..5 {
                t.write(part(i), &varied(0x80, k + i));
                set_f(&mut t, part(i) + 0x30, 0.5 * i as f32 - 1.0);
                t.set_u32(s + 0x1c + 4 * i, part(i));
            }
            t.write(n, &varied(0x10, k + 2));
            t.set_u32(n + 4, ring[0]);
            t.set_u32(n + 8, ring[1]);
            t.write(ring[0], &varied(0x10, k + 3));
            t.write(ring[1], &varied(0x10, k + 4));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[tk], Eax::Checked);
        }
    }
}

/// A node for the pool's give-back at `give` (`node`, one chained node
/// after it at `node + 0x40`, no siblings) and a free ring of one.
fn give_node(t: &mut Twin, give: u32, node: u32, k: u32) {
    let w = |t: &Twin, off: u32| t.img.word(give + off);
    let (cur, head, tail, free, count) =
        (w(t, 0x48), w(t, 0x55), w(t, 0x64), w(t, 0x74), w(t, 0x8e));
    let ring = node + 0x100;
    t.write(node, &varied(0x80, k));
    t.set_u32(node, 0);
    t.set_u32(node + 4, 0);
    t.set_u32(node + 8, node + 0x40);
    t.set_u32(node + 0x48, node + 0x40);
    t.set_u32(node + 0x4c, 0);
    t.write(ring, &varied(0x1c, k + 1));
    t.set_u32(ring, ring);
    t.set_u32(ring + 4, ring);
    t.set_u32(free, ring);
    t.set_u16(count, 0x10 + k as u16);
    t.set_u32(cur, node);
    t.set_u32(head, node + 0x200);
    t.set_u32(tail, node);
}

/// The two summon frames with the camera rigs: the first script running
/// (each rig idle, the motion steps done or waiting) and ended (with a
/// held node and without), in both DLLs each.
#[test]
fn summon_frames_with_rigs() {
    for (dll, rva, fade) in [
        (563u16, 0x13e0u32, false),
        (612, 0x72d0, false),
        (168, 0x14e0, true),
        (599, 0x5ed0, true),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (s, n) in [(284u32, 3usize), (285, 2), (286, 2), (332, 1), (411, 1)] {
            log_calls(&mut t, s, n, &[], move |_, a, k| {
                a.first().copied().unwrap_or(0) ^ (s << 16) ^ k
            });
        }
        t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
        log_calls(&mut t, 1000, 2, &[], |m, a, _| {
            // a command: the script ends
            m.w32(a[0] + 0x54, 0);
            a[1] + 2
        });
        let e = SCRATCH + 0x8000;
        t.set_u32(op(&t, f, 0x7), e);
        let ctx = [op(&t, f, 0x2e), op(&t, f, 0x3d)];
        let runs = [t.img.rel32_target(f + 0x38), t.img.rel32_target(f + 0x44)];
        let rigs = t.img.rel32_target(f + if fade { 0xd3 } else { 0x85 });
        let give = t.img.rel32_target(f + if fade { 0xed } else { 0xb3 });
        for (k, (done, held, mstate)) in [
            (0u8, false, 1u32),
            (4, false, 1),
            (1, true, 0),
            (0, true, 0),
            (0, false, 2),
            (2, true, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(e, &floats(0x1000, k));
            for (i, (&c, &run)) in ctx.iter().zip(&runs).enumerate() {
                // the context's 0x58 bytes (the DLL's next global follows)
                t.write(c, &varied(0x58, k + i as u32));
                t.set_u32(c, SCRATCH + 0x5d00);
                // the second runs (and ends) every other case
                t.set_u8(c + 0x56, if i == 0 { done } else { 4 - 3 * (k % 2) as u8 });
                t.set_u16(c + 0x54, 3);
                t.set_u32(c + 0x24, 0);
                t.set_u32(c + 0x28, 0);
                let table = t.img.word(run + 0x17);
                t.set_u32(table, magic::TRAPS + 16 * 1000);
            }
            t.set_u16(SCRATCH + 0x5d00, 0);
            // the rigs idle (states 0, no camera)
            let r = op(&t, rigs, 0x14);
            t.write(r - 0xdc, &floats(3 * 0x148 / 4, k));
            for i in 0..3 {
                let x = r + 0x148 * i;
                for o in [0x10u32, 0x2c, 0x48, 0x64] {
                    t.set_u32(x + o, 0);
                }
                t.set_u32(x - 0x30, 0);
                t.set_u8(x + 0x68, 0);
            }
            t.set_u32(r + 0x10, mstate % 2 * 7);
            // the first rig sets the camera (the rigs' own operands)
            t.set_u32(r + 0x64, 1);
            t.set_u8(r + 0x6a, 0);
            t.set_u32(op(&t, f, 0x26), 0x0026_0000 + k);
            if fade {
                for o in [0xabu32, 0xbb] {
                    let s = op(&t, f, o);
                    t.write(s, &varied(0x40, k));
                    t.set_u32(s + 0x14, 1);
                }
                t.set_u32(e + 0xe70 + 0x34, mstate);
                t.set_u32(e + 0xe70 + 0x30, 4);
                // the effect's end: no held node, the parts' models 0 or 1
                let end = t.img.rel32_target(f + 0xe2);
                t.set_u32(op(&t, end, 0x90), 0);
                // its block is `E` (the same global)
                let parts = e;
                for i in 0..4u32 {
                    let part = parts + 0xea8 + 0x8a8 * i;
                    t.set_u32(part, 0x0123_0000 + i);
                    t.set_u32(part + 0x824, (i + k) % 2);
                }
                t.set_u32(op(&t, end, 0x146), SCRATCH + 0xbf00);
                t.set_u32(op(&t, end, 0x162), SCRATCH + 0xbf00);
                for i in 0..3u32 {
                    t.set_u32(e + 0xe70 + 0x10 + 4 * i, 0x40 + 9 * i + k);
                    t.set_u32(e + 0xe70 + 0x20 + 4 * i, 0x80 - 5 * i);
                }
            } else {
                // the motion step: done, or a wait
                let ms = t.img.rel32_target(f + 0x7d);
                t.set_u32(op(&t, ms, 0x02), u32::from(mstate == 1));
                t.set_u16(op(&t, ms, 0x11), 5);
            }
            let node = SCRATCH + 0x9000;
            give_node(&mut t, give, node, k);
            let held = if held { SCRATCH + 0xa000 } else { 0 };
            if held != 0 {
                give_node(&mut t, give, held, k + 1);
                // the node given back last goes to the free ring first
                give_node(&mut t, give, node, k);
            }
            if !fade {
                t.set_u32(op(&t, f, 0x8b), held);
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[node], Eax::Checked);
        }
    }
}

/// Stubs and globals for [`object_fan_body`]'s callees at body offset `b`
/// of `f`: the object start (`summon_objects_started`' set-up), the slots
/// they call and a free ring of 6 nodes for the takes.
fn object_fan_setup(t: &mut Twin, f: u32, b: u32) {
    log_calls(t, 714, 5, &[], |_, a, _| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    log_calls(t, 941, 1, &[], move |_, _, _| table);
    for (s, n) in [(332u32, 1usize), (713, 1)] {
        log_calls(t, s, n, &[], move |_, a, k| a[0] ^ (s << 16) ^ k);
    }
    let soa = t.img.rel32_target(f + b + 0x20);
    t.set_u32(op(t, soa, 0x04), SCRATCH + 0x200);
    t.set_u32(op(t, soa, 0x5e), SCRATCH + 0x280);
    let start = t.img.rel32_target(soa + 0x84);
    t.set_u32(op(t, start, 0x04), SCRATCH + 0x7f8);
    let pa = t.img.rel32_target(soa + 0x99);
    t.set_u16(op(t, pa, 0x04), 9);
}

/// The objects' start with their fan and fades in both DLLs (both
/// copies): the DLL's task list empty and not.
#[test]
fn object_fans() {
    for (dll, rva, b) in [
        (511u16, 0x4810u32, 0u32),
        (511, 0x4660, 0),
        (602, 0x88f0, 0),
        (511, 0x43e0, 0xea),
        (602, 0x84c0, 0xea),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        object_fan_setup(&mut t, f, b);
        let e = SCRATCH + 0x8000;
        t.set_u32(op(&t, f, if b == 0 { 0xc } else { 0x5 }), e);
        let take = t.img.rel32_target(f + b + 0xd9);
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        let (first, last) = (op(&t, f, b + 0xdf), op(&t, f, b + 0xf2));
        // the four objects' objs_list (row 204): which objects it holds
        let objs_list = op(&t, f, 0xa);
        let lnode = |i: u32| SCRATCH + 0x5c40 + 0x20 * i;
        for (k, (listed, held)) in [
            (false, 0u32),
            (true, 0b1001),
            (true, 0b0110),
            (false, 0b1111),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(e, &varied(0x2400, k));
            t.write(e + 0x19b0, &floats(0x40, k));
            t.write(e + 0x2258, &varied(4, k + 3));
            free_ring(&mut t, take, node, 6, 6);
            t.write(SCRATCH + 0x3000, &varied(0x20, k + 1));
            t.set_u32(first, if listed { SCRATCH + 0x3100 } else { 0 });
            t.set_u32(last, SCRATCH + 0x3000);
            if b != 0 {
                let objs: Vec<u32> = [0x10u32, 0x49, 0x82, 0xbb]
                    .iter()
                    .map(|&d| e + op(&t, f, d))
                    .collect();
                let x = SCRATCH + 0x5b00;
                t.set_u32(op(&t, f, 0x25), x);
                t.write(objs_list, &varied(0x10, k));
                let mut prev = 0;
                let mut n = 0;
                for (i, &o) in objs.iter().enumerate() {
                    if held >> i & 1 == 0 {
                        continue;
                    }
                    let nd = lnode(n);
                    t.write(nd, &varied(0x20, k + n));
                    t.set_u32(nd, prev);
                    t.set_u32(nd + 4, 0);
                    t.set_u32(nd + 8, o);
                    if prev == 0 {
                        t.set_u32(objs_list, nd);
                    } else {
                        t.set_u32(prev + 4, nd);
                    }
                    prev = nd;
                    n += 1;
                }
                if n == 0 {
                    t.set_u32(objs_list, 0);
                }
                t.set_u32(objs_list + 4, prev);
                t.set_u32(objs_list + 8, lnode(0));
                t.set_u32(objs_list + 0xc, n);
            }
            scrub(&mut t);
            let c = Call {
                eax: 0x0eee_0000 + k,
                ecx: 0x0ccc_0000 + k,
                edx: 0x0ddd_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// The emitters' sounds in the four DLLs: records without a bank, with
/// the scale flag and without (the turn), slot 773 answering both ways,
/// the callbacks on and off (`**G_F`); the slots see the matrix, the
/// record `S` and the lock.
#[test]
fn emitter_sounds() {
    for (dll, rva) in [
        (515u16, 0x3a20u32),
        (611, 0x3a50),
        (667, 0x38d0),
        (668, 0x38f0),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let lock_cell = SCRATCH + 0x5f80;
        let flag_cell = SCRATCH + 0x5f84;
        let d_cell = SCRATCH + 0x5f88;
        t.set_u32(op(&t, f, 0x1d4), lock_cell);
        t.set_u32(op(&t, f, 0x2e1), flag_cell);
        t.set_u32(op(&t, f, 0x1b8), d_cell);
        log_calls(&mut t, 235, 1, &[], |m, a, k| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { k + i });
            }
            0x235
        });
        log_calls(&mut t, 244, 2, &[0], |m, a, k| {
            m.w32(a[0] + 4, a[1] ^ k);
            0x244
        });
        log_calls(&mut t, 233, 3, &[0], |m, a, _| {
            m.w32(a[0] + 8, m.r32(a[1]) ^ a[2]);
            0x233
        });
        log_calls(&mut t, 299, 3, &[0], |m, a, _| {
            m.w32(a[0] + 0xc, a[1]);
            0x299
        });
        log_calls(&mut t, 230, 2, &[1], |m, a, k| {
            // the whole matrix kept (compared), the last row into `L`
            for i in 0..16u32 {
                m.w32(
                    SCRATCH + 0x6800 + 0x40 * (k % 8) + 4 * i,
                    m.r32(a[1] + 4 * i),
                );
            }
            for i in 0..4u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 0x30 + 4 * i) ^ k);
            }
            0x230
        });
        log_calls(&mut t, 772, 2, &[0], |_, a, k| a[0] ^ k);
        t.stub_edx(772, |_, a| a[0].rotate_left(5));
        log_calls(&mut t, 773, 2, &[], move |m, a, _| {
            m.w32(SCRATCH + 0x5f90, m.r32(lock_cell));
            (a[0] >> 2) & 1
        });
        for (s, n, p) in [
            (875u32, 4usize, &[3usize][..]),
            (775, 2, &[][..]),
            (873, 5, &[3][..]),
            (776, 1, &[][..]),
        ] {
            log_calls(&mut t, s, n, p, move |m, a, k| {
                m.w32(SCRATCH + 0x5f94 + 4 * (k % 4), m.r32(lock_cell));
                if n >= 4 {
                    // the sound record `S` whole
                    for i in 0..8u32 {
                        m.w32(
                            SCRATCH + 0x6c00 + 0x20 * (k % 8) + 4 * i,
                            m.r32(a[3] + 4 * i),
                        );
                    }
                }
                a[0] ^ (s << 16) ^ k
            });
        }
        t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
        log_calls(&mut t, 1001, 2, &[0], |_, a, k| a[1] ^ k);
        let recs = op(&t, f, 0x7a);
        let bank = SCRATCH + 0x7000;
        t.write(bank, &varied(0x400, 3));
        for i in 0..16u32 {
            t.set_u16(bank + 8 * i, (0x40 + 0x18 * i) as u16);
        }
        for i in 0..32u32 {
            t.set_u32(bank + 0x40 + 0x18 * i + 4, i % 7);
        }
        for k in 0..3u32 {
            t.set_u32(LOG, 0);
            t.set_u32(lock_cell, 0x1100 + k);
            t.set_u32(flag_cell, u32::from(k == 1));
            t.set_u32(d_cell, 0xd000 + k);
            t.set_u32(op(&t, f, 0x14f), 0x0c00_0000 + k);
            t.write(recs - 0x34, &floats(0x100 * 0x4c / 4, k));
            for r in 0..0x100u32 {
                let e = recs + 0x4c * r;
                let on = r % 37 == k || r == 0xff;
                t.set_u32(e - 0xc, if on { bank } else { 0 });
                t.set_u32(e - 8, (r + k) % 2);
                t.set_u16(e + 0xc, (0x1000 * r + k) as u16);
                t.set_u16(e + 0x16, ((r + k) % 10) as u16);
                t.set_u32(e - 4, magic::TRAPS + 16 * 1001);
            }
            scrub(&mut t);
            let c = Call {
                eax: 0x0eee_0000 + k,
                ecx: 0x0ccc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// The model effect placed in both DLLs: the lookup failing and found
/// (key bits across their fields), `*G_Q` on and off, `a4` below, inside
/// and above the clamp, `a4`/`a5` with high bits the shift drops.
#[test]
fn model_effects_placed() {
    for (dll, rva) in [(162u16, 0x1d20u32), (586, 0x1cc0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let lock = SCRATCH + 0x5f80;
        t.set_u32(op(&t, f, 0x60), lock);
        let key = SCRATCH + 0x5e00;
        let cam = SCRATCH + 0x5e40;
        log_calls(&mut t, 719, 3, &[], move |_, _, _| key);
        log_calls(&mut t, 885, 7, &[], move |m, a, k| {
            m.w32(SCRATCH + 0x5f84, m.r32(lock));
            for i in 0..8u32 {
                m.w32(a[6] + 4 * i, a[0] ^ (i << 8) ^ k);
            }
            m.r32(ANSWER)
        });
        log_calls(&mut t, 545, 0, &[], move |_, _, _| cam);
        log_calls(&mut t, 230, 2, &[], |m, a, _| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i));
            }
            0x230
        });
        log_calls(&mut t, 231, 3, &[1], |m, a, _| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i) ^ a[2] ^ i);
            }
            0x231
        });
        for (s, n) in [(922u32, 2usize), (931, 2), (776, 1), (834, 1)] {
            log_calls(&mut t, s, n, &[], move |_, a, k| a[0] ^ (s << 16) ^ k);
        }
        log_calls(&mut t, 912, 12, &[1, 7, 9], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 833, 3, &[], |_, a, k| a[2] ^ k);
        t.write(cam, &floats(0x10, 7));
        let o = SCRATCH + 0x1000;
        let m = SCRATCH + 0x2000;
        // (found, *G_Q, a4, a5, key low, key high)
        for (k, (found, q, a4, a5, lo, hi)) in [
            (0u32, 0u32, 0u32, 0u32, 0x1234_5678u32, 0x9abc_def0u32),
            (1, 0, 0x40, 0x80, 0xffff_ffff, 0xffff_ffff),
            (1, 1, 0xffff_ffc0, 0x0100_0040, 0x4321_0fed, 3),
            (2, 1, 0x7fff_ffff, 0xfe00_0080, 0x0bad_f00d, 0),
            (1, 0, 0x100, 0x7f, 0x0010_0000, 0x8000_0001),
            (1, 1, 0x0180_0020, 0xffff_ffff, 0xc000_0000, 0x0000_0001),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(ANSWER, found);
            t.set_u32(op(&t, f, 0x21d), q);
            t.set_u32(lock, 0x1100 + k);
            t.set_u32(key, lo);
            t.set_u32(key + 4, hi);
            t.write(o, &varied(0x840, k));
            t.set_u32(o + 0x830, m);
            scrub(&mut t);
            let c = Call {
                ecx: o,
                eax: 0x0eee_0000 + k,
                ..Call::default()
            };
            let args = [
                0x0bad_0000 + k,
                0x0a01_0000 + k,
                0x0a02_0000 + k,
                0x0a03_0000 + k,
                a4,
                a5,
            ];
            t.check_entry(f, c, &args, Eax::Checked);
        }
    }
}

/// The summon frame with its colours in both DLLs: off (`*G_A`, `*G_B`),
/// each colour stage with levels inside and outside their ranges, the
/// sound record on and off, `**G_V` set and not.
#[test]
fn summon_frames_colours() {
    for (dll, rva) in [(515u16, 0x8cb0u32), (667, 0x8910)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (s, n) in [
            (723u32, 1usize),
            (724, 1),
            (315, 1),
            (275, 1),
            (712, 8),
            (258, 1),
        ] {
            log_calls(&mut t, s, n, &[], move |_, a, k| a[0] ^ (s << 16) ^ k);
        }
        log_calls(&mut t, 776, 1, &[], |_, a, _| a[0] << 4);
        log_calls(&mut t, 915, 5, &[], |_, a, k| a[0] ^ a[3] ^ k);
        log_calls(&mut t, 235, 1, &[], |_, a, _| a[0]);
        // the emitters: none with a bank
        let em = t.img.rel32_target(f + 0x2a3);
        let recs = op(&t, em, 0x7a);
        t.write(recs - 0x34, &vec![0; 0x100 * 0x4c]);
        t.set_u32(op(&t, em, 0x2e1), SCRATCH + 0x5f70);
        // the sound record (`model_sound_records`' set-up), slot 773 open
        let sr = t.img.rel32_target(f + 0x74);
        t.set_u32(op(&t, sr, 0x167), SCRATCH + 0x2f0);
        // its `**G_A` is the block this frame stores first (the same global)
        let block = op(&t, f, 0x53);
        t.set_u32(block + 0x20, SCRATCH + 0x1000);
        t.set_u32(SCRATCH + 0x1000 + 0x8e0 + 0x1c, SCRATCH + 0x800);
        t.set_u32(op(&t, sr, 0xd4), SCRATCH + 0x780);
        log_calls(&mut t, 773, 2, &[], |_, _, _| 1);
        let gv = SCRATCH + 0x5f74;
        t.set_u32(op(&t, f, 0x2a8), gv);
        let gy = SCRATCH + 0x5f78;
        t.set_u32(op(&t, f, 0x2c2), gy);
        let ge = SCRATCH + 0x5f7c;
        t.set_u32(op(&t, f, 0x4a), ge);
        let gg = SCRATCH + 0x5f60;
        t.set_u32(op(&t, f, 0x79), gg);
        // (on, sound, stage, level word, other level word, **G_V)
        let cases: [(bool, u16, u32, u32, u32, u32); 24] = [
            (false, 0, 1, 0, 0, 0),
            (true, 1, 1, 0x0004_0000, 0x0010_0000, 0),
            (true, 0, 1, 0x0004_0000, 0x0004_0000, 1),
            (true, 0, 1, 0x0006_0000, 0x0002_0000, 0),
            (true, 0, 3, 10, 0, 0),
            (true, 0, 3, 0x28, 0, 1),
            (true, 0, 3, 0xfff0, 0, 0),
            (true, 0, 16, 0x15, 0, 0),
            (true, 0, 16, 0x14, 0, 0),
            (true, 0, 16, 0x120, 0, 0),
            (true, 0, 21, 0x30, 0, 0),
            (true, 0, 21, 0x40, 0, 0),
            (true, 0, 21, 0xa0, 0, 0),
            (true, 0, 21, 0xcd, 0, 0),
            (true, 0, 21, 0xce, 0, 0),
            (true, 0, 23, 0x1d, 0, 0),
            (true, 0, 23, 0x1e, 0, 0),
            (true, 0, 23, 0xff00, 0, 0),
            (true, 0, 2, 0, 0, 0),
            (true, 1, 24, 0, 0, 1),
            // the boundaries
            (true, 0, 1, 0x0005_1000, 0x0001_0000, 0),
            (true, 0, 16, 0x112, 0, 0),
            (true, 0, 21, 0x37, 0, 0),
            (true, 0, 21, 0x91, 0, 0),
        ];
        for (k, (on, sound, stage, lv, lv2, v)) in cases.into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(op(&t, f, 0x8), u32::from(on || k.is_multiple_of(2)));
            t.set_u32(op(&t, f, 0x15), u32::from(on));
            t.set_u16(op(&t, f, 0x5a), sound);
            t.set_u16(op(&t, f, 0x130), sound ^ (k % 2) as u16);
            t.set_u32(op(&t, f, 0x14a), stage);
            t.set_u32(op(&t, f, 0x167), lv);
            t.set_u32(op(&t, f, 0x16d), lv2);
            for o in [0x19eu32, 0x1ce, 0x21e, 0x270] {
                t.set_u16(op(&t, f, o), lv as u16);
            }
            t.set_u32(gv, v);
            t.set_u32(gy, 0x0a00_0000 + k);
            t.set_u32(op(&t, f, 0x2b2), 0x0b00_0000 + k);
            t.set_u32(op(&t, f, 0x2b8), k);
            t.set_u32(SCRATCH + 0x2f0, 0x1100 + k);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The second summon frame with colours in both DLLs: every stage with
/// levels inside and outside their ranges.
#[test]
fn summon_frames_colours_b() {
    for (dll, rva) in [(611u16, 0x8bf0u32), (668, 0x87f0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (s, n) in [
            (723u32, 1usize),
            (724, 1),
            (315, 1),
            (275, 1),
            (712, 8),
            (258, 1),
        ] {
            log_calls(&mut t, s, n, &[], move |_, a, k| a[0] ^ (s << 16) ^ k);
        }
        log_calls(&mut t, 776, 1, &[], |_, a, _| a[0] << 4);
        log_calls(&mut t, 915, 5, &[], |_, a, k| a[0] ^ a[3] ^ k);
        log_calls(&mut t, 235, 1, &[], |_, a, _| a[0]);
        let em = t.img.rel32_target(f + 0x2e1);
        let recs = op(&t, em, 0x7a);
        t.write(recs - 0x34, &vec![0; 0x100 * 0x4c]);
        t.set_u32(op(&t, em, 0x2e1), SCRATCH + 0x5f70);
        let sr = t.img.rel32_target(f + 0x74);
        t.set_u32(op(&t, sr, 0x167), SCRATCH + 0x2f0);
        let block = op(&t, f, 0x53);
        t.set_u32(block + 0x20, SCRATCH + 0x1000);
        t.set_u32(SCRATCH + 0x1000 + 0x8e0 + 0x1c, SCRATCH + 0x800);
        t.set_u32(op(&t, sr, 0xd4), SCRATCH + 0x780);
        log_calls(&mut t, 773, 2, &[], |_, _, _| 0);
        log_calls(&mut t, 777, 1, &[], |_, a, _| a[0] ^ 0xfff0);
        let gv = SCRATCH + 0x5f74;
        t.set_u32(op(&t, f, 0x2e6), gv);
        let gy = SCRATCH + 0x5f78;
        t.set_u32(op(&t, f, 0x300), gy);
        // (sound, stage, level word, other level word, **G_V)
        let cases: [(u16, u32, u32, u32, u32); 25] = [
            (1, 0, 0x10, 0, 0),
            (0, 0, 0x15, 0, 1),
            (0, 0, 0xfffe, 0, 0),
            (0, 1, 0x0004_0000, 0x0010_0000, 0),
            (0, 1, 0x0006_0000, 0x0002_0000, 0),
            (0, 1, 0x0004_0000, 0x0003_f000, 0),
            (0, 3, 10, 0, 0),
            (0, 3, 0x28, 0, 0),
            (0, 16, 0x15, 0, 0),
            (0, 16, 0x120, 0, 0),
            (0, 21, 0x10, 0, 0),
            (0, 21, 0x60, 0, 0),
            (0, 21, 0xc0, 0, 0),
            (0, 21, 0xeb, 0, 0),
            (0, 21, 0xec, 0, 0),
            (0, 23, 0x1d, 0, 0),
            (0, 23, 0x1e, 0, 0),
            (0, 2, 0, 0, 0),
            (1, 0x18, 0, 0, 1),
            // the boundaries
            (0, 0, 0, 0, 0),
            (0, 1, 0x0005_1000, 0x0001_0000, 0),
            (0, 16, 0x14, 0, 0),
            (0, 16, 0x112, 0, 0),
            (0, 21, 0x55, 0, 0),
            (0, 21, 0xaf, 0, 0),
        ];
        for (k, (sound, stage, lv, lv2, v)) in cases.into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(op(&t, f, 0x8), 1);
            t.set_u32(op(&t, f, 0x15), 1);
            t.set_u16(op(&t, f, 0x5a), sound);
            t.set_u16(op(&t, f, 0x130), sound ^ (k % 2) as u16);
            t.set_u32(op(&t, f, 0x14a), stage);
            t.set_u32(op(&t, f, 0x191), lv);
            t.set_u32(op(&t, f, 0x197), lv2);
            for o in [0x167u32, 0x1b2, 0x1e2, 0x233, 0x2ae] {
                t.set_u16(op(&t, f, o), lv as u16);
            }
            t.set_u32(gv, v);
            t.set_u32(gy, 0x0a00_0000 + k);
            t.set_u32(op(&t, f, 0x2f0), 0x0b00_0000 + k);
            t.set_u32(op(&t, f, 0x2f6), k);
            t.set_u32(SCRATCH + 0x2f0, 0x1100 + k);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The bone node's handler in both DLLs, directly and through its thunk:
/// not passed; passed with an empty ring and with two nodes; without the
/// parent bone and with it.
#[test]
fn bone_nodes() {
    for dll in [150u16, 603] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x35a0);
        for s in [1000u32, 1001] {
            t.set_u32(magic::TABLE + 4 * s, magic::TRAPS + 16 * s);
        }
        // the handlers log their cursor and its node and pass byte
        log_calls(&mut t, 1000, 1, &[0], |m, a, k| {
            m.w32(SCRATCH + 0x6e00 + 4 * (k % 32), m.r32(a[0] + 4) & 0xff00);
            0x1000_0000 + k
        });
        log_calls(&mut t, 1001, 2, &[0, 1], |_, a, k| a[1] ^ k);
        let table = op(&t, f, 0xed);
        for i in 0..4 {
            t.set_u32(table + 4 * i, magic::TRAPS + 16 * 1001);
        }
        log_calls(&mut t, 233, 3, &[0, 1], |m, a, k| {
            for i in 0..4u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i) ^ a[2] ^ k);
            }
            0x233
        });
        for (s, n) in [(325u32, 2usize), (230, 2), (231, 3), (229, 3), (668, 3)] {
            log_calls(&mut t, s, n, &[0, 1], move |m, a, k| {
                m.w32(a[0], a[1] ^ (s << 16) ^ k);
                s
            });
        }
        t.set_u32(op(&t, f, 0x109), f32::to_bits(0.5));
        t.set_u32(op(&t, f, 0x1fa), f32::to_bits(1.5));
        t.set_u32(op(&t, f, 0x2b1), f32::to_bits(1.5));
        let (s, r, p) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x2000);
        let vt = SCRATCH + 0x3000;
        t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1000);
        let linked = [SCRATCH + 0x3100, SCRATCH + 0x3200, SCRATCH + 0x3300];
        let ring = [SCRATCH + 0x3400, SCRATCH + 0x3440];
        for (k, (pass, byte, nodes, parent)) in [
            (3u8, 3u8, 0usize, false),
            (4, 3, 0, false),
            (5, 3, 2, false),
            (6, 1, 0, true),
            (7, 2, 2, true),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(s, &varied(0x10, k));
            t.set_u32(s, r);
            t.set_u8(s + 5, pass);
            t.write(r, &floats(0x150 / 4, k));
            t.set_u8(r + 0x1a, byte);
            t.set_u8(r + 0x148, (k % 4) as u8);
            let mut prev = r;
            for (i, &nd) in ring.iter().enumerate().take(nodes) {
                t.write(nd, &varied(0x40, k + i as u32));
                t.set_u32(nd, vt);
                if i == 0 {
                    t.set_u32(r + 0x10, nd);
                } else {
                    t.set_u32(prev + 8, nd);
                }
                prev = nd;
            }
            if nodes == 0 {
                t.set_u32(r + 0x10, r);
            } else {
                t.set_u32(prev + 8, r);
            }
            for (&l, o) in linked.iter().zip([0x13cu32, 0x144, 0x140]) {
                t.write(l, &floats(0x30 / 4, k + o));
                t.set_u32(l, vt);
                t.set_u32(r + o, l);
            }
            if parent {
                t.write(p, &floats(0x150 / 4, k + 9));
                t.set_u32(p, vt);
                t.set_u32(r + 0x128, p);
            } else {
                t.set_u32(r + 0x128, 0);
            }
            scrub(&mut t);
            let c = Call {
                ecx: s,
                eax: 0x1234_5600 + k,
                ..Call::default()
            };
            let base = t.save();
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s8_misc::bone_node(g, img, fr))
            });
            t.restore(&base);
            t.set_u32(LOG, 0);
            t.check_entry(t.at(0x3900), Call::default(), &[s], Eax::Checked);
        }
    }
}
