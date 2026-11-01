//! The shared-7 list's ports (`ffx_magic::s7_misc`) against the DLLs
//! they were ported from and the other DLLs that carry them. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::{Ret, Xmm, s6_misc, s7_misc};
use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};
use sa::rec;

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

/// The entry's stack pointer (at the return address).
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// The frame area below the entry's stack pointer filled with a pattern
/// before a run (the stores of the run before, or zeros, would hide a
/// port's dropped store from the stubs).
fn scrub(t: &mut Twin) {
    t.write(ENTRY_ESP - 0x3000, &varied(0x3000, 0x5b));
}

/// The trap of table word `s` (a callback the harness answers as slot `s`).
fn trap(s: u32) -> u32 {
    magic::TRAPS + 16 * s
}

/// Logs each call of `slot` into a row of its own (16 rows of 0x40
/// bytes at `log`): the call's number, its first `n` words and the word
/// at `watch` (a flag the caller sets around the call). Answers `ret(m,
/// args, k)`.
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

/// Row 449: the model's sound handles registered, in both DLLs: no, one
/// and three more words, addresses whose sign flips in the loop (the
/// `cdq`), the lock word seen by each registration.
#[test]
fn handles_registered() {
    for (dll, rva) in [(150u16, 0x4110u32), (603, 0x4110)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let lock_global = op(&t, f, 0x21);
        let lock = SCRATCH + 0x2f0;
        t.set_u32(lock_global, lock);
        log_calls(&mut t, 772, log, 2, Some(lock_global), |_, a, k| {
            a[0] ^ a[1] ^ (k << 20)
        });
        t.stub_edx(772, |_, a| a[1].rotate_left(8));
        log_calls(&mut t, 779, log, 2, Some(lock_global), |_, a, k| {
            0x0779_0000 ^ a[0] ^ k
        });
        let n_global = op(&t, f, 0x81);
        let cell = SCRATCH + 0x300;
        for (k, (n, m)) in [
            (0i32, 0x4000_1000u32),
            (1, 0x4000_2000),
            (3, 0x7fff_f7dc),
            (-1, 0x1234_5678),
            (2, 0xffff_f000),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(n_global, n.cast_unsigned());
            t.set_u32(lock, 0x0bad_0000 + k);
            t.set_u32(cell, m);
            let c = Call {
                ecx: cell,
                eax: 0xeee0 + k,
                ..Call::default()
            };
            scrub(&mut t);
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::handles_registered(g, img, fr))
            });
        }
    }
}

/// Row 344: a part's count and fade in the three DLLs: counts far below,
/// in the last five and at or past the limit, indices 0..2, enough runs
/// for `rand()`'s 1-in-16 node start (slots 434 and 942 logged).
#[test]
fn count_fades() {
    for (dll, rva) in [(384u16, 0x1d30u32), (421, 0x1d20), (535, 0x1d00)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let node = SCRATCH + 0x800;
        log_calls(&mut t, 434, log, 3, None, move |_, _, _| node);
        log_calls(&mut t, 942, log, 4, None, |m, a, k| m.r32(a[0]) ^ k);
        t.set_u32(op(&t, f, 0x8), 0.75f32.to_bits());
        t.set_u32(op(&t, f, 0x10), 1.25f32.to_bits());
        t.set_u32(op(&t, f, 0xac), 6.0f32.to_bits());
        t.set_u32(op(&t, f, 0x50), 0x0050_1234);
        let n_global = op(&t, f, 0x79);
        let o = SCRATCH + 0x400;
        for k in 0..80u32 {
            let n = 20 + (k % 3) as i32;
            let c = n - 8 + (k % 10) as i32;
            t.set_u32(log, 0);
            t.set_u32(n_global, n.cast_unsigned());
            t.write(o, &varied(0x200, k));
            let i = k % 3;
            t.set_u32(o + 0xac, i);
            t.set_u32(o + 0x2c + 0x10 * i, c.cast_unsigned());
            t.write(node, &varied(0x200, k + 1));
            t.set_u32(node + 0xac, (k + 1) % 3);
            let call = Call {
                ecx: o,
                eax: 0x1100 + k,
                ..Call::default()
            };
            scrub(&mut t);
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::count_fade(g, img, fr))
            });
        }
    }
}

