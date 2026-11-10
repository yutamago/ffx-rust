//! Round 42's summon ports on the summon stubs (`ffx_magic::r42`) against
//! each original. Needs `FFX_GAME_DIR`.

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
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/summon_stubs.rs"]
mod summon_stubs;

use ffx_magic::{Ret, r42};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;
use summon_stubs::{all_stubs, prime};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// `magic_0533`'s summon task offsets in `magic_0152`'s copy (as
/// `r42::MAP_0152_8980`).
const MAP_0152: &[(u32, u32)] = &[
    (0x0, 0x0),
    (0x2e1, 0x2e8),
    (0x2e4, 0x301),
    (0x3c9, 0x3e0),
    (0x3f1, 0x40e),
    (0x6a4, 0x6a4),
    (0xa38, 0xa39),
    (0xa57, 0xa57),
    (0xa9f, 0xa99),
    (0xaa3, 0xaa2),
    (0xac4, 0xac8),
    (0xafc, 0xb06),
    (0xaff, 0xb0b),
    (0xb09, 0xb1c),
    (0xb1d, 0xb49),
    (0xc69, 0xced),
];

fn mapped(map: &[(u32, u32)], o: u32) -> u32 {
    let mut r = o;
    for &(start, here) in map {
        if o >= start {
            r = o - start + here;
        }
    }
    r
}

