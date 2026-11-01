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
    for (dll, rva, thunk) in [(150u16, 0x4110u32, 0x6e60u32), (603, 0x4110, 0x6e60)] {
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
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[cell], Eax::Checked);
        }
    }
}

/// Row 344: a part's count and fade in the three DLLs: counts far below,
/// in the last five and at or past the limit, indices 0..2, enough runs
/// for `rand()`'s 1-in-16 node start (slots 434 and 942 logged).
#[test]
fn count_fades() {
    for (dll, rva, thunk) in [
        (384u16, 0x1d30u32, 0x1f10u32),
        (421, 0x1d20, 0x1f00),
        (535, 0x1d00, 0x1ee0),
    ] {
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
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[o], Eax::Checked);
        }
    }
}

/// Row 469: a ring pass and a key frame in both DLLs: the ring already
/// passed, empty and with two nodes (their handler logging the cursor);
/// no motion, no key table (the junk local 0), keys before, at and past
/// the table's count.
#[test]
fn ring_key_frames() {
    for (dll, rva, thunk) in [(578u16, 0x66a0u32, 0x6780u32), (649, 0x6740, 0x6820)] {
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
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
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
            .with_xmm(&[Xmm::f32(1.0), Xmm::f32(2.0), Xmm::f32(-3.25 + k as f32)]);
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
    let bound = [
        (155u16, 0xc450u32),
        (570, 0xccb0),
        (609, 0xc170),
        (613, 0xbcc0),
    ];
    let ring = [
        (150u16, 0xdfa0u32),
        (152, 0xc8f0),
        (533, 0xc320),
        (601, 0xa5b0),
    ];
    for (dll, rva, first) in bound
        .into_iter()
        .map(|(d, r)| (d, r, true))
        .chain(ring.into_iter().map(|(d, r)| (d, r, false)))
    {
        summon_frame_in(dll, rva, first);
    }
}

/// Where a summon frame's operands are, for its test.
struct FrameOps {
    /// The pass, bone A and the two nodes, skip, first, root, flags,
    /// mask, W, Z.
    pass: u32,
    nodes: [u32; 3],
    skip: u32,
    first: u32,
    root: u32,
    flags: u32,
    mask: u32,
    word: u32,
    z: u32,
    /// The models' binding `rel32`, `*P = ITEM`'s `P`.
    bind: u32,
    p: u32,
    /// The pass goes up first (and `G_X = *G_Y`: their operands).
    copy: Option<(u32, u32)>,
}

fn summon_frame_in(dll: u16, rva: u32, bone_first: bool) {
    let ops = if bone_first {
        FrameOps {
            pass: 0x28,
            nodes: [0x34, 0xa1, 0xc1],
            skip: 0x43,
            first: 0x4a,
            root: 0x52,
            flags: 0x88,
            mask: 0xcb,
            word: 0xdf,
            z: 0xed,
            bind: 0x23,
            p: 0x18,
            copy: None,
        }
    } else {
        FrameOps {
            pass: 0x78,
            nodes: [0x84, 0xa5, 0xc5],
            skip: 0x29,
            first: 0x30,
            root: 0x38,
            flags: 0x70,
            mask: 0xcf,
            word: 0xe3,
            z: 0xf1,
            bind: 0x23,
            p: 0x18,
            copy: None,
        }
    };
    summon_frame_with(dll, rva, &ops);
}

/// Rows of `magic_0577` 0x1000dd60 (the pass first, a global copied) and
/// `magic_0603` 0x1000da40 (slot 673 first): the variants of the summon
/// frame, through the same cases.
#[test]
fn summon_frames_variants() {
    let common = FrameOps {
        pass: 0x88,
        nodes: [0x94, 0xb5, 0xd5],
        skip: 0,
        first: 0,
        root: 0,
        flags: 0x80,
        mask: 0xdf,
        word: 0xf3,
        z: 0x101,
        bind: 0,
        p: 0,
        copy: None,
    };
    summon_frame_with(
        577,
        0xdd60,
        &FrameOps {
            skip: 0x39,
            first: 0x40,
            root: 0x48,
            bind: 0x33,
            p: 0x18,
            copy: Some((0x2e, 0x29)),
            ..common
        },
    );
    summon_frame_with(
        603,
        0xda40,
        &FrameOps {
            skip: 0x36,
            first: 0x3d,
            root: 0x45,
            bind: 0x30,
            p: 0x25,
            ..common
        },
    );
}

fn summon_frame_with(dll: u16, rva: u32, ops: &FrameOps) {
    let Some(mut t) = twin(dll) else { return };
    let f = t.at(rva);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let FrameOps {
        pass,
        nodes,
        skip,
        first,
        root,
        flags,
        mask,
        word,
        z,
        ..
    } = *ops;
    rec(&mut t, 673, 0, |_, _| 0x673);
    if let Some((x, y)) = ops.copy {
        t.set_u32(w(&t, x), 0x0c0c_0000);
        t.set_u32(w(&t, y), 0x0d0d_1234);
    }
    let bm = t.img.rel32_target(f + ops.bind);
    t.set_u32(t.img.word(bm + 0x7e), sa::node(20));
    for s in [723u32, 724, 230, 253] {
        rec(&mut t, s, 0, move |_, _| s);
    }
    rec(&mut t, 214, 1, |_, a| a[0] ^ 0x0214_0000);
    t.set_u32(w(&t, ops.p), sa::node(21));
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
        let now = if ops.copy.is_some() {
            p.wrapping_add(1)
        } else {
            p
        };
        for o in nodes {
            t.set_u8(w(&t, o) + 0x1a, now);
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
    for (dll, rva) in [
        (98u16, 0x1090u32),
        (188, 0x1090),
        (686, 0x1090),
        (690, 0x1090),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        for (s, n) in [
            (783u32, 1usize),
            (782, 1),
            (784, 1),
            (781, 3),
            (785, 1),
            (801, 6),
        ] {
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

/// A ring for the node handlers' pass: the cursor `s` on the ring `r`
/// (pass byte `pass`, the ring's byte `byte`) with `n` nodes (vtable
/// `vt`) from `SCRATCH + 0x600`.
fn set_ring(t: &mut Twin, s: u32, r: u32, (pass, byte): (u8, u8), n: u32, vt: u32, k: u32) {
    t.write(s, &varied(0x10, k));
    t.set_u32(s, r);
    t.set_u8(s + 5, pass);
    t.set_u8(r + 0x1a, byte);
    let mut prev = r;
    for i in 0..n {
        let nd = SCRATCH + 0x600 + 0x40 * i;
        t.write(nd, &varied(0x40, k + 2 + i));
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
}

/// The ring nodes' handler: slot 1000, logging the cursor's node.
fn ring_handler(t: &mut Twin, log: u32) -> u32 {
    t.set_u32(magic::TABLE + 4 * 1000, trap(1000));
    log_calls(t, 1000, log, 1, None, |m, a, k| {
        let n = m.r32(a[0]);
        m.w32(n + 0x14, k);
        0x1000_0000 + (m.r32(n + 0xc) << 8) + k
    });
    let vt = SCRATCH + 0x700;
    t.set_u32(vt + 0x1c, trap(1000));
    vt
}

/// Row 459: the model points node of both DLLs: not passed, and passed
/// with each flag combination (the child's handler with its own cursor,
/// no child).
#[test]
fn model_points_nodes() {
    for (dll, rva, thunk) in [(150u16, 0x2d30u32, 0x2e60u32), (603, 0x2d30, 0x2e60)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        log_calls(&mut t, 229, log, 3, None, |_, a, k| a[0] ^ k);
        log_calls(&mut t, 284, log, 3, None, |m, a, k| m.r32(a[2]) ^ k);
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let child = SCRATCH + 0x900;
        for (k, (byte, n, bits, has_child)) in [
            (3u8, 1u32, 0u8, true),
            (4, 0, 0, true),
            (4, 2, 0xfe, true),
            (4, 1, 1, true),
            (4, 1, 1, false),
            (4, 0, 3, true),
            (4, 0, 0xff, false),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(r, &varied(0x260, k));
            set_ring(&mut t, s, r, (3, byte), n, vt, k);
            t.set_u8(r + 0x240, bits);
            t.write(child, &varied(0x40, k + 7));
            t.set_u32(child, vt);
            t.set_u32(r + 0x228, if has_child { child } else { 0 });
            scrub(&mut t);
            let c = Call {
                ecx: s,
                eax: 0x2d30_0000 + k,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::model_points_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Row 439: the light node of both DLLs: the ring passed or not, no
/// count, a value not above 0 (0, negative, NaN) and above it.
#[test]
fn light_nodes() {
    for (dll, rva, thunk) in [(150u16, 0x1000u32, 0x1140u32), (603, 0x1000, 0x1140)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        log_calls(&mut t, 245, log, 1, None, |m, a, k| {
            m.w32(a[0], 0x2450_0000 + k);
            m.w32(a[0] + 4, 0x2451_0000 + k);
            0x245
        });
        log_calls(&mut t, 821, log, 1, None, |_, _, k| 0x821 + k);
        log_calls(&mut t, 822, log, 4, None, |_, _, k| 0x822 + k);
        log_calls(&mut t, 823, log, 2, None, |_, _, k| 0x823 + k);
        log_calls(&mut t, 824, log, 4, None, |_, _, k| 0x824 + k);
        t.set_u32(op(&t, f, 0x80), 3.0f32.to_bits());
        t.set_u32(op(&t, f, 0xb5), 0.125f32.to_bits());
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        for (k, (byte, n, x)) in [
            (3u8, 5u8, 2.0f32),
            (4, 0, 2.0),
            (4, 7, 0.0),
            (4, 7, -1.0),
            (4, 7, f32::NAN),
            (4, 0xff, 0.75),
            (3, 1, 1e-3),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(r, &varied(0x40, k));
            set_ring(&mut t, s, r, (3, byte), k % 2, vt, k);
            t.set_u8(r + 0x1c, n);
            t.set_u32(r + 0x20, x.to_bits());
            scrub(&mut t);
            let c = Call {
                ecx: s,
                eax: 0x1000_0000 + k,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::light_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Row 174: the loading summon frame of the five DLLs: each load state
/// busy and done (0 and 1 run on), state 2 with its call, the frame with
/// an empty ring, two nodes and the ring skipped. The two nodes are
/// already at the pass.
#[test]
fn summon_load_frames() {
    for (dll, rva) in [
        (152u16, 0xc7b0u32),
        (155, 0xc310),
        (164, 0xc050),
        (533, 0xc1e0),
        (601, 0xa470),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let log = SCRATCH + 0x6000;
        let busy = SCRATCH + 0x2f0;
        for s in [225u32, 313, 549] {
            log_calls(&mut t, s, log, 1, None, move |m, _, _| {
                m.r32(busy) & (1 << (s % 8))
            });
        }
        log_calls(&mut t, 226, log, 2, None, |_, _, k| 0x226 + k);
        log_calls(&mut t, 673, log, 0, None, |_, _, k| 0x673 + k);
        t.set_u32(w(&t, 0x99), sa::node(21));
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
        let root = w(&t, 0xca);
        assert_eq!(w(&t, 0xaa), root + 8);
        // (state, busy bits, skip, nodes)
        for (k, (state, b, skip, n)) in [
            (0u32, 0xffu32, 0u32, 0u32),
            (0, 0, 0, 2),
            (1, 1 << (313 % 8), 0, 0),
            (0, 1 << (313 % 8), 0, 0),
            (1, 0, 0, 1),
            (2, 1 << (549 % 8), 0, 0),
            (2, 0, 1, 2),
            (3, 0xff, 0, 0),
            (4, 0, 1, 0),
            (0xffff_ffff, 0, 0, 2),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(busy, b);
            t.set_u32(w(&t, 0x4), state);
            t.set_u32(w(&t, 0xbc), skip);
            t.set_u32(w(&t, 0x71), 0x7171_0000 + k);
            t.set_u32(w(&t, 0xa4), 0xa4a4_0000 + k);
            let p = (0x40 + k) as u8;
            t.set_u8(w(&t, 0xb4), p);
            let mut prev = root;
            for i in 0..n {
                let nd = sa::node(10 + i);
                t.write(nd, &varied(0x40, k + i));
                t.set_u32(nd, vt);
                t.set_u32(prev + 8, nd);
                prev = nd;
            }
            t.set_u32(prev + 8, root);
            // the two nodes at the new pass: nothing moves
            for o in [0x113u32, 0x12b] {
                t.set_u8(w(&t, o) + 0x1a, p.wrapping_add(1));
            }
            t.set_u32(w(&t, 0x104), 0x10);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Row 346: the model draw node of the three DLLs: not passed, passed
/// (the slots see the matrix and the draw record).
#[test]
fn model_draw_nodes() {
    for (dll, rva, thunk) in [
        (478u16, 0x3f00u32, 0x4050u32),
        (513, 0x3f80, 0x40d0),
        (514, 0x4320, 0x4470),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        t.stub(231, move |m, a| {
            for i in 0..16u32 {
                let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x231
        });
        t.stub(308, move |m, a| {
            let row = log + 0x200;
            for i in 0..0x12u32 {
                m.w32(row + 4 * i, m.r32(a[1] + 4 * i));
            }
            m.w32(row + 0x48, a[0]);
            m.r32(a[1] + 0x20) ^ a[0]
        });
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let model = SCRATCH + 0x1800;
        t.set_u32(op(&t, f, 0xa1), SCRATCH + 0x2f0);
        for (k, byte) in [3u8, 4, 4].into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(r, &varied(0x40, k));
            set_ring(&mut t, s, r, (3, byte), k, vt, k);
            t.set_u32(r + 0x20, model);
            t.set_u32(r + 0x1c, SCRATCH + 0x1c00);
            t.write(SCRATCH + 0x1c00, &varied(0x100, k + 1));
            t.write(model, &varied(0x40, k + 2));
            t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
            t.set_u32(op(&t, f, 0x105), 0x0105_0000 + k);
            scrub(&mut t);
            let c = Call {
                ecx: s,
                eax: 0x3f00_0000 + k,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::model_draw_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Row 572: the look matrix of both DLLs (the slots compute on the frame's
/// vectors; their answers fill the rows).
#[test]
fn look_matrices() {
    for (dll, rva) in [(188u16, 0x3c20u32), (690, 0x3c20)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        // a = b - c, a = norm(b) (here: b with a mark), a = b x c
        log_calls(&mut t, 234, log, 3, None, |m, a, k| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i).wrapping_sub(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, v);
            }
            0x234 + k
        });
        log_calls(&mut t, 232, log, 2, None, |m, a, k| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i) ^ (k << 4 | i);
                m.w32(a[0] + 4 * i, v);
            }
            0x232 + k
        });
        log_calls(&mut t, 266, log, 3, None, |m, a, k| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ m.r32(a[2] + 4 * ((i + 1) % 4));
                m.w32(a[0] + 4 * i, v);
            }
            0x266 + k
        });
        let o = SCRATCH + 0x300;
        let (p, q) = (SCRATCH + 0x400, SCRATCH + 0x440);
        for k in 0..3u32 {
            t.set_u32(log, 0);
            t.write(o, &varied(0x40, k));
            t.write(p, &varied(0x10, k + 1));
            t.write(q, &varied(0x10, k + 2));
            scrub(&mut t);
            let c = Call {
                ecx: o,
                edx: p,
                ..Call::default()
            };
            t.check(f, c, &[q], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::look_matrix(g, img, fr))
            });
        }
    }
}

/// Row 758: the landing check and puff of both DLLs: points outside the
/// box on each side (and NaN), inside with a rate (0, -0 and others),
/// the object started and the puff (its ring with a free record).
#[test]
fn landing_puffs() {
    for (dll, rva) in [(285u16, 0x4cc0u32), (286, 0x1b70)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // the object start's slots (`s4_misc::start_object_at`)
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        let soa = t.img.rel32_target(f + 0x105);
        let start = t.img.rel32_target(soa + 0x84);
        t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
        t.set_u32(op(&t, soa, 0x04), SCRATCH + 0x2f0);
        t.set_u32(op(&t, soa, 0x5e), SCRATCH + 0x280);
        let pa = t.img.rel32_target(soa + 0x9d);
        let free = op(&t, pa, 0x04);
        let tail = op(&t, soa, 0xa8);
        // the puff's (`s6_misc::smoke_puff`)
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
        let calls = SCRATCH + 0x7e0;
        rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ (a[0] & 0xff));
        t.stub_st0(314, move |m, a| {
            let k = m.r32(calls);
            m.w32(calls, k + 1);
            f64::from(f32::from_bits(a[0])) * f64::from(k % 5 + 1)
        });
        let puff = t.img.rel32_target(f + 0x14c);
        let take = t.img.rel32_target(puff + 0x9);
        let ring = SCRATCH + 0x1000;
        t.set_u32(op(&t, take, 0x17), ring);
        t.set_u32(op(&t, take, 0x0a), 4);
        for (o, x) in [
            (0xfu32, -10.0f32),
            (0x1d, 10.0),
            (0x32, -4.0),
            (0x6e, 1.0),
            (0xc3, 0.5),
        ] {
            t.set_u32(w(&t, o), x.to_bits());
        }
        let e = SCRATCH + 0x300;
        // (x, y, z, the rate, the object's index)
        for (k, (x, y, z, v, i)) in [
            (-11.0f32, 1.0f32, 0.0f32, 1.0f32, 0u32),
            (11.0, 1.0, 0.0, 1.0, 0),
            (f32::NAN, 1.0, 0.0, 1.0, 0),
            (0.0, 1.0, -5.0, 1.0, 0),
            (0.0, -0.5, 0.0, 1.0, 0),
            (0.0, f32::NAN, 0.0, 1.0, 0),
            (1.5, 2.0, 3.0, 0.0, 1),
            (1.5, 2.0, 3.0, -0.0, 2),
            (1.5, 2.0, 3.0, 0.75, 0),
            (-2.5, 0.0, 7.0, -3.0, 3),
            (2.0, 4.0, 1.0, f32::NAN, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.write(e, &varied(0x290, k));
            t.set_u32(e + 0xc, x.to_bits());
            t.set_u32(e + 0x10, y.to_bits());
            t.set_u32(e + 0x14, z.to_bits());
            t.set_u32(e + 0x25c, 1.5f32.to_bits());
            t.set_u32(e + 0x264, (-0.75f32).to_bits());
            t.set_u32(e + 0x260, v.to_bits());
            t.set_u32(e + 0x280, i);
            t.set_u32(e + 0x250, (k as f32 - 4.0).to_bits());
            t.set_u16(free, 3);
            t.write(SCRATCH + 0x1400, &varied(0x10, k));
            t.set_u32(tail, if k % 2 == 0 { SCRATCH + 0x1400 } else { 0 });
            t.write(ring, &varied(0x68 * 4, k));
            for j in 0..4u32 {
                t.set_u32(ring + 0x68 * j + 0x60, u32::from(j != 2));
            }
            t.set_u32(op(&t, take, 0x02), 1);
            scrub(&mut t);
            let c = Call {
                eax: 0x0eaa_0000 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[e], Eax::Checked);
        }
    }
}

/// Row 248: the moving light node of the four DLLs: not passed, no steps
/// (bit 0 of `+0x58` with a parent, without one, clear), the clock below,
/// at and past its bound (and NaN).
#[test]
fn moving_light_nodes() {
    for (dll, rva, thunk) in [
        (155u16, 0x4720u32, 0x4890u32),
        (164, 0x4710, 0x4880),
        (609, 0x47c0, 0x4930),
        (613, 0x4790, 0x4900),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        log_calls(&mut t, 325, log, 2, None, |m, a, k| {
            m.w32(a[0], m.r32(a[1]) ^ k);
            0x325 + k
        });
        log_calls(&mut t, 776, log, 1, None, |_, a, k| a[0] ^ (k << 16));
        log_calls(&mut t, 915, log, 5, None, |_, a, k| a[4] ^ k);
        t.set_u32(op(&t, f, 0x9a), 2.0f32.to_bits());
        t.set_u32(op(&t, f, 0xdf), 0.5f32.to_bits());
        t.set_u32(op(&t, f, 0xe7), 4.0f32.to_bits());
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let parent = SCRATCH + 0x1800;
        // (ring byte, steps, flags, parent, clock, rate)
        for (k, (byte, steps, fl, par, clock, rate)) in [
            (3u8, 2u32, 0u8, true, 0.0f32, 0.5f32),
            (4, 0, 1, true, 0.0, 0.5),
            (4, 0, 1, false, 0.0, 0.5),
            (4, 0, 0xfe, true, 0.0, 0.5),
            (4, 3, 0, false, 0.25, 0.5),
            (4, 3, 0, false, 1.5, 0.5),
            (4, 1, 0, false, 1.75, 0.5),
            (4, 2, 0, false, f32::NAN, 0.5),
            (4, 0xffff_ffff, 0, false, 3.0, -0.25),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(r, &varied(0x60, k));
            set_ring(&mut t, s, r, (3, byte), k % 2, vt, k);
            t.set_u32(r + 0x54, steps);
            t.set_u8(r + 0x58, fl);
            t.write(parent, &varied(0x40, k + 3));
            t.set_u32(r + 0x14, if par { parent } else { 0 });
            // the parent's ring around the node (`sa_misc::leave`)
            t.set_u32(r + 4, parent + 0x20);
            t.set_u32(r + 8, parent + 0x20);
            t.set_u32(r + 0x4c, clock.to_bits());
            t.set_u32(r + 0x50, rate.to_bits());
            for i in 0..8u32 {
                t.set_u32(r + 0x1c + 4 * i, (0.5 * i as f32 - k as f32).to_bits());
            }
            scrub(&mut t);
            let c = Call {
                ecx: s,
                eax: 0x4720_0000 + k,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::moving_light_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Rows 176 and 178: the scripted summon frames (`magic_0565` and its
/// four copies with the selection groups, `magic_0644` and its four with
/// the kind records) in every state: 0, 1 with the script running and
/// ended, 2 (a record's script whose command ends it) and 3 with records
/// left and not (the closing slots, the node given back), and past 3.
/// The motion step is done (it returns at once).
#[test]
fn summon_script_frames() {
    for (dll, rva) in [
        (565u16, 0x7a50u32),
        (691, 0x7a50),
        (693, 0x7a50),
        (695, 0x7a50),
        (697, 0x7a50),
        (644, 0xb8f0),
        (692, 0xb8f0),
        (694, 0xb8f0),
        (696, 0xb8f0),
        (698, 0xb8f0),
    ] {
        summon_script_frame_in(dll, rva);
    }
}

fn summon_script_frame_in(dll: u16, rva: u32) {
    let Some(mut t) = twin(dll) else { return };
    let f = t.at(rva);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let log = SCRATCH + 0x6000;
    // the pads (`sb_misc::read_pads`: slot 305 one port), the record's
    // slot and the closing ones (`sb_misc::call_628_213`)
    log_calls(&mut t, 305, log, 0, None, |_, _, k| k % 2);
    for (s, n) in [
        (302u32, 3usize),
        (303, 3),
        (304, 3),
        (647, 1),
        (628, 1),
        (213, 2),
    ] {
        log_calls(&mut t, s, log, n, None, move |_, a, k| s ^ a[0] ^ (k << 12));
    }
    // the kinds: one list with a word, the rest empty
    log_calls(&mut t, 645, log, 4, None, |m, a, _| {
        if a[2] == 1 {
            m.w32(a[1], 2);
            m.w32(a[3], 0x0645_0000);
        } else {
            m.w32(a[1], 0);
            m.w32(a[3], 0);
        }
        SCRATCH + 0x5f0
    });
    // the motion step is done: it returns 1 at once
    let motion = t.img.rel32_target(f + 0x88);
    t.set_u32(op(&t, motion, 0x02), 1);
    // the script's command table: every command ends the script
    let run = t.img.rel32_target(f + 0x92);
    let cmds = op(&t, run, 0x17);
    t.set_u32(magic::TABLE + 4 * 1001, trap(1001));
    log_calls(&mut t, 1001, log, 2, None, |m, a, _| {
        // the mode byte (`+0x56`) 0: ended
        let at = a[0] + 0x54;
        let v = m.r32(at) & 0xff00_ffff;
        m.w32(at, v);
        a[1] + 2
    });
    for i in 0..8 {
        t.set_u32(cmds + 4 * i, trap(1001));
    }
    let b = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x6), b);
    // the block's words around the state (an operand)
    let st = b + w(&t, 0x44);
    let s = w(&t, 0x59);
    let node = SCRATCH + 0x400;
    // the records' scripts (the DLL's table of them)
    let scripts = w(&t, 0xee);
    for i in 0..2 {
        t.set_u32(scripts + 4 * i, SCRATCH + 0x5100 + 0x10 * i);
        t.set_u32(SCRATCH + 0x5100 + 0x10 * i, 3 + i);
    }
    // (state, the script's mode byte, the record number, the records)
    for (k, (state, mode, n, count)) in [
        (0u32, 0u8, 0u32, 2u32),
        (1, 2, 0, 2),
        (1, 0, 0, 2),
        (2, 1, 0, 2),
        (2, 1, 1, 2),
        (3, 0, 1, 2),
        (3, 0, 2, 2),
        (3, 2, 2, 2),
        (4, 0, 0, 0),
        (0xffff_ffff, 0, 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(log, 0);
        t.write(st - 0xac, &varied(0xb8, k));
        for r in 0..3u32 {
            t.set_u32(st - 0xac + 0x28 * r, r % 2);
        }
        t.set_u32(st - 8, count);
        t.set_u32(st - 4, n);
        t.set_u32(st, state);
        t.write(s, &varied(0x58, k));
        let script = SCRATCH + 0x5000;
        t.set_u32(s, script);
        t.set_u32(script, k % 4);
        t.set_u8(s + 0x56, mode);
        // a wait for ever
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x24, 0);
        t.set_u32(s + 0x28, 0);
        // the node and its child (`sa_pool::give_back`)
        t.write(node, &varied(0x40, k));
        t.set_u32(node + 8, node + 0x20);
        t.set_u32(node + 0x2c, 0);
        t.set_u32(node + 4, node);
        t.set_u32(node, node);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// The camera settings' slots, each logging its words.
fn camera_stubs(t: &mut Twin, log: u32) {
    for (s, n) in [
        (783u32, 1usize),
        (782, 1),
        (784, 1),
        (781, 3),
        (785, 1),
        (801, 6),
        (267, 1),
    ] {
        log_calls(t, s, log, n, None, |_, a, k| a[0] ^ (k << 24));
    }
}

/// Row 257: the camera settings after slot 267 in the four DLLs.
#[test]
fn camera_settings_after_267() {
    for (dll, rva, cam) in [
        (98u16, 0x20d0u32, 0x1090u32),
        (188, 0x2560, 0x1090),
        (686, 0x1dc0, 0x1090),
        (690, 0x2560, 0x1090),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let c = t.at(cam);
        let log = SCRATCH + 0x6000;
        camera_stubs(&mut t, log);
        for (k, d) in [0.0f32, 2.0].into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(log, 0);
            for (i, o) in (0u32..).zip([0x9u32, 0x24, 0x3e, 0x53, 0x65, 0x7c, 0x90, 0xa8, 0xef]) {
                t.set_u32(op(&t, c, o), (1.5 + i as f32 + k as f32).to_bits());
            }
            t.set_u32(op(&t, c, 0xb0), d.to_bits());
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Row 664: the fading level of both DLLs: off, the level held and going
/// down, clamped at 0 and at the top, NaN, reaching the end value or not.
#[test]
fn fading_levels() {
    for (dll, rva) in [(443u16, 0x19d0u32), (686, 0x1b00)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        // the matrices' slots (`scaled_matrices`)
        rec(&mut t, 719, 3, |_, a| a[0] ^ 0x0719_0000);
        t.stub(235, move |m, a| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { i });
            }
            0x0235_0000
        });
        rec(&mut t, 231, 3, |_, a| 0x0231_0000 ^ a[1]);
        let down = SCRATCH + 0x2f0;
        log_calls(&mut t, 253, log, 0, None, move |m, _, _| m.r32(down));
        log_calls(&mut t, 38, log, 3, None, |_, a, k| a[1] ^ k);
        let m = t.img.rel32_target(f + 0x75);
        for (o, x) in [(0x2au32, 0.25f32), (0x42, 2.0), (0x84, 0.5)] {
            t.set_u32(op(&t, f, o), x.to_bits());
        }
        // (on, down, the level)
        for (k, (on, dn, v)) in [
            (0u32, 1u32, 1.0f32),
            (1, 0, 1.0),
            (1, 1, 1.0),
            (1, 1, 0.75),
            (1, 1, 0.1),
            (1, 0, -3.0),
            (1, 0, 5.0),
            (1, 1, f32::NAN),
            (1, 0, 0.5),
            (1, 0, 2.0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(op(&t, f, 0x2), on);
            t.set_u32(down, dn);
            t.set_u32(op(&t, f, 0x1e), v.to_bits());
            t.set_u32(op(&t, f, 0x6b), 0x5350_0000 + k);
            t.set_u32(op(&t, f, 0x90), 0x9090);
            t.set_u32(op(&t, m, 0x2b), 0x5150_0000 + k);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Row 409: the near points marked in the three DLLs: no points, three
/// (near, far, NaN length), then the count and fade.
#[test]
fn near_points_fades() {
    for (dll, rva) in [(384u16, 0x1e10u32), (421, 0x1e00), (535, 0x1de0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        // the length: the vector's first term, or NaN for a marked one
        t.stub(453, move |m, a| {
            let k = m.r32(log);
            m.w32(log, k + 1);
            for i in 0..4 {
                m.w32(log + 4 + 0x10 * (k % 8) + 4 * i, m.r32(a[0] + 4 * i));
            }
            0x453
        });
        t.stub_st0(453, |m, a| {
            let x = f32::from_bits(m.r32(a[0]));
            if x == 3.0 { f64::NAN } else { f64::from(x) }
        });
        // the count and fade's slots (`count_fade`)
        let node = SCRATCH + 0x800;
        log_calls(&mut t, 434, log + 0x200, 2, None, move |_, _, _| node);
        log_calls(&mut t, 942, log + 0x200, 1, None, |m, a, k| m.r32(a[0]) ^ k);
        let cf = t.img.rel32_target(f + 0xe0);
        t.set_u32(op(&t, cf, 0x79), 30);
        t.set_u32(op(&t, f, 0xa2), 1.0f32.to_bits());
        let points = op(&t, f, 0x4c);
        let flags = op(&t, f, 0xb2);
        let o = SCRATCH + 0x400;
        for (k, n) in [0i32, 3, 4, -1].into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(op(&t, f, 0x40), n.cast_unsigned());
            t.write(o, &varied(0x200, k));
            t.set_u32(o + 0xac, k % 3);
            t.set_u32(o + 0xd0, 0.5f32.to_bits());
            t.set_u32(o + 0xd8, (-1.0f32).to_bits());
            t.set_u32(o + 0xdc, 2.0f32.to_bits());
            for i in 0..4u32 {
                // x - 0.5: 0.25 (near), 3.0 (NaN), 2.5 (far), -0.75
                let x = [0.75f32, 3.5, 3.0, -0.25][i as usize];
                t.set_u32(points - 8 + 0x10 * i, x.to_bits());
                t.set_u32(points - 4 + 0x10 * i, (i as f32).to_bits());
                t.set_u32(points + 0x10 * i, (1.0 - i as f32).to_bits());
                t.set_u32(points + 4 + 0x10 * i, 0.5f32.to_bits());
                t.set_u32(flags + 4 * i, 0x0f0f_0000 + i);
            }
            t.write(node, &varied(0x200, k + 1));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[o], Eax::Checked);
        }
    }
}

/// Row 179: the spark at a point in the five DLLs: blocked by the flags
/// (bit 0, bit 1), no free record, and started (the slots' floats).
#[test]
fn sparks_at_points() {
    for dll in [644u16, 692, 694, 696, 698] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x78c0);
        let log = SCRATCH + 0x6000;
        for s in [314u32, 404] {
            log_calls(&mut t, s, log, 1, None, move |_, a, k| a[0] ^ (s << 20) ^ k);
            t.stub_st0(s, move |m, a| {
                let k = m.r32(log);
                f64::from(f32::from_bits(a[0])) * (0.25 + f64::from(k % 7)) + 1e-9
            });
        }
        let pt = t.img.rel32_target(f + 0x17);
        let recs = SCRATCH + 0x2000;
        t.set_u32(op(&t, pt, 0x37), recs);
        let flags = SCRATCH + 0x2f0;
        t.set_u32(op(&t, f, 0x5), flags);
        let c = SCRATCH + 0x700;
        t.set_u32(op(&t, f, 0x27), c);
        for (o, x) in [(0xd7u32, 0.5f32), (0x14a, -0.25), (0x167, 3.0)] {
            t.set_u32(op(&t, f, o), x.to_bits());
        }
        let p = SCRATCH + 0x300;
        // (flags, records)
        for (k, (fl, n)) in [(1u8, 8u32), (2, 8), (0, 0), (0xfc, 8), (0, 8)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u8(flags, fl);
            t.set_u32(op(&t, pt, 0x0f), n);
            t.set_u32(op(&t, pt, 0x08), 0);
            t.write(recs, &varied(0x74 * 2, k));
            t.set_u32(recs + 0x6c, 0);
            t.write(c, &varied(0x40, k + 1));
            t.write(p, &varied(0x10, k + 2));
            t.set_u32(p, (1.5 * k as f32).to_bits());
            t.set_u32(p + 8, (-2.0 + k as f32).to_bits());
            scrub(&mut t);
            let call = Call {
                ecx: p,
                eax: 0x78c0_0000 + k,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::spark_at_point(g, img, fr))
            });
        }
    }
}

/// Rows 191 and 416: the two-stage fade (one constant in `magic_0083`'s
/// five copies, two in `magic_0091`'s three) in every stage: 0 below and
/// at its time (and NaN), 1 below and at its time, the stages past 1.
#[test]
fn two_stage_fades() {
    for (dll, two) in [
        (83u16, false),
        (86, false),
        (87, false),
        (88, false),
        (89, false),
        (91, true),
        (96, true),
        (97, true),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x66c0);
        let d = if two { 4 } else { 0 };
        let log = SCRATCH + 0x6000;
        log_calls(&mut t, 239, log, 3, None, |_, a, k| a[0] ^ (k << 8));
        t.set_u32(op(&t, f, 0x16), 0.5f32.to_bits());
        t.set_u32(op(&t, f, 0x173 + d), 2.0f32.to_bits());
        t.set_u32(op(&t, f, if two { 0x3c } else { 0x3d }), 4.0f32.to_bits());
        if two {
            t.set_u32(op(&t, f, 0x4a), 1.25f32.to_bits());
        }
        for (i, o) in (0u32..).zip([0x51u32, 0x82, 0xbd, 0xf1, 0x10c, 0x11f, 0x127, 0x137, 0x13f]) {
            t.set_u32(op(&t, f, o + d), (0.125 * (i + 1) as f32 - 0.5).to_bits());
        }
        t.set_u32(op(&t, f, 0x18a + d), 0x0018_a000);
        let task = SCRATCH + 0x300;
        let o = SCRATCH + 0x400;
        let node = SCRATCH + 0x380;
        // (stage, clock before the step)
        for (k, (stage, clock)) in [
            (0u16, 0.0f32),
            (0, 1.5),
            (0, f32::NAN),
            (1, 2.0),
            (1, 3.5),
            (1, 7.0),
            (2, 9.0),
            (0xffff, 9.0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(o, &varied(0x2d0, k));
            t.set_u32(o + 0x2c4, clock.to_bits());
            t.set_u16(o + 0x2c8, stage);
            t.set_u32(task, node);
            t.set_u32(task + 4, o);
            t.write(node, &varied(0x10, k + 3));
            t.set_u32(node + 4, SCRATCH + 0x700);
            t.set_u32(node + 8, SCRATCH + 0x740);
            t.write(SCRATCH + 0x700, &varied(0x80, k));
            t.set_u32(op(&t, f, 0x5b + d), 0xffff_0003 ^ k);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// Row 417: the random part of the three DLLs (both `rand()`s seen in the
/// scales; the three slots log their words).
#[test]
fn random_parts() {
    for (dll, rva) in [(191u16, 0x19a0u32), (415, 0x1980), (534, 0x19b0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        log_calls(&mut t, 458, log, 3, None, |_, a, k| a[0] ^ k);
        log_calls(&mut t, 450, log, 2, None, |_, a, k| a[1] ^ k);
        log_calls(&mut t, 451, log, 2, None, |_, a, k| a[1] ^ (k << 4));
        for (o, x) in [
            (0x25u32, 32768.0f32),
            (0x2d, 3.0),
            (0x71, 16384.0),
            (0x89, 0.75),
            (0x91, 1.5),
            (0x9e, -0.25),
        ] {
            t.set_u32(op(&t, f, o), x.to_bits());
        }
        t.set_u32(op(&t, f, 0xb8), 0x00b8_0000);
        let o = SCRATCH + 0x400;
        let p = SCRATCH + 0x800;
        for k in 0..4u32 {
            t.set_u32(log, 0);
            t.write(o, &varied(0x200, k));
            t.set_u32(o + 0xac, k % 3);
            t.set_u32(o + 0x2c + 0x10 * (k % 3), p);
            t.write(p, &varied(0x100, k + 1));
            for i in 0..3u32 {
                t.set_u32(p + 0xd0 + 4 * i, (i as f32 - 0.5 * k as f32).to_bits());
                t.set_u32(p + 0xe0 + 4 * i, (2.0 * i as f32 + 0.25).to_bits());
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[o], Eax::Checked);
        }
    }
}

/// Row 193: a stream's next block in the five DLLs: past the last block,
/// an open block and a closed one (rate 0 and not), the block's bytes
/// below and at its length, the last block ending the stream; the
/// state at a negative address (the handle's sign word).
#[test]
fn stream_blocks() {
    for (dll, rva) in [
        (150u16, 0x3f60u32),
        (153, 0x1650),
        (295, 0x1650),
        (374, 0x1770),
        (603, 0x3f60),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let lock = SCRATCH + 0x2f0;
        let lg = op(&t, f, 0x27);
        t.set_u32(lg, lock);
        let open = SCRATCH + 0x2f4;
        for (s, n) in [(773u32, 2usize), (775, 2), (779, 2)] {
            log_calls(&mut t, s, log, n, Some(lg), move |m, a, k| {
                if s == 773 { m.r32(open) } else { a[0] ^ k }
            });
        }
        for (s, n) in [(774u32, 5usize), (778, 6)] {
            log_calls(&mut t, s, log, n, Some(lg), move |m, a, k| {
                // the local the original hands over
                let row = log + 0x400 + 0x10 * (k % 8);
                m.w32(row, m.r32(a[2]));
                m.w32(a[2], 0x7777_0000 + k);
                a[3] ^ k
            });
        }
        log_calls(&mut t, 776, log, 1, None, |_, a, _| a[0] << 8);
        log_calls(&mut t, 777, log, 1, None, |_, a, _| a[0] & 0xff);
        let tbl = SCRATCH + 0x1000;
        let a = SCRATCH + 0x800;
        // (state at, block, bytes, open, rate, count)
        for (k, (s, block, bytes, op_, rate, len)) in [
            (SCRATCH + 0x300, 3u16, 0u16, 0u32, 0u8, 0x200u16),
            (SCRATCH + 0x300, 0, 0, 1, 0, 0x200),
            (SCRATCH + 0x300, 1, 0x100, 1, 0, 0x200),
            (SCRATCH + 0x300, 2, 0x180, 0, 0, 0x200),
            (SCRATCH + 0x300, 2, 0x180, 0, 7, 0x8000),
            (SCRATCH + 0x300, 1, 0x40, 0, 0x30, 0x100),
            (0x9000_0000, 0, 0, 1, 0, 0x200),
            (0x9000_0000, 5, 0, 1, 0, 0x200),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(lock, 0x0bad_0000 + k);
            t.set_u32(open, op_);
            t.write(s, &varied(8, k));
            t.set_u16(s, block);
            t.set_u16(s + 2, bytes);
            t.write(tbl, &varied(0x100, k));
            t.set_u16(tbl + 6, 3);
            for i in 0..3u32 {
                let e = tbl + 8 * (i + 2);
                t.set_u16(e, (0x40 + 0x20 * i) as u16);
                t.set_u16(e + 2, len);
                // the data: `+4` the rate's offset from it
                let d = tbl + 0x40 + 0x20 * i;
                t.set_u32(d + 4, 8);
                t.set_u8(d + 0xc, rate);
            }
            t.write(a, &varied(0x40, k + 1));
            scrub(&mut t);
            let c = Call {
                ecx: s,
                edx: tbl,
                ..Call::default()
            };
            t.check(f, c, &[a], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::stream_block(g, img, fr))
            });
        }
    }
}

/// Row 419: the summon's loading and fade-in frame in the three DLLs:
/// each step waiting and done, the fade's two ranges and their bounds.
#[test]
fn summon_fade_ins() {
    for (dll, rva) in [(701u16, 0x1320u32), (702, 0x1320), (703, 0x1310)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let answers = SCRATCH + 0x2f0;
        // slots 225, 227 and 347 answer from the plan; the rest log
        for (i, s) in (0u32..).zip([225u32, 227, 347]) {
            log_calls(&mut t, s, log, 1, None, move |m, _, _| {
                m.r32(answers + 4 * i)
            });
        }
        for (s, n) in [(226u32, 2usize), (239, 3), (220, 3), (349, 2), (348, 6)] {
            log_calls(&mut t, s, log, n, None, |_, a, k| a[0] ^ (k << 8));
        }
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        for (o, v) in [
            (0x10u32, 0x5150_0000u32),
            (0xd8, 0x5250_0000),
            (0xed, 0xeded),
            (0xf8, 0xf8f8),
        ] {
            t.set_u32(w(&t, o), v);
        }
        // (A, B, C, D, the answers of 225, 227 and 347)
        for (k, (a, b, c, d, ans)) in [
            (0u32, 0u32, 0u32, -1i32, [1u32, 1, 1]),
            (0, 0, 0, -1, [0, 0, 1]),
            (1, 0, 0, -1, [0, 1, 0]),
            (1, 1, 0, -1, [0, 1, 1]),
            (1, 1, 1, -1, [0, 0, 0xfe]),
            (1, 1, 1, 0, [0, 0, 0]),
            (1, 1, 1, 1, [0, 0, 0]),
            (1, 1, 1, 15, [0, 0, 0]),
            (1, 1, 1, 16, [0, 0, 0]),
            (1, 1, 1, 45, [0, 0, 0]),
            (1, 1, 1, 46, [0, 0, 0]),
            (1, 1, 1, 22, [0, 0, 0]),
            (0, 1, 1, 7, [0, 0, 0]),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(w(&t, 0x2), a);
            t.set_u32(w(&t, 0x2f), b);
            t.set_u32(w(&t, 0x5a), c);
            t.set_u32(w(&t, 0x88), d.cast_unsigned());
            for (i, x) in (0u32..).zip(ans) {
                t.set_u32(answers + 4 * i, x);
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Row 454: the sprite fade node of both DLLs: not passed, the flag
/// clear, each counter running (in, held, out), none left (with and
/// without bit 1: the node leaves its parent), the alpha at or below
/// the bound and above it (the packet written, slot 581).
#[test]
fn sprite_fade_nodes() {
    for dll in [202u16, 273] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x12c0);
        let thunk = 0x1480;
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        log_calls(&mut t, 245, log, 1, None, |m, a, k| {
            m.w32(a[0], 0x0245_0000 + k);
            0x245
        });
        log_calls(&mut t, 581, log, 4, None, |m, a, k| m.r32(a[0] + 8) ^ k);
        t.set_u32(op(&t, f, 0x92), 0.75f32.to_bits());
        t.set_u32(op(&t, f, 0xbf), 1.0f32.to_bits());
        t.set_u32(op(&t, f, 0xe9), 8.0f32.to_bits());
        let buf = SCRATCH + 0x3000;
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let parent = SCRATCH + 0x1800;
        // (ring byte, flags, in, held, out, the level byte)
        for (k, (byte, bits, a, b, c, level)) in [
            (3u8, 1u8, 2u16, 0u16, 0u16, 0x80u8),
            (4, 0xfe, 2, 0, 0, 0x80),
            (4, 1, 3, 5, 5, 0x80),
            (4, 1, 1, 5, 5, 0x80),
            (4, 1, 0, 2, 5, 0x80),
            (4, 1, 0, 2, 5, 4),
            (4, 1, 0, 0, 4, 0xff),
            (4, 1, 0, 0, 1, 0xff),
            (4, 3, 0, 0, 0, 0xff),
            (4, 1, 0, 0, 0, 0xff),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.write(r, &varied(0x40, k));
            set_ring(&mut t, s, r, (3, byte), k % 2, vt, k);
            t.set_u32(r + 0x1c, 4.0f32.to_bits());
            t.set_u32(r + 0x20, 5.0f32.to_bits());
            t.set_u16(r + 0x24, b);
            t.set_u16(r + 0x26, a);
            t.set_u16(r + 0x28, c);
            t.set_u8(r + 0x2a, level);
            t.set_u8(r + 0x2b, bits);
            t.set_u32(r + 0x2c, (0.5 + k as f32).to_bits());
            t.write(parent, &varied(0x40, k + 3));
            t.set_u32(r + 0x14, parent);
            t.set_u32(r + 4, parent + 0x20);
            t.set_u32(r + 8, parent + 0x20);
            t.write(buf, &varied(0x120, k));
            t.set_u32(op(&t, f, 0x10b), buf);
            scrub(&mut t);
            // the two words of the sprite the original never writes
            let ebp = ENTRY_ESP - 4;
            t.set_u32(ebp - 0x34, 0);
            t.set_u32(ebp - 0x2c, 0);
            let call = Call {
                ecx: s,
                eax: 0x12c0_0000 + k,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::sprite_fade_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Row 420: a spline's second derivatives in the three DLLs, for knot
/// counts 0..7 (every loop run, skipped, and the short systems) and
/// each component.
#[test]
fn spline_moments() {
    for (dll, rva) in [(432u16, 0x4ef0u32), (434, 0xe0b0), (610, 0x5420)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let (p, x) = (SCRATCH + 0x1000, SCRATCH + 0x400);
        let (m, h, u) = (SCRATCH + 0x600, SCRATCH + 0x700, SCRATCH + 0x800);
        for (k, n) in [0u32, 1, 2, 3, 4, 5, 7].into_iter().enumerate() {
            let k = k as u32;
            for (i, base) in (0u32..).zip([m, h, u]) {
                // the rows from one word before (index -1 when n is 0)
                t.write(base - 0x10, &varied(0x60, k + i));
            }
            for i in 0..8u32 {
                let xi = 0.75 * i as f32 + 0.125 * (i * i) as f32 + k as f32;
                t.set_u32(x + 4 * i, xi.to_bits());
                for c in 0..4u32 {
                    let y = ((i * 7 + c * 3 + k) % 11) as f32 * 0.5 - 2.0;
                    t.set_u32(p + 0x10 * i + 4 * c, y.to_bits());
                }
            }
            let c = Call {
                ecx: p,
                edx: x,
                ..Call::default()
            };
            t.check(f, c, &[m, h, u, k % 4, n], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::spline_moments(g, img, fr))
            });
        }
    }
}

/// Row 421: a spline set up through points in the three DLLs, for point
/// counts 0..6.
#[test]
fn spline_setups() {
    for (dll, rva) in [(432u16, 0x4b00u32), (434, 0xdcc0), (610, 0x5030)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let q = SCRATCH + 0x1000;
        let (tk, w) = (SCRATCH + 0x400, SCRATCH + 0x500);
        let ms = [SCRATCH + 0x600, SCRATCH + 0x680, SCRATCH + 0x700];
        for (k, n) in [0u32, 1, 2, 3, 4, 6].into_iter().enumerate() {
            let k = k as u32;
            t.write(tk - 0x10, &varied(0x50, k));
            t.write(w - 0x10, &varied(0x70, k + 1));
            for (i, m) in (0u32..).zip(ms) {
                t.write(m - 0x10, &varied(0x50, k + 2 + i));
            }
            for i in 0..8u32 {
                for c in 0..4u32 {
                    let y = ((i * 5 + c * 3 + k) % 9) as f32 * 0.75 - 2.0 + i as f32;
                    t.set_u32(q + 0x10 * i + 4 * c, y.to_bits());
                }
            }
            scrub(&mut t);
            let c = Call {
                ecx: w,
                edx: q,
                ..Call::default()
            };
            t.check(
                f,
                c,
                &[tk, ms[0], ms[1], ms[2], n],
                Eax::Checked,
                |g, img, fr| Ret::eax(s7_misc::spline_setup(g, img, fr)),
            );
        }
    }
}

/// Row 467: the shake node of both DLLs: not passed, bit 0 clear, the
/// timer at 0 (with and without bit 1: the node leaves its parent), and
/// running with each pair of signs (and a NaN timer).
#[test]
fn shake_nodes() {
    for (dll, rva, thunk) in [(513u16, 0x4510u32, 0x46c0u32), (577, 0x65b0, 0x6760)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        let signs = SCRATCH + 0x2f0;
        log_calls(&mut t, 324, log, 0, None, move |m, _, k| {
            m.r32(signs) >> (2 * (k % 2))
        });
        log_calls(&mut t, 307, log, 0, None, |_, _, k| 0x307 + k);
        t.stub_st0(307, |_, _| 0.625);
        log_calls(&mut t, 250, log, 2, None, |_, a, k| a[0] ^ a[1] ^ k);
        t.set_u32(op(&t, f, 0x6a), 0.5f32.to_bits());
        t.set_u32(op(&t, f, 0x96), 0.25f32.to_bits());
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let parent = SCRATCH + 0x1800;
        // (ring byte, flags, timer, sign bits)
        for (k, (byte, bits, timer, sg)) in [
            (3u8, 1u8, 2.0f32, 0u32),
            (4, 0xfe, 2.0, 0),
            (4, 1, 0.0, 0),
            (4, 3, 0.0, 0),
            (4, 3, -0.0, 0),
            (4, 1, 2.0, 0),
            (4, 1, 2.0, 2),
            (4, 1, 2.0, 8),
            (4, 1, 3.5, 0xa),
            (4, 1, f32::NAN, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(signs, sg);
            t.write(r, &varied(0x48, k));
            set_ring(&mut t, s, r, (3, byte), k % 2, vt, k);
            t.set_u8(r + 0x40, bits);
            for (i, at) in (0u32..).zip([0x1cu32, 0x20, 0x24, 0x28, 0x30, 0x38, 0x3c]) {
                t.set_u32(r + at, (0.5 + 0.25 * i as f32 + k as f32).to_bits());
            }
            t.set_u32(r + 0x2c, timer.to_bits());
            t.write(parent, &varied(0x40, k + 3));
            t.set_u32(r + 0x14, parent);
            t.set_u32(r + 4, parent + 0x20);
            t.set_u32(r + 8, parent + 0x20);
            scrub(&mut t);
            let call = Call {
                ecx: s,
                eax: 0x4510_0000 + k,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::shake_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Row 320: the sprite system step on the DLL's globals in the four
/// DLLs: no sprites, a sprite that ends (released and cleared), a dead
/// one, and held ones with gravity and the speed cap on and off. The
/// original's `eax` after the square root is the CRT's (junk).
#[test]
fn sprite_systems_global() {
    for (dll, rva) in [
        (206u16, 0x2d30u32),
        (505, 0x2b70),
        (641, 0x2ee0),
        (669, 0x3360),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        rec(&mut t, 248, 3, |m, a| {
            for i in 0..3 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x248
        });
        let table = SCRATCH + 0x3400;
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
        let lock = SCRATCH + 0x7f0;
        let lg = op(&t, f, 0x58);
        t.set_u32(lg, lock);
        log_calls(&mut t, 779, SCRATCH + 0x6000, 2, Some(lg), |_, _, _| 0x779);
        let parts = SCRATCH + 0x1000;
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        for (k, (count, gravity, cap)) in [
            (0u32, 0u32, 0u32),
            (4, 1, 0),
            (4, 0, 1),
            (4, 1, 1),
            (3, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(lock, 0x5a5a_0000 + k);
            t.set_u32(w(&t, 0x16), count);
            t.set_u32(w(&t, 0x10), parts);
            t.set_u32(w(&t, 0x97), gravity);
            t.set_u32(w(&t, 0xa7), (-0.5f32).to_bits());
            t.set_u32(w(&t, 0xb2), cap);
            t.set_u32(w(&t, 0x121), 2.0f32.to_bits());
            // (live, held, the velocity)
            for (i, (live, hold, v)) in [
                (1u32, 0u32, [1.0f32, 2.0, 2.0]),
                (0, 1, [1.0, 1.0, 1.0]),
                (1, 1, [3.0, 4.0, 0.0]),
                (1, 1, [0.5, 0.25, -0.5]),
            ]
            .into_iter()
            .enumerate()
            {
                let p = parts + 0x94 * i as u32;
                t.write(p, &varied(0x94, i as u32 + k));
                t.set_u32(p, live * (0x100 + i as u32));
                t.set_u32(p + 0x2c, hold);
                t.set_u32(p + 0x28, 1);
                t.set_u32(p + 0x30, 0x0300_0000 + i as u32);
                for (j, x) in (0u32..).zip(v) {
                    t.set_u32(p + 0x14 + 4 * j, x.to_bits());
                }
            }
            let eax = if cap != 0 { Eax::Junk } else { Eax::Checked };
            let c = Call {
                eax: 0x0eaa_0000 + k,
                ..Call::default()
            };
            scrub(&mut t);
            t.check(f, c, &[], eax, |g, img, fr| {
                Ret::eax(s7_misc::sprites_step_global(g, img, fr).eax)
            });
        }
    }
}

/// Row 350: the thunder node of the three DLLs: not passed, passed with
/// bit 0 of `+0x54` set and clear, the record's slot on and off; every
/// slot logs what its pointers see.
#[test]
fn thunder_nodes() {
    for (dll, rva, thunk) in [
        (387u16, 0x7ca0u32, 0x7e40u32),
        (578, 0x64a0, 0x6690),
        (649, 0x6540, 0x6730),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        // the vector: scaled, then normalised (both write it)
        log_calls(&mut t, 233, log, 3, None, |m, a, k| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x233 + k
        });
        log_calls(&mut t, 232, log, 2, None, |m, a, k| {
            for i in 0..4 {
                let x = m.r32(a[1] + 4 * i) ^ 0x10;
                m.w32(a[0] + 4 * i, x);
            }
            0x232 + k
        });
        // the slots with pointers: their words and what they point at
        t.stub(677, move |m, a| {
            let row = log + 0x400;
            for i in 0..4 {
                m.w32(row + 4 * i, a[i as usize]);
                m.w32(row + 0x10 + 4 * i, m.r32(a[3] + 4 * i));
            }
            0x677
        });
        t.stub(679, move |m, a| {
            let row = log + 0x440;
            for i in 0..6 {
                m.w32(row + 4 * i, m.r32(a[0] + 4 * i));
            }
            m.w32(row + 0x18, m.r32(m.r32(a[0] + 0x14)));
            0x679
        });
        t.stub(680, move |m, a| {
            let row = log + 0x480;
            for (i, x) in (0u32..).zip(&a[..11]) {
                m.w32(row + 4 * i, *x);
            }
            m.w32(row + 0x2c, m.r32(a[2]));
            m.w32(row + 0x30, m.r32(a[2] + 4));
            0x680 ^ a[3]
        });
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let off = op(&t, f, 0x12d);
        for (k, (byte, bits, on)) in [
            (3u8, 1u8, 0u32),
            (4, 1, 0),
            (4, 0xfe, 0),
            (4, 0xff, 1),
            (4, 0, 2),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.write(log, &varied(0x500, k));
            t.set_u32(log, 0);
            t.write(r, &varied(0x60, k));
            set_ring(&mut t, s, r, (3, byte), k % 2, vt, k);
            t.set_u8(r + 0x54, bits);
            for (i, at) in (0u32..).zip([0x40u32, 0x44, 0x48, 0x4c, 0x50]) {
                t.set_u32(r + at, SCRATCH + 0x2000 + 0x100 * i);
            }
            t.write(SCRATCH + 0x2000, &varied(0x500, k + 1));
            t.set_u32(off, on);
            scrub(&mut t);
            let call = Call {
                ecx: s,
                eax: 0x7ca0_0000 + k,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::thunder_node(g, img, fr))
            });
            // and through its thunk (`ecx` from the stack)
            t.check_entry(t.at(thunk), Call::default(), &[s], Eax::Checked);
        }
    }
}

/// Row 323: the spline object set up in the four DLLs: one and four
/// points, straight and reversed, the handle open and not; the lock
/// word and the local slot 907 reads are logged.
#[test]
fn spline_objects() {
    for (dll, rva) in [
        (515u16, 0x2dc0u32),
        (611, 0x2de0),
        (667, 0x2c70),
        (668, 0x2c90),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let lock = SCRATCH + 0x2f0;
        let lg = op(&t, f, 0xf8);
        t.set_u32(lg, lock);
        let open = SCRATCH + 0x2f4;
        log_calls(&mut t, 772, log, 2, Some(lg), |_, a, k| a[0] ^ (k << 8));
        t.stub_edx(772, |_, a| a[0].rotate_left(4));
        log_calls(&mut t, 773, log, 2, Some(lg), move |m, _, _| m.r32(open));
        log_calls(&mut t, 776, log, 1, Some(lg), |_, a, _| a[0] << 4);
        log_calls(&mut t, 779, log, 2, Some(lg), |_, a, k| a[1] ^ k);
        log_calls(&mut t, 942, log, 1, None, |_, a, _| a[0] ^ 0x0942_0000);
        t.stub(907, move |m, a| {
            let row = log + 0x500;
            for i in 0..5 {
                m.w32(row + 4 * i, a[i as usize]);
            }
            m.w32(row + 0x14, m.r32(a[2]));
            m.w32(row + 0x18, m.r32(m.r32(lg)));
            0x907
        });
        log_calls(&mut t, 269, log, 3, None, |_, a, k| a[2] ^ k);
        let o = SCRATCH + 0x1000;
        let p = SCRATCH + 0x1800;
        let mdl = SCRATCH + 0x2000;
        for (k, (n, rev, op_)) in [(1u32, 0u32, 0u32), (4, 0, 1), (4, 1, 0), (3, 7, 1)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(lock, 0x0bad_0000 + k);
            t.set_u32(open, op_);
            t.write(o, &varied(0xc0, k));
            t.write(p, &varied(0x80, k + 1));
            t.write(mdl, &varied(0x100, k + 2));
            t.set_u16(mdl + 0x10, 0x40);
            // the data: `+4` the rate's offset from it
            t.set_u32(mdl + 0x44, 0x20);
            t.set_u8(mdl + 0x40 + 0x24, (0x30 + k) as u8);
            let args = [
                n,
                0xabcd_1234,
                0x55aa_0077,
                rev,
                mdl,
                1.5f32.to_bits(),
                (-2.0f32).to_bits(),
                0.25f32.to_bits(),
                0x8c8c_0000 + k,
                0x9090_0000 + k,
                0x1234_5600 + k,
            ];
            scrub(&mut t);
            let c = Call {
                ecx: o,
                edx: p,
                ..Call::default()
            };
            t.check(f, c, &args, Eax::Checked, |g, img, fr| {
                Ret::eax(s7_misc::spline_object(g, img, fr))
            });
        }
    }
}

/// Row 301: the aimed emitter of the four DLLs: targets in each quarter
/// (the yaw brought back from above and below and kept), the task list
/// empty and not; the matrix slots see the frame's matrix.
#[test]
fn aimed_emitters() {
    for (dll, rva) in [
        (284u16, 0x3a40u32),
        (285, 0x26d0),
        (286, 0x36b0),
        (287, 0x26d0),
    ] {
        aimed_in(
            dll,
            rva,
            (0x24c, 0xc),
            [0x1be, 0x1c4, 0x1d7],
            s7_misc::aimed_emitter,
        );
    }
}

/// Row 196: the aimed emitter that keeps its angles, in the five DLLs.
#[test]
fn aimed_emitters_angles() {
    for (dll, rva) in [
        (18u16, 0x31e0u32),
        (19, 0x1ce0),
        (307, 0xae00),
        (380, 0x30b0),
        (432, 0x4590),
    ] {
        aimed_in(
            dll,
            rva,
            (0x358, 0xd0),
            [0x1e7, 0x1ed, 0x200],
            s7_misc::aimed_emitter_angles,
        );
    }
}

/// The aimed emitter's cases: `(target, position)` the fields, `[take,
/// head, tail]` the operands.
fn aimed_in(
    dll: u16,
    rva: u32,
    (target, pos): (u32, u32),
    [take_at, head, tail]: [u32; 3],
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> u32,
) {
    {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        log_calls(&mut t, 235, log, 1, None, |m, a, k| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { k + i });
            }
            0x235
        });
        for s in [328u32, 329] {
            log_calls(&mut t, s, log, 3, None, move |m, a, k| {
                let v = m.r32(a[1] + 4 * (k % 16)) ^ a[2];
                m.w32(a[0] + 4 * (k % 16), v);
                s
            });
        }
        log_calls(&mut t, 229, log, 3, None, |m, a, _| {
            for i in 0..4u32 {
                let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x229
        });
        let take = t.img.rel32_target(f + take_at);
        let e = SCRATCH + 0x1000;
        let node = SCRATCH + 0x400;
        let ring = [SCRATCH + 0x480, SCRATCH + 0x4c0];
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // (dx, dy, dz, the list's head)
        for (k, (dx, dy, dz, listed)) in [
            (1.0f32, 0.5f32, 1.0f32, false),
            (-1.0, 2.0, 0.25, true),
            (-0.5, -1.0, -3.0, false),
            (2.0, 0.0, -0.75, true),
            (0.0, 1.0, 0.0, false),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.write(e, &varied(0x390, k));
            for (i, x) in (0u32..).zip([dx, dy, dz]) {
                t.set_u32(e + pos + 4 * i, (0.5 * k as f32).to_bits());
                t.set_u32(e + target + 4 * i, (0.5 * k as f32 + x).to_bits());
            }
            t.write(node, &varied(0x20, k));
            t.set_u32(node, ring[0]);
            t.set_u32(node + 4, ring[1]);
            t.set_u16(op(&t, take, 0x04), 3);
            t.set_u32(op(&t, take, 0x17), node);
            t.set_u32(w(&t, head), if listed { SCRATCH + 0x500 } else { 0 });
            t.set_u32(w(&t, tail), SCRATCH + 0x540);
            t.write(SCRATCH + 0x500, &varied(0x80, k + 1));
            scrub(&mut t);
            let c = Call {
                ecx: e,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(9.0), Xmm::f32(1.5 + k as f32)]);
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(port(g, img, fr))
            });
        }
    }
}

/// Row 798: the two-stage summon set-up task of both DLLs: stage 0 below
/// and at its time (both model ways), stage 1 waiting on each flag and
/// done (the nodes leave, with and without parents), other stages.
#[test]
fn summon_setup_tasks() {
    for dll in [439u16, 682] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x6a90);
        let log = SCRATCH + 0x6000;
        let model = SCRATCH + 0x7e0;
        log_calls(&mut t, 390, log, 1, None, move |m, _, _| m.r32(model));
        log_calls(&mut t, 534, log, 1, None, |_, _, k| k % 2);
        log_calls(&mut t, 325, log, 2, None, |m, a, k| {
            m.w32(a[0], m.r32(a[1]) ^ k);
            0x325
        });
        log_calls(&mut t, 239, log, 3, None, |_, a, k| a[1] ^ k);
        // the particle's slots (`sb_nodes::particle_node`)
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        let pn = t.img.rel32_target(f + 0x1fa);
        let start = t.img.rel32_target(pn + 0x88);
        t.set_u32(op(&t, start, 0x04), SCRATCH + 0x2f0);
        t.set_u32(op(&t, f, 0x1e), 0.5f32.to_bits());
        t.set_u32(op(&t, f, 0xd5), 1.0f32.to_bits());
        t.set_u32(op(&t, f, 0x1c1), SCRATCH + 0x900);
        t.set_u32(op(&t, f, 0x210), 0x0210_0000);
        let list = op(&t, f, 0xe6);
        let task = SCRATCH + 0x300;
        let node = SCRATCH + 0x380;
        let o = SCRATCH + 0x1000;
        let user = SCRATCH + 0x800;
        // (stage, clock, the flags, the model, parents)
        for (k, (stage, clock, fl, id, parents)) in [
            (0u32, 0.0f32, 0u8, 0x1071u32, false),
            (0, 0.75, 0, 0x1071, false),
            (0, 2.0, 0, 0x2000, false),
            (0, 2.0, 0, 0x2000, false),
            (1, 0.0, 4, 0, false),
            (1, 0.0, 0x40, 0, false),
            (1, 0.0, 0xff, 0, true),
            (1, 0.0, 0xff, 0, false),
            (2, 0.0, 0, 0, false),
            (0xff, 0.0, 0, 0, false),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(log, 0);
            t.set_u32(model, id);
            t.write(o, &varied(0x600, k));
            t.set_u32(o + 0x568, clock.to_bits());
            t.set_u32(o + 0x574, stage);
            t.set_u32(o + 0x578, user);
            t.write(user, &varied(0x60, k));
            t.set_u32(user + 0x50, SCRATCH + 0x880);
            t.set_u8(o + 0x108, fl);
            t.set_u8(o + 0x208, if k == 5 { 0 } else { fl });
            for at in [0x220u32, 0x278, 0x2d0, 0x24c, 0x2a4, 0x41c, 0x57c] {
                let par = SCRATCH + 0x4000 + 2 * at;
                t.set_u32(o + at + 0x14, if parents { par } else { 0 });
                t.set_u32(o + at + 4, par + 0x40);
                t.set_u32(o + at + 8, par + 0x40);
            }
            t.write(SCRATCH + 0x4000, &varied(0x1000, k + 1));
            t.set_u32(task, node);
            t.set_u32(task + 4, o);
            t.write(node, &varied(0x10, k + 3));
            t.set_u32(node + 4, SCRATCH + 0x700);
            t.set_u32(node + 8, SCRATCH + 0x740);
            t.write(SCRATCH + 0x700, &varied(0x80, k));
            // the list: empty, or holding a node
            t.write(list, &varied(0x1c, k + 5));
            t.set_u32(list + 0x10, if k % 2 == 0 { list } else { SCRATCH + 0x5c0 });
            t.write(SCRATCH + 0x5c0, &varied(0x20, k + 1));
            t.write(SCRATCH + 0x900, &varied(0x80, k + 4));
            // the particle's parent node (a DLL node): its list empty
            let pl = op(&t, f, 0x1dd);
            t.write(pl, &varied(0x1c, k + 6));
            t.set_u32(pl + 0x10, pl);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// Row 327: the model items node of the four DLLs: not passed, passed
/// with both groups empty, one empty, and both holding items; the
/// slots see the frame's record.
#[test]
fn model_items_nodes() {
    for (dll, rva) in [
        (42u16, 0x4c90u32),
        (152, 0x4940),
        (533, 0x4950),
        (601, 0x4880),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let vt = ring_handler(&mut t, log);
        log_calls(&mut t, 245, log, 1, None, |m, a, k| {
            m.w32(a[0], 0x2450_0000 + k);
            0x245
        });
        log_calls(&mut t, 229, log, 3, None, |m, a, k| {
            for i in 0..4 {
                m.w32(a[0] + 4 * i, m.r32(a[2] + 4 * i) ^ k);
            }
            0x229
        });
        t.stub(325, move |m, a| {
            let k = m.r32(log);
            m.w32(log, k + 1);
            let row = log + 0x400 + 0x80 * (k % 8);
            for i in 0..0x20 {
                m.w32(row + 4 * i, m.r32(a[0] + 4 * i));
            }
            m.w32(a[0], m.r32(a[1]));
            0x325
        });
        t.stub(231, move |m, a| {
            let k = m.r32(log);
            m.w32(log, k + 1);
            let row = log + 0x800 + 0x40 * (k % 8);
            for i in 0..0x10 {
                m.w32(row + 4 * i, m.r32(a[2] + 4 * i));
            }
            m.w32(a[0], a[1] ^ k);
            0x231
        });
        t.set_u32(op(&t, f, 0x20a), SCRATCH + 0x2f0);
        let s = SCRATCH + 0x300;
        let r = SCRATCH + 0x1000;
        let m = SCRATCH + 0x1400;
        let items = SCRATCH + 0x1800;
        for (k, (byte, a, b)) in [
            (3u8, 1u16, 1u16),
            (4, 0, 0),
            (4, 2, 0),
            (4, 0, 1),
            (4, 3, 2),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.write(log, &varied(0xc00, k));
            t.set_u32(log, 0);
            t.write(r, &varied(0x40, k));
            set_ring(&mut t, s, r, (3, byte), k % 2, vt, k);
            t.set_u16(r + 0x20, (0x30 + k) as u16);
            t.set_u32(r + 0x1c, m);
            t.write(m, &varied(0x40, k + 1));
            t.set_u32(m + 0x24, items);
            t.set_u32(m + 0x28, items + 0x100);
            t.set_u16(m + 0x2c, a);
            t.set_u16(m + 0x2e, b);
            t.write(items, &varied(0x200, k + 2));
            scrub(&mut t);
            let c = Call {
                eax: 0x4c90_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[s], Eax::Checked);
        }
    }
}

/// Row 328: the model's sound record in the four DLLs: the handle open
/// (slots 819 and 870) and not (the flags, slot 871); the slots log the
/// record, the matrix and the lock word.
#[test]
fn model_sound_records() {
    for (dll, rva) in [
        (515u16, 0x4a40u32),
        (611, 0x4a70),
        (667, 0x4810),
        (668, 0x4830),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let log = SCRATCH + 0x6000;
        let lock = SCRATCH + 0x2f0;
        let lg = op(&t, f, 0x167);
        t.set_u32(lg, lock);
        let open = SCRATCH + 0x2f4;
        log_calls(&mut t, 298, log, 1, None, |m, a, k| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, k << 8 | i);
            }
            0x298
        });
        log_calls(&mut t, 231, log, 3, None, |m, a, k| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, m.r32(a[2] + 4 * i) ^ a[1] ^ k);
            }
            0x231
        });
        log_calls(&mut t, 772, log, 2, Some(lg), |_, a, k| a[0] ^ (k << 8));
        t.stub_edx(772, |_, a| a[0].rotate_left(4));
        log_calls(&mut t, 773, log, 2, Some(lg), move |m, _, _| m.r32(open));
        log_calls(&mut t, 776, log, 1, Some(lg), |_, a, _| a[0] << 4);
        log_calls(&mut t, 777, log, 1, Some(lg), |_, a, _| a[0] ^ 0x0100);
        for s in [819u32, 870, 871] {
            t.stub(s, move |m, a| {
                let row = log + 0x400 + 0x100 * (s % 4);
                let n = match s {
                    819 => 4,
                    870 => 2,
                    _ => 5,
                };
                for (i, x) in (0u32..).zip(a.iter().take(n)) {
                    m.w32(row + 4 * i, *x);
                }
                m.w32(row + 0x14, m.r32(m.r32(lg)));
                if s != 870 {
                    for i in 0..0x12u32 {
                        m.w32(row + 0x18 + 4 * i, m.r32(a[3] + 4 * i));
                    }
                    let mtx = m.r32(a[3] + 0x20);
                    for i in 0..16u32 {
                        m.w32(row + 0x60 + 4 * i, m.r32(mtx + 4 * i));
                    }
                }
                s
            });
        }
        let ga = op(&t, f, 0x27);
        t.set_u32(ga, SCRATCH + 0x700);
        t.set_u32(SCRATCH + 0x700, SCRATCH + 0x740);
        t.set_u32(SCRATCH + 0x760, SCRATCH + 0x1000);
        let c = SCRATCH + 0x1000 + 0x8e0;
        t.set_u32(c + 0x1c, SCRATCH + 0x800);
        t.set_u32(op(&t, f, 0xd4), SCRATCH + 0x780);
        t.set_u32(SCRATCH + 0x780, 0x0780_1234);
        for (o, x) in [(0x55u32, 1.5f32), (0x62, -2.0), (0x6a, 0.25), (0x84, 3.0)] {
            t.set_u32(op(&t, f, o), x.to_bits());
        }
        t.set_u32(op(&t, f, 0x77), 0x0077_0000);
        for (k, op_) in [0u32, 1, 0, 2].into_iter().enumerate() {
            let k = k as u32;
            t.write(log, &varied(0x800, k));
            t.set_u32(log, 0);
            t.set_u32(lock, 0x0bad_0000 + k);
            t.set_u32(open, op_);
            t.write(SCRATCH + 0x800, &varied(0x40, k + 1));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}