/// Row 469: a ring pass and a key frame in both DLLs: the ring already
/// passed, empty and with two nodes (their handler logging the cursor);
/// no motion, no key table (the junk local 0), keys before, at and past
/// the table's count.
#[test]
fn ring_key_frames() {
    for (dll, rva) in [(578u16, 0x66a0u32), (649, 0x6740)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        t.set_u32(magic::TABLE + 4 * 1000, trap(1000));
        log_calls(&mut t, 1000, log, 1, None, |m, a, k| {
            // the handler sees the cursor's node
            let n = m.r32(a[0]);
            m.w32(n + 0x14, k);
            0x1000_0000 + (m.r32(n + 0xc) << 8) + k
        });
        let table = SCRATCH + 0x2000;
        log_calls(&mut t, 941, log, 1, None, move |_, a, _| {
            if a[0] & 1 == 0 { table } else { 0 }
        });
        let vt = SCRATCH + 0x700;
        t.set_u32(vt + 0x1c, trap(1000));
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x400;
        let m = SCRATCH + 0x500;
        let motion = SCRATCH + 0x580;
        let nodes = [SCRATCH + 0x600, SCRATCH + 0x640];
        // (pass, ring byte, nodes, motion, table word, time, count)
        for (k, (pass, byte, n, moving, x, time, count)) in [
            (3u8, 3u8, 2usize, true, 0u32, 1.0f32, 4.0f32),
            (4, 3, 0, false, 0, 1.0, 4.0),
            (5, 3, 2, true, 2, 2.5, 4.0),
            (6, 3, 1, true, 1, 2.0, 4.0),
            (7, 0, 0, true, 4, 4.0, 4.0),
            (8, 1, 2, true, 6, 9.75, 4.5),
            (9, 1, 0, true, 8, -1.0, 3.0),
            (10, 2, 1, true, 0, 1.0, 4.0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(s, &varied(0x10, k));
            t.set_u32(s, r);
            t.set_u8(s + 5, pass);
            t.write(r, &varied(0x40, k + 1));
            t.set_u8(r + 0x1a, byte);
            let mut prev = r;
            for (i, &nd) in nodes.iter().enumerate().take(n) {
                t.write(nd, &varied(0x40, k + 2 + i as u32));
                t.set_u32(nd, vt);
                if i == 0 {
                    t.set_u32(r + 0x10, nd);
                } else {
                    t.set_u32(prev + 8, nd);
                }
                prev = nd;
            }
            if n == 0 {
                t.set_u32(r + 0x10, r);
            } else {
                t.set_u32(prev + 8, r);
            }
            t.set_u32(r + 0x2c, if moving { m } else { 0 });
            t.set_u32(r + 0x30, time.to_bits());
            t.write(m, &varied(0x48, k + 4));
            t.set_u32(m + 0x30, x);
            t.set_u32(m + 0x40, motion);
            t.set_u32(motion + 0x20, 0.5f32.to_bits());
            t.write(table, &varied(0x200, k + 5));
            t.set_u32(table + 0x18, count.to_bits());
            scrub(&mut t);
            // the local the original tests without a table
            t.set_u32(ENTRY_ESP - 8, 0);
            let call = Call {
                ecx: s,
                eax: 0x1234_5600 + k,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::ring_key_frame(g, img, fr))
            });
        }
    }
}