/// `magic_0152` 0x10008980 (`r42::summon_task_0152`, `magic_r35.rs`'s
/// `summon_task_0533` on the copy's layout): every state of the DLL's
/// state global (waiting and going on), slot 520's answer both ways (its
/// test is in state 0x3d here), state 0x47's model flag both ways (the
/// screen object set-up after it), the fade byte above and below 2, the
/// count reaching 0x80, 10000, the states past the table and a negative
/// one.
#[test]
fn summon_task_0152() {
    let Some(mut t) = twin(152) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x8980);
    let w = |t: &Twin, o: u32| t.img.word(f + mapped(MAP_0152, o));
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 520, 0, move |m, _| m.r32(answer));
    rec(&mut t, 557, 0, |_, _| 0x5571);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    for (s, n) in [(228u32, 2usize), (257, 2), (296, 2), (687, 3), (691, 7)] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a[0] ^ a[1].rotate_left(7)
        });
    }
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (307, 0.625)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    rec(&mut t, 307, 0, |_, _| 0x307);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let gs = w(&t, 0x10);
    // state 0x47's flag byte and slot 228's byte (the copy's own operands)
    let flag = t.img.word(f + 0xc90);
    let b228 = t.img.word(f + 0x2e4);
    // (state, `R + 0x20`, `R + 0x1c`, slot 520's answer, the byte `R +
    // 0x390`, `R + 0x310`)
    let cases: [(u32, u32, f32, u32, u8, u32); 45] = [
        (0, 0, 0.0, 0, 0, 0),
        (1, 3, 0.0, 0, 0, 0),
        (1, 0x27, 0.0, 0, 0, 0),
        (0x51, 0, 0.0, 0, 0, 0),
        (0x5a, 0, 0.0, 0, 0, 0),
        (0x5b, 0, 0.0, 0, 0, 0),
        (0x5c, 0, -100.0, 0, 0, 0),
        (0x5c, 0, 0.0, 0, 0, 0),
        (0x5c, 0, 1000.0, 0, 0, 0),
        (0xa, 0, 0.0, 0, 0, 0),
        (0xb, 0, 0.0, 0, 0, 0),
        (0x64, 0, 0.0, 0x024b_0005, 5, 3),
        (0x64, 0, 0.0, 7, 1, 0x7e),
        (0x65, 0, 0.0, 0, 9, 3),
        (0x65, 0, 0.0, 0, 2, 0x7e),
        (0x65, 0, 0.0, 0, 0, 3),
        (0x66, 0, 0.0, 0, 0, 0x7f),
        (0x66, 0, 0.0, 0, 0, 3),
        (0x14, 0, 0.0, 0, 0, 0),
        (0x1e, 0, 0.0, 0, 0, 0),
        (0x1f, 0, 0.0, 0, 0, 0),
        (0x28, 0, 0.0, 0, 0, 0),
        (0x29, 0, 0.0, 0, 0, 0),
        (0x33, 0, 0.0, 0, 0, 0),
        (0x32, 0, 0.0, 0, 0, 0),
        (0x3c, 0, 0.0, 0, 0, 0),
        (0x3c, 0, 0.0, 0x0205_0000, 0, 0),
        (0x3d, 0, 0.0, 0, 0, 0),
        (0x3d, 0, 0.0, 0x0205_0000, 0, 0),
        (0x3e, 0, 0.0, 0, 0, 0),
        (0x3f, 0, 0.0, 0, 0, 0),
        (0x40, 0, 0.0, 0, 0, 0),
        (0x46, 0, 0.0, 0, 0, 0),
        (0x47, 0, -100.0, 0, 0, 0),
        (0x47, 0, 1000.0, 0, 0, 0),
        (0x47, 1, 1000.0, 0, 0, 0),
        (0x50, 0, 0.0, 0, 0, 0),
        (0x6e, 0, 0.0, 0, 0, 0),
        (0x6f, 0, 0.0, 0, 0, 0),
        (0x70, 3, 0.0, 0, 0, 0),
        (0x70, 0x4a, 0.0, 0, 0, 0),
        (0x2710, 0, 0.0, 0, 0, 0),
        (0x2711, 0, 0.0, 0, 0, 0),
        (5, 0, 0.0, 0, 0, 0),
        (0xffff_fff0, 0, 0.0, 0, 0, 0),
    ];
    for (k, (state, n20, x1c, ans, b390, n310)) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x600]);
        for o in (0x18u32..0x30).step_by(4) {
            t.set_u32(
                r + o,
                (0.5f32 * (o - 0x18) as f32 - 1.0 + k as f32).to_bits(),
            );
        }
        for o in (0x330u32..0x370).step_by(4) {
            t.set_u32(
                r + o,
                (0.25f32 * (o - 0x330) as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        // the screen object's junk (state 0x47 sets it up)
        t.write(r + 0x39c, &[0x3b ^ k as u8; 0x260]);
        t.set_u32(r + 0x1c, x1c.to_bits());
        t.set_u32(r + 0x20, n20);
        t.set_u8(r + 0x390, b390);
        // words the states clear, the flag globals they set bits in
        t.set_u32(r + 0x2e4, 0x5d5d_5d5d);
        t.set_u32(r + 0x398, 0x5d5d_5d5d);
        for op in [0x3fau32, 0xc72] {
            let g = w(&t, op);
            t.set_u32(g, 0x5d5d_5d50 + k);
        }
        t.set_u8(flag, (n20 & 1) as u8 | 0x10);
        t.set_u8(b228, 0x40 + k as u8);
        t.set_u32(r + 0x310, n310);
        t.set_u32(gs, state);
        // the parent node inside the record (state 0xb's append)
        let pn = r + 0x2c4;
        t.set_u32(pn + 0x10, pn);
        t.set_u32(pn + 0xc, pn);
        t.set_u32(answer, ans);
        // the link node under a parent (10000)
        let p = SCRATCH + 0x5c00;
        t.write(p, &[0; 0x20]);
        t.set_u32(p + 0x10, p);
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 9);
        t.set_u32(r + 0x374 + 0x14, p);
        t.set_u32(r + 0x374 + 4, p);
        t.set_u32(r + 0x374 + 8, p);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the parents: empty or with a last child
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        for op in [0x97u32, 0x198, 0x2b0, 0xabb] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, if k % 3 == 1 { kid } else { l });
            t.set_u32(l + 0xc, l);
            t.set_u16(l + 0x18, 2);
        }
        ownb::dirty_stack(&mut t, 0x8980 + k, &[]);
        let call = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[a], Eax::Checked);
    }
}

