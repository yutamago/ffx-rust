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
    let cases: [(u32, u32, u32, u8, f32, u32); 22] = [
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
        match lv {
            0 => t.set_u32(leaving, 0),
            2 => {
                let p = t.u32(leaving);
                t.set_u32(lp, p);
                t.set_u32(ln, p);
            }
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