/// Row 663: the two scaled unit matrices of both DLLs (the slots see the
/// locals and the matrices they made).
#[test]
fn scaled_matrices() {
    for (dll, rva) in [(443u16, 0x1000u32), (686, 0x11a0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        rec(&mut t, 719, 3, |_, a| a[0] ^ 0x0719_0000);
        t.stub(235, move |m, a| {
            let k = m.r32(log);
            m.w32(log, k + 1);
            for i in 0..16u32 {
                let v = if i % 5 == 0 { 0x3f80_0000 } else { k << 4 | i };
                m.w32(a[0] + 4 * i, v);
            }
            0x0235_0000 + k
        });
        t.stub(231, move |m, a| {
            let row = log + 0x100;
            for i in 0..16u32 {
                m.w32(row + 4 * i, m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i));
            }
            m.w32(row + 0x40, a[1]);
            0x0231_0000 ^ a[1]
        });
        for k in 0..3u32 {
            t.set_u32(log, 0);
            t.set_u32(op(&t, f, 0x2b), 0x5150_0000 + k);
            t.set_u32(op(&t, f, 0x59), 0x100 * k + 7);
            t.set_u32(op(&t, f, 0x5f), 0x33 + k);
            t.set_u32(op(&t, f, 0x6d), 0xffff_ff00 + k);
            t.write(op(&t, f, 0xa3), &varied(0x40, k));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Row 405: the eight global differences of the three DLLs, with the flag
/// and without (values with signs, zeros and an infinity).
#[test]
fn global_differences() {
    for (dll, rva) in [(42u16, 0x8200u32), (152, 0x7fb0), (533, 0x7fc0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (k, flag) in [0u32, 1, 0, 0x8000_0000].into_iter().enumerate() {
            let k = k as u32;
            // the operands of the sixteen loads and subtractions
            let mut reads = vec![0x22u32, 0x2a];
            for s in (0x3du32..=0xcd).step_by(0x18) {
                reads.extend([s + 4, s + 0xc]);
            }
            for (i, at) in (0u32..).zip(reads) {
                let x = match (i + k) % 5 {
                    0 => 0.0,
                    1 => -1.5 * i as f32,
                    2 => f32::INFINITY,
                    _ => 0.375 * (i + k) as f32,
                };
                t.set_u32(op(&t, f, at), x.to_bits());
            }
            for o in [0x39u32, 0x51, 0x69, 0x81, 0x99, 0xb1, 0xc9, 0xe1] {
                t.set_u32(op(&t, f, o), 0x0bad_0000 + o);
            }
            t.set_u32(op(&t, f, 0x18), 0xffff_00f0 ^ k);
            t.set_u32(op(&t, f, 0xa), 0x1234);
            let c = Call {
                edx: flag,
                eax: 0x4050_0000 + k,
                ..Call::default()
            }
            .with_xmm(&[
                Xmm::f32(1.0),
                Xmm::f32(2.0),
                Xmm::f32(-3.25 + k as f32),
            ]);
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::global_differences(g, img, fr))
            });
        }
    }
}

/// Row 406: the object start with the 0x1e reset state block in the
/// three DLLs: the object list empty and not, the pool empty.
#[test]
fn objects_started_reset_1e() {
    for (dll, rva) in [(101u16, 0x1a80u32), (104, 0x20f0), (105, 0x18f0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        let start = t.img.rel32_target(f + 0x7d);
        t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
        t.set_u32(op(&t, f, 0x1), SCRATCH + 0x2f0);
        t.set_u32(op(&t, f, 0x5b), SCRATCH + 0x280);
        let pa = t.img.rel32_target(f + 0x95);
        let free = op(&t, pa, 0x04);
        let tail = op(&t, f, 0xa0);
        let obj = SCRATCH + 0x1000;
        for (k, (nodes, listed)) in [(5u16, false), (2, true), (0, true)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.write(obj, &varied(0x140, k));
            t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
            t.set_u16(free, nodes);
            t.write(SCRATCH + 0x1400, &varied(0x10, k));
            t.set_u32(tail, if listed { SCRATCH + 0x1400 } else { 0 });
            let c = Call {
                edx: obj,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            }
            .with_xmm(&[
                Xmm::f32(9.0),
                Xmm::f32(8.0),
                Xmm::f32(7.0),
                Xmm::f32(1.25 + k as f32),
            ]);
            scrub(&mut t);
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s6_misc::start_object_reset(g, img, fr))
            });
        }
    }
}