/// `magic_0186` 0x100077c0 (`r42::scene_task_0186`): every state (0 on
/// into 0xa, 0xb before and at 0x18 calls, 0x14 with its byte clear and
/// set, 0x15 with slot 411's answer 0 and not (on into 0x1e and 0x1f),
/// 0x1e with bit 2 of its flag clear and set, 0x1f, 0x20 with the fade
/// above 0, NaN and at or below 0 (the end scene: the flag bit, the
/// leaving node with and without a parent, its neighbours the parent or
/// not), 10000 before and at 60 calls), the states past the table and
/// between 0x16 and 0x1d, a big one.
#[test]
fn scene_task_0186() {
    let Some(mut t) = twin(186) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x77c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 411, 1, move |m, _| m.r32(answer));
    for (s, n) in [
        (228u32, 2usize),
        (215, 1),
        (239, 3),
        (21, 3),
        (213, 2),
        (1, 1),
        (713, 1),
        (719, 3),
    ] {
        // the answer from the words the slot reads only
        rec(&mut t, s, n, move |_, a| {
            let b = if n > 1 { a[1] } else { 0 };
            (s << 16) ^ a[0] ^ b.rotate_left(7)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the parents and lists the nodes go under: empty rings where 0
    let pnw = t.img.rel32_target(f + 0x51d);
    let enb = t.img.rel32_target(f + 0x534);
    let mut parents: Vec<u32> = [
        0x428u32, 0x661, 0x6c2, 0x79c, 0x7d7, 0x947, 0x3fc, 0x595, 0x5de, 0x75c, 0x78a,
    ]
    .iter()
    .map(|&o| w(&t, o))
    .collect();
    parents.push(t.img.word(pnw + 0x26));
    parents.push(t.img.word(enb + 0x18));
    for p in parents {
        if t.u32(p + 0x10) == 0 {
            t.set_u32(p + 0x10, p);
            t.set_u32(p + 0xc, p);
        }
    }
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let fade = w(&t, 0x5f);
    let parts_byte = w(&t, 0x734);
    let end_flag = w(&t, 0x8d7);
    let last_flag = w(&t, 0x1ce);
    let leaving = w(&t, 0x27c);
    let (lp, ln) = (w(&t, 0x28f), w(&t, 0x294));
    let saved: Vec<u32> = [leaving, lp, ln].iter().map(|&x| t.u32(x)).collect();
    // (state, count, slot 411's answer, flag byte, fade, the leaving
    // node's parent: 0 none, 1 primed, 2 the neighbours are the parent)
    // the parent list's last word (the end scene's first append)
    let plast = w(&t, 0xd6);
    let plast0 = t.u32(plast);
    let other = SCRATCH + 0x3700;
    let cases: [(u32, u32, u32, u8, f32, u32); 25] = [
        (0, 0, 0, 0, 0.0, 1),
        (0xa, 0, 0, 0, 0.0, 1),
        (0xb, 0x17, 0, 0, 0.0, 1),
        (0xb, 0x18, 0, 0, 0.0, 1),
        (0x14, 0, 0, 0, 0.0, 1),
        (0x14, 0, 0, 1, 0.0, 1),
        (0x15, 0, 0, 0, 0.0, 1),
        (0x15, 0, 7, 0, 0.0, 1),
        (0x15, 0, 7, 4, 0.0, 1),
        (0x1e, 0, 0, 0, 0.0, 1),
        (0x1e, 0, 0, 4, 0.0, 1),
        (0x1f, 0, 0, 0, 0.0, 1),
        (0x20, 0, 0, 0, 0.5, 1),
        (0x20, 0, 0, 0, f32::NAN, 1),
        (0x20, 0x3c, 0, 0, 0.0, 1),
        (0x20, 0, 0, 4, -1.0, 0),
        (0x20, 0x50, 0, 0, -1.0, 2),
        (0x20, 0x51, 0, 0, -1.0, 3),
        (0x20, 0x52, 0, 0, -1.0, 4),
        (0xa, 0, 0, 0, 0.0, 5),
        (0x2710, 0x3b, 0, 0, 0.0, 1),
        (0x2710, 0x3c, 0, 0, 0.0, 1),
        (5, 0, 0, 0, 0.0, 1),
        (0x18, 0, 0, 0, 0.0, 1),
        (0x3000, 0, 0, 0, 0.0, 1),
    ];
    for (k, (state, count, ans, fl, fd, lv)) in (0u32..).zip(cases) {
        t.write(r, &[0x5c ^ k as u8; 0x40]);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x2c, count);
        t.set_u32(answer, ans);
        t.set_u8(parts_byte, fl & 1);
        t.set_u8(end_flag, fl | 0x10);
        t.set_u8(last_flag, fl | 0x20);
        t.set_u32(fade, fd.to_bits());
        for (&g, &v) in [leaving, lp, ln].iter().zip(&saved) {
            t.set_u32(g, v);
        }
        t.set_u32(plast, plast0);
        t.write(other, &[0x6e; 0x20]);
        // the flag globals the states set bits in
        for op in [0x53au32, 0x712] {
            let g = w(&t, op);
            t.set_u32(g, 0x5d5d_5d50 + k);
        }
        // 0: no parent; 2, 3: one neighbour the parent, the other a node;
        // 4: the parent list empty (its last the parent)
        let p = t.u32(leaving);
        match lv {
            0 => t.set_u32(leaving, 0),
            2 => {
                t.set_u32(lp, p);
                t.set_u32(ln, other);
            }
            3 => {
                t.set_u32(lp, other);
                t.set_u32(ln, p);
            }
            4 => t.set_u32(plast, w(&t, 0x114)),
            _ => {}
        }
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        ownb::dirty_stack(&mut t, 0x77c0 + k, &[]);
        let call = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[a], Eax::Checked);
    }
}