/// Row 407: a node from the free list onto the live list in the three
/// DLLs: one, two and five free, the live list empty and not.
#[test]
fn nodes_to_live() {
    for (dll, rva) in [(285u16, 0x20b0u32), (286, 0x3090), (287, 0x20b0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let o = SCRATCH + 0x300;
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        for (k, (free, first)) in [(1u16, 0u32), (2, 0), (5, SCRATCH + 0x800), (2, 1)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.write(o, &varied(0x140, k));
            for i in 0..4 {
                t.write(node(i), &varied(0x40, k + i));
            }
            // the free ring: node 0 between nodes 1 (prev) and 2 (next)
            t.set_u32(node(0), node(2));
            t.set_u32(node(0) + 4, node(1));
            t.set_u16(op(&t, f, 0x4), free);
            t.set_u32(op(&t, f, 0x48), node(0));
            t.set_u32(op(&t, f, 0x2b), 0x2b2b);
            t.set_u32(op(&t, f, 0x35), 0x3535);
            t.set_u32(op(&t, f, 0xa), 0x0a0a);
            let first = if first == 1 { node(3) } else { first };
            t.set_u32(op(&t, f, 0xaf), first);
            t.write(SCRATCH + 0x800, &varied(0x20, k));
            t.set_u32(op(&t, f, 0xbf), if first == 0 { 0x5555 } else { node(3) });
            let c = Call {
                ecx: o,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::node_to_live(g, img, fr))
            });
        }
    }
}

/// Row 412: a part's count down in the three DLLs: above 0 (three
/// children, each getting slot 942's answer) and reaching 0 (one node
/// with the part's counter), indices 0..2.
#[test]
fn countdown_children() {
    for (dll, rva) in [(191u16, 0x1620u32), (415, 0x1600), (534, 0x1630)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let nodes = SCRATCH + 0x800;
        log_calls(&mut t, 434, log, 2, None, move |m, _, k| {
            let n = nodes + 0x100 * (k % 4);
            m.w32(n + 0xac, k % 3);
            n
        });
        log_calls(&mut t, 942, log, 1, None, |m, a, k| m.r32(a[0]) ^ (k << 12));
        for (k, timer) in [5i32, 1, 2, 0, -3, 0x7fff_ffff].into_iter().enumerate() {
            let k = k as u32;
            let o = SCRATCH + 0x400;
            t.set_u32(log, 0);
            t.write(o, &varied(0x200, k));
            t.write(nodes, &varied(0x400, k + 1));
            let i = k % 3;
            t.set_u32(o + 0xac, i);
            t.set_u32(o + 0x30 + 0x10 * i, timer.cast_unsigned());
            for x in [0x3du32, 0x63, 0x98, 0xd8] {
                t.set_u32(op(&t, f, x), 0x0d00_0000 + x);
            }
            scrub(&mut t);
            let c = Call {
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[o], Eax::Checked);
        }
    }
}

/// Rows 311 and 312: the two summon frames (bone before the ring, ring
/// before the bone) in their four DLLs each: before stage 4, an empty
/// ring, a ring of two and the ring skipped (with and without nodes);
/// slot 214 with the flag and the word set or not. The bone and the two
/// nodes are already at the pass.
#[test]
fn summon_frames_bound() {
    let bound = [(155u16, 0xc450u32), (570, 0xccb0), (609, 0xc170), (613, 0xbcc0)];
    let ring = [(150u16, 0xdfa0u32), (152, 0xc8f0), (533, 0xc320), (601, 0xa5b0)];
    for (dll, rva, first) in bound
        .into_iter()
        .map(|(d, r)| (d, r, true))
        .chain(ring.into_iter().map(|(d, r)| (d, r, false)))
    {
        summon_frame_in(dll, rva, first);
    }
}