/// The actors' positions slot 217 gives in `magic_0178`'s tests (four
/// floats each, by the actor word's low two bits).
const POS_0178: u32 = SCRATCH + 0x7a00;

/// The stubs of `magic_0178`'s tests: the summon stubs; 217 an actor's
/// position ([`POS_0178`]), 218 a target list, 531 four words, 289 a
/// float, the scene's slots logged with answers from the words they read,
/// 12 a float in ST0.
fn stubs_0178(t: &mut Twin) {
    all_stubs(t);
    summon_stubs::float_matrix_stubs(t);
    summon_stubs::vector_stubs(t);
    summon_stubs::summon_stubs(t);
    rec(t, 217, 2, |m, a| {
        let at = POS_0178 + 0x10 * (a[0] & 3);
        for i in 0..4 {
            let x = m.r32(at + 4 * i);
            m.w32(a[1] + 4 * i, x);
        }
        0x217
    });
    let list = SCRATCH + 0x6000;
    rec(t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(list + 0x7c));
        list
    });
    rec(t, 531, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, a[0].wrapping_mul(0x0101_0101) ^ i);
        }
        0x531
    });
    rec(t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
    for (s, n) in [
        (215u32, 1usize),
        (240, 1),
        (213, 2),
        (696, 2),
        (268, 3),
        (666, 2),
        (718, 1),
        (719, 3),
        (711, 3),
        (710, 4),
        (222, 4),
        (221, 4),
        (551, 3),
    ] {
        // the answer from the words the slot reads only
        rec(t, s, n, move |_, a| {
            let b = if n > 1 { a[1] } else { 0 };
            (s << 16) ^ a[0] ^ b.rotate_left(7)
        });
    }
    rec(t, 12, 1, |_, a| 0x12 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25 + 1.0);
    // the other float slots the summon code reads ST0 of
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(333u32, 0.75f64), (282, 2.25), (307, 0.625)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    rec(t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
}

/// `magic_0178` 0x100069a0 (`r42::scene_0178`): the parent's list empty
/// and not, the lists' and flags' words junk.
#[test]
fn scene_0178() {
    let Some(mut t) = twin(178) else { return };
    stubs_0178(&mut t);
    let f = t.at(0x69a0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let parent = w(&t, 0x50);
    let plast = w(&t, 0x15);
    let list2 = w(&t, 0x51a);
    let l2last = w(&t, 0x4da);
    let slast = w(&t, 0x540);
    let other = SCRATCH + 0x3700;
    for k in 0..3u32 {
        t.write(other, &[0x6e ^ k as u8; 0x40]);
        t.set_u32(plast, if k == 1 { other } else { parent });
        t.set_u32(l2last, if k == 2 { other } else { list2 });
        t.set_u32(slast, other + 0x20);
        for op in [0x5a2u32, 0x5fe, 0x652] {
            let g = w(&t, op);
            t.set_u32(g, 0x5d5d_5d50 + k);
        }
        t.set_u8(w(&t, 0x43c), 0x30 + k as u8);
        ownb::dirty_stack(&mut t, 0x69a0 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r42::scene_0178(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x40, "the scene's calls");
}

/// `magic_0178` 0x10007020 (`r42::summon_task_0178`): the states 0, 2
/// (the timer below, at and past its end, NaN), 3 (slot 240's answer 0
/// and not), 1 and 7.
#[test]
fn summon_task_0178() {
    let Some(mut t) = twin(178) else { return };
    stubs_0178(&mut t);
    let f = t.at(0x7020);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 240, 1, move |m, _| m.r32(answer));
    let st = w(&t, 0x6);
    let timer = w(&t, 0x78);
    let end = f32::from_bits(t.u32(w(&t, 0x87)));
    let step = f32::from_bits(t.u32(w(&t, 0x80)));
    let (a, node, n0, n1) = (
        SCRATCH + 0x100,
        SCRATCH + 0x3600,
        SCRATCH + 0x3680,
        SCRATCH + 0x36c0,
    );
    let cases: [(u16, f32, u32); 9] = [
        (0, 3.0, 0),
        (2, 0.0, 0),
        (2, end - step, 0),
        (2, end + 1.0, 0),
        (2, f32::NAN, 0),
        (3, 0.0, 0),
        (3, 0.0, 9),
        (1, 0.0, 0),
        (7, 0.0, 0),
    ];
    for (k, (state, x, ans)) in (0u32..).zip(cases) {
        t.set_u16(st, state);
        t.set_u32(timer, x.to_bits());
        t.set_u32(answer, ans);
        t.set_u32(a, node);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        ownb::dirty_stack(&mut t, 0x7020 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0178` 0x10007100 (`r42::start_up_0178`): the actors in every
/// quadrant of the flat angle (and on the axes, at the same place), the
/// static list empty and not, the lock word varied.
#[test]
fn start_up_0178() {
    let Some(mut t) = twin(178) else { return };
    stubs_0178(&mut t);
    let f = t.at(0x7100);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let list = SCRATCH + 0x6000;
    let lock = SCRATCH + 0x7f50;
    t.set_u32(w(&t, 0x731), lock);
    let slast = w(&t, 0x7e8);
    let other = SCRATCH + 0x3700;
    // (x, z) of `Q - P` (the actor `*G_A` at 0, the caster byte at 1)
    let quads: [(f32, f32); 10] = [
        (2.0, 3.0),
        (-2.0, -3.0),
        (2.0, -3.0),
        (-2.0, 3.0),
        (0.0, 3.0),
        (0.0, -3.0),
        (0.0, 0.0),
        (2.0, 0.0),
        (-2.0, 0.0),
        (f32::NAN, 1.0),
    ];
    for (k, (x, z)) in (0u32..).zip(quads) {
        // P at actor 0 (the word *G_A with low bits 0), Q at actor 1
        let pa = w(&t, 0x534);
        t.set_u32(pa, 0x0abc_0000 + 4 * k);
        t.set_u8(w(&t, 0x548), 1);
        for (i, v) in [1.0f32, 0.5, -1.0, 1.0].into_iter().enumerate() {
            t.set_u32(POS_0178 + 4 * i as u32, v.to_bits());
        }
        for (i, v) in [1.0 + x, 0.25, -1.0 + z, 1.0].into_iter().enumerate() {
            t.set_u32(POS_0178 + 0x10 + 4 * i as u32, v.to_bits());
        }
        t.write(list, &[0x80, 0x81, 0x83, 0x84]);
        t.set_u32(list + 0x7c, k % 2 + 1);
        t.set_u32(lock, 0x40 + k);
        t.write(other, &[0x6e ^ k as u8; 0x40]);
        t.set_u32(slast, other + 0x20);
        t.set_u32(w(&t, 0x829), 0x5d5d_5d50 + k);
        t.set_u8(w(&t, 0xef), 0x30 + k as u8);
        ownb::dirty_stack(&mut t, 0x7100 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
}

/// The stubs of `magic_0203`'s tests: the summon stubs, slot 324 a mask
/// from `MASK_0203`, 307 a float in ST0 by its call count, 713 and 388
/// logged.
const MASK_0203: u32 = SCRATCH + 0x7f40;

/// Empty rings at the lists the operands `ops` of `f` name (where 0).
fn rings_0203(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let p = t.img.word(f + o);
        if t.u32(p + 0x10) == 0 {
            t.set_u32(p + 0x10, p);
            t.set_u32(p + 0xc, p);
        }
    }
}

fn stubs_0203(t: &mut Twin) {
    all_stubs(t);
    summon_stubs::float_matrix_stubs(t);
    summon_stubs::vector_stubs(t);
    summon_stubs::summon_stubs(t);
    rec(t, 324, 0, |m, _| {
        let k = m.r32(MASK_0203 + 4);
        m.w32(MASK_0203 + 4, k + 1);
        m.r32(MASK_0203).rotate_left(k)
    });
    rec(t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(MASK_0203 + 8);
        m.w32(MASK_0203 + 8, k + 1);
        0.125 + f64::from(k % 7) * 0.375
    });
    for (s, n) in [(713u32, 1usize), (388, 3)] {
        rec(t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
}

/// A node at `node` under `parent` (0: none) with the neighbours `prev`
/// and `next`.
fn child_0203(t: &mut Twin, node: u32, parent: u32, prev: u32, next: u32) {
    t.set_u32(node + 4, prev);
    t.set_u32(node + 8, next);
    t.set_u32(node + 0x14, parent);
}

/// `magic_0203` 0x100079b0 (`r42::end_task_0203`): state 0 waiting
/// (below the end, NaN) and going on (the part's parent and flag both
/// ways, masks over 0..3 parts), 10000 with bit 2 of `R + 0x448` clear
/// and set (the six nodes with and without parents, the parent their
/// neighbour or not), other states.
#[test]
fn end_task_0203() {
    let Some(mut t) = twin(203) else { return };
    stubs_0203(&mut t);
    let f = t.at(0x79b0);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    rings_0203(&mut t, f, &[0x118]);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let parent = SCRATCH + 0x3800;
    let other = SCRATCH + 0x3900;
    let parts = SCRATCH + 0x3a00;
    let end = f32::from_bits(t.u32(t.img.word(f + 0x57)));
    // (state, timer, count of parts, mask, flags byte +0x448, part flag
    // +0xe8, parents: 0 none, 1 the parent with it its neighbours, 2 the
    // parent with another node as neighbours)
    let cases: [(u32, f32, u32, u32, u8, u8, u32); 12] = [
        (0, 0.0, 0, 0, 0, 0, 1),
        (0, f32::NAN, 0, 0, 0, 0, 1),
        (0, end, 0, 0, 0, 0, 1),
        (0, end + 1.0, 2, 0b11, 4, 0, 1),
        (0, end + 1.0, 3, 0b101, 0, 4, 2),
        (0, end + 1.0, 0, 7, 4, 4, 0),
        (0, end + 1.0, 0xffff_fffe, 7, 0, 0, 1),
        (0x2710, 0.0, 0, 0, 0, 0, 1),
        (0x2710, 0.0, 0, 0, 4, 0, 1),
        (0x2710, 0.0, 0, 0, 4, 0, 2),
        (0x2710, 0.0, 0, 0, 4, 0, 0),
        (5, 0.0, 0, 0, 4, 0, 1),
    ];
    for (k, (state, timer, count, mask, fl, pfl, pk)) in (0u32..).zip(cases) {
        t.write(r, &[0x3c ^ k as u8; 0x500]);
        t.write(parent, &[0; 0x20]);
        t.write(other, &[0x6e; 0x40]);
        t.set_u32(r + 0x48c, state);
        t.set_u32(r + 0x38, (timer - 0.25).to_bits());
        t.set_u32(r + 0x474, 0.25f32.to_bits());
        t.set_u32(r + 0x488, count);
        t.set_u8(r + 0x448, fl);
        t.set_u32(r + 0x480, 0x40 + k);
        t.set_u32(MASK_0203, mask);
        t.set_u32(MASK_0203 + 4, 0);
        for i in 0..3u32 {
            let p = parts + 0x80 * (i + 1);
            t.set_u32(parts + 4 * i, p);
            t.set_u32(p + 0x50, 0x0050_0000 + i + 0x10 * k);
        }
        t.set_u32(r + 0x460, parts);
        for n in [0x18u32, 0x44, 0x9c, 0x70, 0xc8, 0x214, 0x360] {
            let nd = r + n;
            match pk {
                0 => child_0203(&mut t, nd, 0, nd, nd),
                1 => child_0203(&mut t, nd, parent, parent, parent),
                _ => child_0203(&mut t, nd, parent, other, other + 0x20),
            }
        }
        t.set_u16(parent + 0x18, 9);
        t.set_u8(r + 0x360 + 0xe8, pfl);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        ownb::dirty_stack(&mut t, 0x79b0 + k, &[]);
        let call = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[a], Eax::Checked);
    }
}

/// `magic_0203` 0x10007de0 (`r42::summon_setup_0203`): the parent's and
/// the static list's lists empty and not, slot 324's bit 10 both ways.
#[test]
fn summon_setup_0203() {
    let Some(mut t) = twin(203) else { return };
    stubs_0203(&mut t);
    let f = t.at(0x7de0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    rings_0203(&mut t, f, &[0x1c6, 0x319, 0x358]);
    let o = SCRATCH + 0x1_0000;
    let v = SCRATCH + 0x3400;
    let plast = w(&t, 0x75);
    let parent = w(&t, 0x82);
    let slast = w(&t, 0x22);
    let other = SCRATCH + 0x3900;
    for k in 0..3u32 {
        t.write(o, &[0x3c ^ k as u8; 0x500]);
        t.write(other, &[0x6e ^ k as u8; 0x40]);
        ownb::floats(&mut t, v, &[1.5, -2.0 + k as f32, 0.25, 1.0]);
        t.set_u32(plast, if k == 1 { other } else { parent });
        t.set_u32(slast, other + 0x20);
        t.set_u32(w(&t, 0x3d), 0x5d5d_5d50 + k);
        t.set_u32(MASK_0203, if k == 2 { 0x400 } else { 0x3ff });
        t.set_u32(MASK_0203 + 4, 0);
        ownb::dirty_stack(&mut t, 0x7de0 + k, &[]);
        let call = Call {
            ecx: o,
            edx: v,
            ..Call::default()
        };
        t.check(
            f,
            call,
            &[0x0001_2000 + k, SCRATCH + 0x3a00, 3 + k],
            Eax::Checked,
            |g, img, fr| Ret::eax(r42::summon_setup_0203(g, img, fr)),
        );
    }
}