fn summon_frame_in(dll: u16, rva: u32, bone_first: bool) {
    let Some(mut t) = twin(dll) else { return };
    let f = t.at(rva);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // (pass, bone A, the two nodes, skip, first, root, flags, mask, W, Z)
    let (pass, nodes, skip, first, root, flags, mask, word, z) = if bone_first {
        (0x28, [0x34u32, 0xa1, 0xc1], 0x43, 0x4a, 0x52, 0x88, 0xcb, 0xdf, 0xed)
    } else {
        (0x78, [0x84u32, 0xa5, 0xc5], 0x29, 0x30, 0x38, 0x70, 0xcf, 0xe3, 0xf1)
    };
    let bm = t.img.rel32_target(f + 0x23);
    t.set_u32(t.img.word(bm + 0x7e), sa::node(20));
    for s in [723u32, 724, 230, 253] {
        rec(&mut t, s, 0, move |_, _| s);
    }
    rec(&mut t, 214, 1, |_, a| a[0] ^ 0x0214_0000);
    t.set_u32(w(&t, 0x18), sa::node(21));
    let vt = sa::node(22);
    t.set_u32(vt + 0x20, magic::TRAPS);
    rec(&mut t, 0, 1, |m, a| {
        let x = m.r32(a[0]);
        let k = m.r32(sa::node(23));
        m.w32(sa::node(23), k + 1);
        m.w32(sa::node(23) + 4 + 4 * (k % 8), x);
        m.w32(sa::node(23) + 0x24 + 4 * (k % 8), m.r32(a[0] + 4));
        0x77
    });
    let root_at = w(&t, root);
    assert_eq!(w(&t, first), root_at + 8);
    for (k, (stage, sk, n, fl, wd)) in [
        (3u32, 0u32, 0u32, 1u32, 0u16),
        (4, 0, 0, 0x11, 0),
        (4, 0, 2, 0x10, 0),
        (5, 1, 2, 1, 0),
        (4, 1, 0, 1, 1),
        (0x7fff_ffff, 0, 1, 3, 0),
        (0xffff_ffff, 0, 0, 1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0xb), stage);
        t.set_u32(w(&t, skip), sk);
        let p = (0x40 + k) as u8;
        t.set_u8(w(&t, pass), p);
        let mut prev = root_at;
        for i in 0..n {
            let nd = sa::node(10 + i);
            t.write(nd, &varied(0x40, k + i));
            t.set_u32(nd, vt);
            t.set_u32(prev + 8, nd);
            prev = nd;
        }
        t.set_u32(prev + 8, root_at);
        // the bone and the two nodes at the pass: nothing moves
        for o in nodes {
            t.set_u8(w(&t, o) + 0x1a, p);
        }
        t.set_u32(w(&t, flags), 0x10);
        t.set_u32(w(&t, mask), fl);
        t.set_u16(w(&t, word), wd);
        t.set_u32(w(&t, z), 0x2a2a_0000 + k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Row 256: the camera settings of the four DLLs (every slot logs its
/// words; the divisor 0, 1 and another value).
#[test]
fn camera_settings() {
    for (dll, rva) in [(98u16, 0x1090u32), (188, 0x1090), (686, 0x1090), (690, 0x1090)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        for (s, n) in [(783u32, 1usize), (782, 1), (784, 1), (781, 3), (785, 1), (801, 6)] {
            log_calls(&mut t, s, log, n, None, |_, a, k| a[0] ^ (k << 24));
        }
        for (k, d) in [0.0f32, 1.0, -2.5].into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(log, 0);
            for (i, o) in (0u32..).zip([0x9u32, 0x24, 0x3e, 0x53, 0x65, 0x7c, 0x90, 0xa8, 0xef]) {
                t.set_u32(op(&t, f, o), (0.5 + i as f32 + k as f32).to_bits());
            }
            t.set_u32(op(&t, f, 0xb0), d.to_bits());
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}
