//! Round 46's ports (`ffx_magic::r46`) against each original. Needs
//! `FFX_GAME_DIR`.

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
use summon_stubs::{all_stubs, junk, junk_floats, prime, rf, visit_stub, wf};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn set_f(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// One case of [`camera_task_0070`]: the state, the wait `R + 0x38`, the
/// flag byte (bit 0 of the slot-666 test), the list's count, a variant
/// `v` that moves the compared globals across their limits, and slot
/// 520's three answers.
#[derive(Clone, Copy)]
struct Case0070 {
    state: u32,
    wait: i32,
    flag: u8,
    count: u32,
    v: u32,
    s520: [u32; 3],
}

const fn c070(state: u32, wait: i32) -> Case0070 {
    Case0070 {
        state,
        wait,
        flag: 0,
        count: 0,
        v: 0,
        s520: [7, 7, 7],
    }
}

/// `magic_0070` 0x10009e30 (`r46::camera_task_0070`): every state of the
/// three tables waiting and going on, the chains (0x96 into 0x97, 0xaa
/// into 0xac, 0x190 into 0x192, 0x19c into 0x19d, 0x3e9 into 0x3ec), slot
/// 520's answers (each nudge, the cut, the list skipped), the slot-666
/// flag both ways, slot 288's list (empty, every kind of byte, a count
/// slot 296 lowers), the fade at and off 0, the light's and the move's
/// limits both ways, 0x2710, and the states between and past the tables.
#[test]
fn camera_task_0070() {
    let Some(mut t) = twin(70) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x9e30);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    // the frame: `base` = E - 0xc; slot 288's count at `E + 4`
    let base = ((ownb::ENTRY_ESP - 4) & !7) - 0x54 - 0xc;
    let n_at = base + 0x10;
    let list = SCRATCH + 0x7a00;
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(SCRATCH + 0x7af0);
        m.w32(a[1], n);
        list
    });
    // slot 296 lowers the count when it sees 0x33 (the loop re-reads it)
    rec(&mut t, 296, 2, move |m, a| {
        if a[0] == 0x33 {
            let n = m.r32(n_at);
            m.w32(n_at, n.wrapping_sub(2));
        }
        0x296
    });
    // slot 520: the case's answers in turn
    rec(&mut t, 520, 0, |m, _| {
        let i = m.r32(SCRATCH + 0x7e30);
        m.w32(SCRATCH + 0x7e30, i + 1);
        m.r32(SCRATCH + 0x7e40 + 4 * (i % 3))
    });
    // slot 255 (the part call) keeps the place it is handed (slot 520's
    // nudge shows)
    rec(&mut t, 255, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x7e60 + 4 * i, x);
        }
        0x255
    });
    rec(&mut t, 292, 1, |m, a| {
        for i in 0..4 {
            wf(m, a[0] + 4 * i, 1.25 * i as f32 - 3.0);
        }
        0x292
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 + (a[1] & 7) as f32);
        }
        0x237
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [
        (12u32, -1.5f64),
        (333, 0.75),
        (282, 2.25),
        (277, 4.0),
        (307, 0.625),
    ] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the owned nodes' vtable visits through a stub; the bones' table
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    // (the owned node's, the two appends' vtables: their visits are
    // overlay code, which only the port would call out to)
    for (c, op) in [(0x1dau32, 5u32), (0x15d, 0x2f), (0x180, 2)] {
        let vt = t.img.word(call(&t, c) + op);
        t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    }
    // (and the node ease's object: its sub walk, 0x10006950, at
    // 0x10010518 + 0x1c)
    t.set_u32(t.at(0x1_0534), magic::TRAPS + 16 * visit);
    let bone = call(&t, 0x296);
    let table = t.img.word(bone + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    // the move's ends read `to`'s z (E + 0x38) and, in 0x19b, `from`'s z
    // (E + 0x18) before anything writes them: 0 as the port
    let junk = [
        ownb::ENTRY_ESP - (base + 0x44),
        ownb::ENTRY_ESP - (base + 0x24),
    ];
    let mut cases = vec![
        c070(0, 0),
        c070(0xb4, 0),
        c070(0x320, 0),
        c070(0xa0, 5),
        c070(0x2c0, 0),
        c070(0x97, 0),
        c070(0x98, 0),
        c070(0x98, 0x46),
        c070(0x98, 0x40),
        c070(0xa1, 0x18),
        c070(0xa1, 0x19),
        c070(0xc8, 0),
        c070(0xc9, 0),
        Case0070 {
            v: 1,
            ..c070(0xc9, 0)
        },
        Case0070 {
            v: 3,
            ..c070(0xc9, 0)
        },
        c070(0xaa, 0),
        c070(0xab, 0),
        c070(0xac, 0),
        c070(0x190, 0),
        c070(0x191, 3),
        c070(0x191, 4),
        c070(0x192, 0),
        Case0070 {
            v: 1,
            ..c070(0x192, 0)
        },
        Case0070 {
            v: 2,
            ..c070(0x192, 0)
        },
        Case0070 {
            v: 3,
            ..c070(0x192, 0)
        },
        c070(0x193, 0),
        c070(0x19a, 0),
        c070(0x19b, 9),
        c070(0x19b, 0xa),
        c070(0x19c, 0x18),
        c070(0x19c, 0x19),
        c070(0x19d, 0),
        Case0070 {
            v: 1,
            ..c070(0x19d, 0)
        },
        Case0070 {
            v: 3,
            ..c070(0x19d, 0)
        },
        c070(0x19e, 0),
        c070(0x2bc, 0),
        c070(0x2bd, 0),
        c070(0x2be, 0),
        c070(0x2bf, 0),
        c070(0x321, 0),
        c070(0x384, 0),
        c070(0x385, 0x45),
        c070(0x385, 0x46),
        c070(0x386, 0x13),
        c070(0x386, 0x14),
        c070(0x387, 0x40),
        c070(0x387, 0x41),
        c070(0x388, 0),
        c070(0x3e8, 0),
        c070(0x3e9, 9),
        c070(0x3e9, 0xa),
        c070(0x3ea, 9),
        c070(0x3ea, 0xa),
        c070(0x3eb, 0x1d),
        c070(0x3eb, 0x1e),
        c070(0x3ec, 0),
        c070(0x3ed, 0),
        c070(0x2710, 0),
        c070(0x2711, 0),
        c070(0xca, 0),
        c070(5, 0),
        c070(0xffff_fff0, 0),
    ];
    for (flag, count, s520) in [
        (0u8, 0u32, [7u32, 7, 7]),
        (1, 0, [0x0235_0000, 0x0237_0000, 7]),
        (0, 7, [0x0236_0000, 0x0236_0000, 0x0236_0000]),
        (1, 9, [0x0237_0000, 7, 0x0238_0000]),
        (0, 9, [0x0238_0000, 0x0237_0000, 0x0237_0000]),
        (0, 7, [0x0239_0000, 0x0238_0000, 0xfff0_0000]),
        (1, 7, [0xfdc9_0000, 0, 0x0237_0000]),
    ] {
        cases.push(Case0070 {
            flag,
            count,
            s520,
            ..c070(0x96, 0)
        });
    }
    // the list: the DLL's byte, the last's, 0xff, others (0x33 lowers the
    // count by 2)
    let own = t.u32(w(&t, 0x484)) as u8;
    let last_byte = 0x44u8;
    for (k, cs) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x100]);
        t.set_u32(r + 0x18, cs.state);
        set_f(&mut t, r + 0x1c, 0.75 + k as f32);
        t.set_u32(r + 0x34, 0x100 + k);
        t.set_u32(r + 0x38, cs.wait.cast_unsigned());
        t.set_u32(r + 0x3c, 0x30 + k);
        t.set_u32(SCRATCH + 0x7af0, cs.count);
        t.set_u32(SCRATCH + 0x7e30, 0);
        for (i, x) in (0u32..).zip(cs.s520) {
            t.set_u32(SCRATCH + 0x7e40 + 4 * i, x);
        }
        t.write(
            list,
            &[
                0x21, own, last_byte, 0xff, 0x33, 0x52, 0x60, own, 0x7f, 0x80,
            ],
        );
        t.set_u32(w(&t, 0x48f), u32::from(last_byte));
        // the flag words the states set and clear bits of
        for (i, o) in (0u32..).zip([0x2bau32, 0xa69, 0xbbe, 0xddb, 0x1497]) {
            t.set_u32(w(&t, o), 0x5a5a_0f0e ^ (k << 4) ^ i);
        }
        // (the slot-666 flag byte is the low byte of `+0x2ba`'s word: after
        // it)
        t.set_u8(w(&t, 0x113), 0x4e ^ cs.flag);
        // slot 520's cut word, the release's handle, slot 228's byte
        t.set_u32(w(&t, 0x1b3), 0x0003_1234 + (k << 8));
        t.set_u32(w(&t, 0x2f3), 0x0f00_0000 + k);
        t.set_u8(w(&t, 0x2d7), 0x40 + k as u8);
        // (the climb's and 0x19d's step word is 0x192's `y`)
        t.set_u32(w(&t, 0x9b2), 0x2000 + k);
        // the move's words: 0x192's `x` above or at -0x800, `y` below or at
        // 0x5800; the light's limits (`K > L` or not)
        let (x, y) = match cs.v {
            1 => (-0x800i32, 0x5800i32),
            2 => (-0x7ff, 0x57ff),
            _ => (0x100, 0x200),
        };
        t.set_u32(w(&t, 0xb48), x.cast_unsigned());
        t.set_u32(w(&t, 0xb5c), y.cast_unsigned());
        // words the states clear or copy into, junk first
        for (i, o) in (0u32..).zip([0x11e2u32, 0x1286, 0x1334, 0x105d]) {
            t.set_u16(w(&t, o), 0x5a00 ^ (k as u16) ^ (i as u16));
        }
        for (i, o) in (0u32..).zip([0x1531u32, 0x1539, 0x14db, 0x14e3]) {
            t.set_u32(w(&t, o), 0x6b6b_0000 ^ k ^ (i << 8));
        }
        // the time left's effect and the model state's object: their parts
        for (i, o) in (0u32..).zip([0x18cu32, 0x22c]) {
            let obj = w(&t, o);
            t.set_u32(obj + 0x50, 0x0700_0000 + 0x10 * i + k);
            t.set_u32(obj + 0x54, 0x0800_0000 + 0x10 * i + k);
        }
        // the move globals' floats and the light's place
        for (i, o) in (0u32..).zip([
            0xa5cu32, 0xa64, 0xa74, 0xa87, 0xa8f, 0xac0, 0xac8, 0xaf9, 0xb01, 0x833, 0xa28, 0x11f8,
            0x1266, 0x1398, 0x1692, 0x16aa, 0x16c2, 0x1013, 0x11ac,
        ]) {
            let at = w(&t, o);
            set_f(&mut t, at, 1.5 * i as f32 - 3.9 + 0.125 * k as f32);
        }
        // the light's level (one global for the three states; after the
        // floats, `+0xa28` names it too): below, far above, or equal to
        // the state's limit
        let (o, lim) = match cs.state {
            0x19c | 0x19d => (0xd05u32, 0xcf8u32),
            0xc9 => (0x861, 0x85a),
            _ => (0xb2e, 0xb21),
        };
        let at = w(&t, o);
        let level = match cs.v {
            1 => 1.0e6,
            3 => f32::from_bits(t.u32(w(&t, lim))),
            _ => -3.5,
        };
        set_f(&mut t, at, level);
        // the summon's byte (the model node's object `+0x50`)
        t.set_u8(w(&t, 0x5a), 0x20 + k as u8);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the DLL's node list empty or with a last child
        let dll_list = w(&t, 0x153);
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(dll_list + 0x10, if k % 3 == 1 { kid } else { dll_list });
        t.set_u32(dll_list + 0xc, dll_list);
        t.set_u16(dll_list + 0x18, 2);
        ownb::dirty_stack(&mut t, 0x9e30 + k, &junk);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// One case of [`main_task_0152`]: the state, the count `R + 0x2918`
/// (before the call's step), the timers `R + 0x290c`/`R + 0x2910`, the
/// step `R + 0x2914`, and a variant `v`: bit 0 slot 689 answers, bit 1
/// slot 20 answers, bit 2 the byte `R + 0xbb8`'s bit 2, bit 3 `R + 0x78`
/// negative, bit 4 slot 288's list holds bytes, bit 5 slot 520 answers
/// 0x18b0046, bit 6 the character flag's bit 0.
#[derive(Clone, Copy)]
struct Case0152 {
    state: u32,
    count: i32,
    t1: f32,
    t2: f32,
    step: u32,
    v: u32,
}

const fn c152(state: u32, count: i32, t1: f32, t2: f32, step: u32, v: u32) -> Case0152 {
    Case0152 {
        state,
        count,
        t1,
        t2,
        step,
        v,
    }
}

/// `magic_0152` 0x10009a90 (`r46::main_task_0152`, r37b's 0533 main task
/// on 0152's layout): every state waiting and going on, the chains (0 into 0x64, 0x65 into 0x5a into 0x5b into
/// 0x14, 0x20 into 0x28 into 0x32, 0x33 into 0x34 into 0x35 into 0x36,
/// 0x37 into 0x38 into 0x39 into 0x3a, 0x50 into 10000), slots 689, 20,
/// 549 (once busy) both ways, the bones' steps, slot 288's list, the
/// states past the table, slot 520's answer, the character flag both
/// ways.
#[test]
fn main_task_0152() {
    let Some(mut t) = twin(152) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x9a90);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    // slot 689 and 20 answer by the case; 549 is busy every other call
    rec(&mut t, 689, 0, move |m, _| m.r32(answer) & 1);
    rec(&mut t, 20, 1, move |m, a| {
        (m.r32(answer) >> 1 & 1) * (a[0] | 1)
    });
    rec(&mut t, 520, 0, move |m, _| {
        if m.r32(answer) & 0x20 != 0 {
            0x018b_0046
        } else {
            0x018b_0047
        }
    });
    rec(&mut t, 549, 1, |m, _| {
        let k = m.r32(SCRATCH + 0x7f4);
        m.w32(SCRATCH + 0x7f4, k + 1);
        (k + 1) & 1
    });
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x7f8);
        m.w32(SCRATCH + 0x7f8, k + 0x0123_0457);
        k ^ 0xfc00
    });
    let list = SCRATCH + 0x7a00;
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(SCRATCH + 0x7af0);
        m.w32(a[1], n);
        list
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (307, 0.625)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let (lo, hi) = (-100.0f32, 1000.0f32);
    let mut cases = vec![
        c152(0, 0, 0.0, 0.0, 0, 0),
        c152(0, 0, 0.0, 0.0, 0, 1),
        c152(0x64, 0, 0.0, 0.0, 0, 0),
        c152(0x65, 0x3a, 0.0, 0.0, 0, 0),
        c152(0x65, 0x3b, lo, 0.0, 0, 0),
        c152(0x65, 0x3b, hi, 0.0, 0, 0),
        c152(0x1e, 0, 0.0, 0.0, 0, 0),
        c152(0x5a, 0, lo, 0.0, 0, 0),
        c152(0x5a, 0, hi, lo, 0, 0),
        c152(0x5b, 0, lo, 0.0, 0, 0),
        c152(0x5b, 0x2c, hi, hi, 0, 0),
        c152(0x5b, 0x2c, hi, lo, 0, 0),
        c152(0x14, 0, 0.0, lo, 0, 0),
        c152(0x14, 0, 0.0, hi, 2, 0),
        c152(0x14, 0x2c, 0.0, hi, 3, 0),
        c152(0x14, 0x2c, 0.0, hi, 4, 0),
        c152(0x1f, 5, 0.0, 0.0, 0, 0),
        c152(0x1f, 6, 0.0, 0.0, 0, 0),
        c152(0x20, 0, 0.0, 0.0, 0, 0),
        c152(0x20, 0x10, 0.0, 0.0, 0, 8),
        c152(0x20, 0x76, 0.0, 0.0, 0, 8),
        c152(0x20, 0x76, 0.0, 0.0, 0, 0x18),
        c152(0x20, 0x77, 0.0, lo, 0, 8),
        c152(0x20, 0x77, 0.0, hi, 0, 0xc),
        c152(0x20, 0x77, 0.0, hi, 0, 0x8),
        c152(0x28, 0, 0.0, lo, 0, 0),
        c152(0x28, 0, 0.0, hi, 0, 0),
        c152(0x28, 0, 0.0, hi, 0, 1),
        c152(0x28, 0, 0.0, hi, 0, 3),
        c152(0x32, 0, 0.0, 0.0, 0, 0),
        c152(0x32, 0, 0.0, 0.0, 0, 1),
        c152(0x32, 0, 0.0, 0.0, 0, 2),
        c152(0x33, 5, 0.0, 0.0, 0, 0),
        c152(0x33, 0, 0.0, 0.0, 0, 0),
        c152(0x33, 1, 0.0, hi, 0, 0),
        c152(0x34, 0, 0.0, lo, 0, 0),
        c152(0x34, 0, 0.0, hi, 0, 0),
        c152(0x34, 0, 0.0, hi, 3, 0),
        c152(0x34, 0x1d, 0.0, lo, 3, 0),
        c152(0x35, 0, 0.0, hi, 0, 0),
        c152(0x35, 0, 0.0, hi, 3, 0),
        c152(0x35, 0x1d, 0.0, hi, 3, 0),
        c152(0x36, 0, 0.0, 0.0, 0, 0),
        c152(0x37, 0, 0.0, lo, 0, 0),
        c152(0x37, 0, 0.0, lo, 0, 1),
        c152(0x38, 0, 0.0, lo, 0, 0),
        c152(0x38, 0, 0.0, hi, 0, 0),
        c152(0x39, 0, 0.0, lo, 0, 0),
        c152(0x39, 0, 0.0, 2.0, 0, 0),
        c152(0x39, 0, 0.0, hi, 0, 0),
        c152(0x3a, 0, 0.0, lo, 0, 0),
        c152(0x3a, 0, 0.0, hi, 0, 0),
        c152(0x46, 0, 0.0, 0.0, 0, 0),
        c152(0x47, 0xb0, 0.0, 0.0, 0, 0),
        c152(0x47, 0xb1, 0.0, 0.0, 0, 0),
        c152(0x50, 0x7c, 0.0, 0.0, 0, 0),
        c152(0x50, 0x7d, 0.0, 0.0, 0, 0),
        c152(0x2710, 0, 0.0, 0.0, 0, 0),
        c152(0x2711, 0, 0.0, 0.0, 0, 0),
        c152(0x2712, 0, 0.0, 0.0, 0, 0),
        c152(0x66, 0, 0.0, 0.0, 0, 0),
        c152(5, 0, 0.0, 0.0, 0, 0),
        c152(0xffff_fff0, 0, 0.0, 0.0, 0, 0),
    ];
    // slot 288's list (152 runs slot 296 on every byte)
    let (own, skip) = (0x44u32, 0x45u8);
    cases.push(c152(0x20, 0x76, 0.0, 0.0, 0, 0x18));
    // slot 520's answer, the character flag both ways
    for v in [0x20u32, 0x21, 0x23] {
        cases.push(c152(0x28, 0, 0.0, hi, 0, v));
    }
    cases.push(c152(0x5a, 0, lo, 0.0, 0, 0x40));
    cases.push(c152(0x5a, 0, hi, lo, 0, 0x40));
    // 0x14's last bone at count 0x2c (`G_S` 10) and 0x2d
    cases.push(c152(0x14, 0x2b, 0.0, lo, 4, 0));
    cases.push(c152(0x14, 0x2b, 0.0, hi, 4, 0));
    for (k, cs) in (0u32..).zip(cases) {
        t.write(r, &vec![0u8; 0x2920]);
        // the record's floats and words: junk-ish but finite
        for o in (0u32..0x2900).step_by(0x40) {
            t.set_u32(r + o + 0x1c, (0.5f32 + (o >> 6) as f32).to_bits());
        }
        // words the states clear (nonzero first)
        for o in [
            0x94cu32, 0x974, 0x99c, 0x9c4, 0xa38, 0xa40, 0xa90, 0xa64, 0xa6c, 0xabc,
        ] {
            t.set_u32(r + o, 0x6c6c_0000 + o);
        }
        for j in 0..8u32 {
            t.set_u32(r + 0x27b4 + 0x2c * j, 0x6d6d_0000 + j);
        }
        // the eight bones' owned nodes: every other one alone under a
        // parent of its own, the others without one
        for i in 0..8u32 {
            let n = r + 0x1a70 + 0x14c * i;
            let p = SCRATCH + 0x6000 + 0x40 * i;
            t.write(p, &[0; 0x40]);
            if i % 2 == 0 {
                t.set_u32(n + 0x14, p);
                t.set_u32(n + 4, p);
                t.set_u32(n + 8, p);
                t.set_u32(p + 0x10, n);
                t.set_u32(p + 0xc, n);
                t.set_u16(p + 0x18, 1);
            } else {
                t.set_u32(n + 0x14, 0);
            }
        }
        t.set_u32(r + 0x2908, cs.state);
        t.set_u32(r + 0x2918, cs.count.cast_unsigned());
        set_f(&mut t, r + 0x290c, cs.t1);
        set_f(&mut t, r + 0x2910, cs.t2);
        t.set_u32(r + 0x2914, cs.step);
        t.set_u32(
            r + 0x78,
            if cs.v & 8 != 0 {
                0xffff_ff00
            } else {
                0x100 + k
            },
        );
        t.set_u8(r + 0xbb8, if cs.v & 4 != 0 { 4 } else { 0xfb });
        t.set_u32(answer, cs.v & 0x23);
        let flag = t.u32(w(&t, 0x1be)) as u8;
        t.set_u8(w(&t, 0x1be), (flag & !1) | u8::from(cs.v & 0x40 != 0));
        // `G_S` junk first (0x14 copies it at counts other than 0x2c)
        t.set_u32(w(&t, 0x88c), 0x77 + k);
        t.set_u32(SCRATCH + 0x7f4, k & 1);
        t.set_u32(SCRATCH + 0x7af0, if cs.v & 0x10 != 0 { 7 } else { 0 });
        t.write(
            list,
            &[0x21, own as u8, skip, 0x33, 0x52, own as u8, 0x7f, 0x80],
        );
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the DLL's lists empty or with a last child
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        for op in [
            0x1f4u32, 0x241, 0x3f6, 0x400, 0x484, 0x5fd, 0x635, 0x693, 0x7d2, 0xecb, 0xfb1, 0x1082,
            0x10f8, 0x1379, 0x141c, 0x149d,
        ] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, if k % 3 == 1 { kid } else { l });
            t.set_u32(l + 0xc, l);
            t.set_u16(l + 0x18, 2);
        }
        ownb::dirty_stack(&mut t, 0x9a90 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// 0155's offset `x` in `magic_0164`'s summon main (as the port's map).
fn o164(x: u32) -> u32 {
    let mut r = x;
    for (start, here) in [
        (0u32, 0u32),
        (0x40e, 0x411),
        (0xbf7, 0xbfd),
        (0xc00, 0xc03),
        (0xeb0, 0xe8f),
        (0xece, 0xea3),
        (0xf2d, 0xef0),
        (0xfc8, 0xf8e),
        (0xff3, 0xfa4),
        (0x1151, 0x1105),
        (0x117c, 0x111b),
        (0x12fc, 0x12a9),
        (0x131d, 0x12b8),
        (0x1421, 0x1439),
        (0x1435, 0x144e),
        (0x14cd, 0x146c),
    ] {
        if x >= start {
            r = x - start + here;
        }
    }
    r
}

/// `magic_0155` 0x1000a990 and `magic_0164` 0x1000a770
/// (`r46::summon_main_0155`, `summon_main_0164`): state 0 (slot 294
/// answering 0xff and not), every state's wait on both sides of its
/// bound and the chains it runs into, the fade word above and at its
/// floor, the light's flag and the part's flag both ways, 10000 and the
/// states past the table; the parents' lists empty and not.
#[test]
fn summon_main_0155_0164() {
    for (n, rva, is_0164) in [(155u16, 0xa990u32, false), (164, 0xa770, true)] {
        summon_main_case(n, rva, is_0164);
    }
}

fn summon_main_case(n: u16, rva: u32, is_0164: bool) {
    let Some(mut t) = twin(n) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(rva);
    let o = move |x: u32| if is_0164 { o164(x) } else { x };
    let w = move |t: &Twin, x: u32| t.img.word(f + o(x));
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 294, 1, move |m, _| m.r32(answer));
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (277, 4.0)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    // slot 771's handle: its high word too
    t.stub_edx(771, |_, a| 0x0077_1000 ^ a[0]);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the owned nodes' vtable visits through a stub
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let owned = t.img.rel32_target(f + o(0xa7b));
    let vt = t.img.word(owned + 5);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    // the parents (the scene's, the pair's)
    let parents = [w(&t, 0x182), w(&t, 0x532)];
    // the static list's last (its `+4` takes the node)
    let qlast = w(&t, 0x5e2);
    let wait49 = if is_0164 { 0x32 } else { 0x5a };
    // (state, the count `R + 0x2c`, slot 294, the flag bits, lists empty)
    let mut cases: Vec<(u32, i32, u32, u8, bool)> = vec![
        (0, 0, 0xff, 0, true),
        (0, 0, 3, 5, false),
        (0, 0x20, 0x7f, 0, true),
        (0xa, 0xe, 0, 0, true),
        (0xa, 0xf, 0, 0, true),
        (0xb, 0x18, 0, 0, false),
        (0xb, 0x19, 0, 0, false),
        (0xc, 0x34, 0, 0, true),
        (0xc, 0x35, 0, 0, true),
        (0xd, 0x35, 0, 0, false),
        (0xd, 0x36, 0, 0, false),
        (0xd, 0x36, 0, 0, true),
        (0x14, 3, 0, 0, true),
        (0x15, 0xe, 0, 0, false),
        (0x15, 0xf, 0, 0, false),
        (0x16, 0x13, 0, 0, true),
        (0x16, 0x14, 0, 0, true),
        (0x17, 0x1d, 0, 0, true),
        (0x17, 0x1e, 0, 0, true),
        (0x18, 0x48, 0, 0, true),
        (0x18, 0x49, 0, 0, true),
        (0x1e, 2, 0, 0, false),
        (0x1f, 4, 0, 0, true),
        (0x1f, 5, 0, 0, true),
        (0x20, 8, 0, 0, false),
        (0x20, 9, 0, 0, false),
        (0x21, 0xe, 0, 0, true),
        (0x21, 0xf, 0, 0, true),
        (0x28, 0, 0, 0, true),
        (0x29, 0x1d, 0, 0, true),
        (0x29, 0x1e, 0, 0, true),
        (0x2a, 0x31, 0, 0, true),
        (0x2a, 0x32, 0, 0, true),
        (0x2b, 0x45, 0, 0, true),
        (0x2b, 0x46, 0, 0, true),
        (0x2c, 0x77, 0, 0, true),
        (0x2c, 0x78, 0, 0, true),
        (0x32, 0, 0, 0, true),
        (0x33, 0, 0, 0, true),
        (0x34, 0x42, 0, 0, true),
        (0x34, 0x43, 0, 1, true),
        (0x34, 0x43, 0, 4, true),
        (0x35, 0, 0, 0, true),
        (0x35, 0, 0, 1, true),
        (0x35, 0, 0, 5, true),
        (0x3c, 0, 0, 0, true),
        (0x3d, 0, 0, 4, true),
        (0x3e, 7, 0, 0, true),
        (0x3e, 8, 0, 0, true),
        (0x3f, 0x1d, 0, 0, true),
        (0x3f, 0x1e, 0, 0, true),
        (0x40, 0x39, 0, 0, true),
        (0x40, 0x3a, 0, 0, true),
        (0x46, 0, 0, 0, true),
        (0x47, 0, 0, 0, true),
        (0x47, 1, 0, 0, true),
        (0x48, 1, 0, 0, true),
        (0x48, 2, 0, 0, true),
        (0x49, wait49 - 1, 0, 0, true),
        (0x49, wait49, 0, 0, true),
        (0x2710, 0, 0, 0, true),
        (0x2711, 0, 0, 0, true),
        (5, 0, 0, 0, true),
        (0x4a, 0, 0, 0, true),
        (0x1000, 0, 0, 0, true),
    ];
    cases.push((0x0e, 0, 0, 0, true));
    for (k, (state, count, ans, bits, empty)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x60, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x2c, count.cast_unsigned());
        t.set_u32(answer, ans);
        t.set_u32(a + 4, r);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
        t.set_u32(a, node);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        for (i, &p) in (0u32..).zip(&parents) {
            let blk = SCRATCH + 0x7800 + 0x40 * i;
            junk(&mut t, blk, 0x20, k + i);
            t.set_u32(p + 0x10, if empty { p } else { blk });
            t.set_u32(p + 0xc, p);
            t.set_u16(p + 0x18, 1 + i as u16);
        }
        let qblk = SCRATCH + 0x7900;
        junk(&mut t, qblk, 0x20, k);
        t.set_u32(qlast, qblk);
        // the light's flag (bit 0) and the part's flag (bit 2)
        for x in [0x8eeu32, 0x90c] {
            let at = w(&t, x);
            t.set_u8(at, bits | 0xa0);
        }
        // the fade word above its floor or at it
        let fade = w(&t, 0xff7);
        let floor = f32::from_bits(t.u32(w(&t, 0xffe)));
        t.set_u32(fade, (floor + if k % 2 == 0 { 0.5 } else { 0.0 }).to_bits());
        junk_floats(
            &mut t,
            f,
            &[
                0x19c, 0x4c, 0x8c, 0x788, 0x798, 0x81b, 0x1197, 0x6e8, 0x10a8, 0xdf5,
            ]
            .map(o),
            k,
        );
        // the static list's flag word (bit 0 cleared), the part's
        // environment pointer and its value
        t.set_u32(w(&t, 0x64e), 0x5a5a_0f0f ^ (k << 4));
        let env = SCRATCH + 0x7a80;
        t.set_u32(w(&t, 0x914), env);
        t.set_u32(env, 0x1111);
        t.set_u32(w(&t, 0x91a), 0x2222 + k);
        ownb::dirty_stack(&mut t, rva + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// One case of [`main_task_0042`]: the state, the count `R + 0x2a98`
/// (before the call's step), the timers `R + 0x290c`/`R + 0x2a90`, the
/// bone step `R + 0x2a94`, and a variant `v`: bit 0 slot 20 answers, bit
/// 1 slot 549 answers, bit 3 `R + 0x78` negative, bit 4 the fade words
/// in order.
#[derive(Clone, Copy)]
struct Case0042 {
    state: u32,
    count: i32,
    t1: f32,
    t2: f32,
    step: u32,
    v: u32,
}

const fn c042(state: u32, count: i32, t1: f32, t2: f32, step: u32, v: u32) -> Case0042 {
    Case0042 {
        state,
        count,
        t1,
        t2,
        step,
        v,
    }
}

/// `magic_0042` 0x10009490 (`r46::main_task_0042`): every state waiting
/// and going on, the chains (0 into 0x64, 0x66 into 0x5a into 0x5b into
/// 0x14, 0x28 into 0x32, 0xca into 0xcc, 0x46 into 0x48, 0x50 into 0x51
/// and the end), slots 20 and 549 both ways, the bones' steps, the
/// states past the table.
#[test]
fn main_task_0042() {
    let Some(mut t) = twin(42) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x9490);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 20, 1, move |m, a| (m.r32(answer) & 1) * (a[0] | 1));
    rec(&mut t, 549, 1, move |m, _| (m.r32(answer) >> 1) & 1);
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x7f8);
        m.w32(SCRATCH + 0x7f8, k + 0x0123_0457);
        k ^ 0xfc00
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    rec(&mut t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (307, 0.625)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the owned nodes' vtable visits through a stub
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let owned = t.img.rel32_target(f + 0x185);
    let vt = t.img.word(owned + 5);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let (lo, hi) = (-100.0f32, 1000.0f32);
    let cases = vec![
        c042(0, 0, 0.0, 0.0, 0, 0),
        c042(0x64, 0, 0.0, 0.0, 0, 0),
        c042(0x65, 0x17, 0.0, 0.0, 0, 0),
        c042(0x65, 0x18, 0.0, 0.0, 0, 0),
        c042(0x66, 0x53, 0.0, 0.0, 0, 0),
        c042(0x66, 0x54, lo, 0.0, 0, 0),
        c042(0x66, 0x54, hi, lo, 0, 0),
        c042(0x5a, 0, lo, 0.0, 0, 0),
        c042(0x5a, 0, hi, lo, 0, 1),
        c042(0x5b, 0, lo, 0.0, 0, 0),
        c042(0x5b, 0x2c, hi, hi, 0, 0),
        c042(0x5b, 0x2c, hi, lo, 0, 1),
        c042(0x14, 0, 0.0, lo, 0, 0),
        c042(0x14, 0, 0.0, lo, 0, 1),
        c042(0x14, 0, 0.0, hi, 2, 0),
        c042(0x14, 0x2c, 0.0, hi, 3, 1),
        c042(0x14, 0x2c, 0.0, hi, 4, 0),
        c042(0x1e, 0, 0.0, 0.0, 0, 0),
        c042(0x1f, 0, 0.0, 0.0, 0, 0),
        c042(0x1f, 0, 0.0, 0.0, 0, 0x10),
        c042(0x20, 0, 0.0, 0.0, 0, 0),
        c042(0x20, 0, 0.0, 0.0, 0, 8),
        c042(0x20, 0, 0.0, 0.0, 0, 9),
        c042(0x28, 0, 0.0, lo, 0, 0),
        c042(0x28, 0, 0.0, hi, 0, 0),
        c042(0x28, 0, 0.0, hi, 0, 2),
        c042(0x32, 0, 0.0, 0.0, 0, 0),
        c042(0x32, 0, 0.0, 0.0, 0, 2),
        c042(0xc8, 0, 0.0, 0.0, 0, 0),
        c042(0xc9, 0, 0.0, 0.0, 0, 0),
        c042(0xc9, 1, 0.0, 0.0, 0, 0),
        c042(0xca, 0, 0.0, lo, 0, 0),
        c042(0xca, 0, 0.0, 2.0, 0, 0),
        c042(0xca, 0, 0.0, hi, 0, 0),
        c042(0xcb, 0, 0.0, lo, 0, 0),
        c042(0xcb, 0, 0.0, hi, 0, 0),
        c042(0xcc, 0, 0.0, lo, 0, 0),
        c042(0xcc, 0, 0.0, hi, 0, 0),
        c042(0x46, 0, 0.0, 0.0, 0, 0),
        c042(0x46, 0, 0.0, 0.0, 0, 1),
        c042(0x47, 0, 0.0, 0.0, 0, 2),
        c042(0x48, 0, 0.0, 0.0, 0, 0),
        c042(0x50, 0, 0.0, 0.0, 0, 0),
        c042(0x51, 0, 0.0, lo, 0, 0),
        c042(0x51, 0, 0.0, hi, 0, 0),
        c042(0x2710, 0, 0.0, 0.0, 0, 0),
        c042(0x2711, 0, 0.0, 0.0, 0, 0),
        c042(0xcd, 0, 0.0, 0.0, 0, 0),
        c042(5, 0, 0.0, 0.0, 0, 0),
        c042(0xffff_fff0, 0, 0.0, 0.0, 0, 0),
    ];
    for (k, cs) in (0u32..).zip(cases) {
        t.write(r, &vec![0u8; 0x2aa0]);
        // the record's floats and words: junk-ish but finite
        for o in (0u32..0x2a80).step_by(0x40) {
            t.set_u32(r + o + 0x1c, (0.5f32 + (o >> 6) as f32).to_bits());
        }
        for o in [
            0x94cu32, 0x974, 0x99c, 0x9c4, 0xa38, 0xa40, 0xa90, 0xa64, 0xa6c, 0xabc,
        ] {
            t.set_u32(r + o, 0x6c6c_0000 + o);
        }
        t.set_u32(r + 0x2a88, cs.state);
        t.set_u32(r + 0x2a98, cs.count.cast_unsigned());
        set_f(&mut t, r + 0x2a8c, cs.t1);
        set_f(&mut t, r + 0x2a90, cs.t2);
        t.set_u32(r + 0x2a94, cs.step);
        t.set_u32(
            r + 0x78,
            if cs.v & 8 != 0 {
                0xffff_ff00
            } else {
                0x100 + k
            },
        );
        t.set_u32(answer, cs.v & 3);
        // the fade's two words: in order (the fade runs) or not
        let (fa, fb) = if cs.v & 0x10 != 0 {
            (2.0f32, 1.0f32)
        } else {
            (1.0, 2.0)
        };
        let (ga, gb) = (w(&t, 0xae6), w(&t, 0xaed));
        set_f(&mut t, ga, fa);
        set_f(&mut t, gb, fb);
        junk_floats(
            &mut t,
            f,
            &[
                0xd8, 0x328, 0x821, 0x831, 0x841, 0xb8a, 0xbf8, 0xc08, 0xc5a, 0xdbf, 0xde1, 0xe03,
            ],
            k,
        );
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the DLL's lists empty or with a last child
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        for op in [
            0xe4u32, 0x135, 0x202, 0x22d, 0x372, 0x37c, 0x401, 0x57a, 0x5b2, 0x610, 0x74f, 0xd96,
        ] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, if k % 3 == 1 { kid } else { l });
            t.set_u32(l + 0xc, l);
            t.set_u16(l + 0x18, 2);
        }
        ownb::dirty_stack(&mut t, 0x9490 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0071` 0x100076e0 (`r46::main_task_0071`): state 0 with the
/// light's flag both ways (slot 333 or 601), 0xa waiting and stepping for
/// each of its three steps (the last on), 0xb and 0x32 on both sides of
/// their bounds (0xb into 0x32 into the end), the parts' flags both ways
/// and parents or not, 10000, the states between and past; `P`'s list
/// empty and not.
#[test]
fn main_task_0071() {
    let Some(mut t) = twin(71) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x76e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]);
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let p = w(&t, 0x11a);
    // (state, `R + 0x38`, the bound's side, the flag bits, lists empty)
    let mut cases: Vec<(u32, u32, bool, u8, bool)> = vec![
        (0, 0, true, 0, true),
        (0, 0, true, 1, false),
        (0, 0, false, 1, true),
        (0xa, 0, false, 0, true),
        (0xa, 0, true, 0, false),
        (0xa, 1, true, 0, true),
        (0xa, 2, true, 0, true),
        (0xb, 0, false, 0, true),
        (0xb, 0, true, 0, true),
        (0xb, 0, true, 4, false),
        (0x32, 0, false, 0, true),
        (0x32, 0, true, 0, true),
        (0x32, 0, true, 4, false),
        (0x2710, 0, true, 0, true),
        (0x2711, 0, true, 0, true),
        (5, 0, true, 0, true),
        (0x33, 0, true, 0, true),
        (0x1000, 0, true, 0, true),
    ];
    cases.push((0xc, 0, true, 0, true));
    // (bit 3: 0x32 waits after 0xb; bit 4: 0xa's bound met exactly)
    cases.push((0xb, 0, true, 8, true));
    cases.push((0, 2, true, 0, true));
    cases.push((0xa, 1, false, 0x10, true));
    for (k, (state, step, go, bits, empty)) in (0u32..).zip(cases) {
        let bits_flag = bits & 7;
        junk(&mut t, r, 0x60, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x38, step);
        set_f(&mut t, r + 0x2c, 0.5 + k as f32 / 8.0);
        set_f(&mut t, r + 0x28, if bits & 0x10 != 0 { 0.0 } else { 1.0 });
        t.set_u32(a, SCRATCH + 0x3600);
        t.set_u32(a + 4, r);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // `P`'s list empty or with a last child
        let blk = SCRATCH + 0x7800;
        junk(&mut t, blk, 0x20, k);
        t.set_u32(p + 0x10, if empty { p } else { blk });
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 3);
        // the particle and effect nodes' lists: empty
        for op in [0x31du32, 0x35c, 0x77b, 0xa93, 0xc33, 0xd6e] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        // the light's flag (bit 0)
        let fl = w(&t, 0x58d);
        t.set_u8(fl, bits_flag | 0xa0);
        // the bounds: 0xb's `K >= G`, 0x32's `G >= K`, 0xa's `G - t >= s K`
        let kb = f32::from_bits(t.u32(w(&t, 0x29)));
        let gb = w(&t, 0x43);
        set_f(&mut t, gb, if go { kb - 1.0 } else { kb + 1.0 });
        let k32 = f32::from_bits(t.u32(w(&t, 0xba)));
        let g32 = w(&t, 0xb3);
        let run32 = go && bits & 8 == 0;
        set_f(&mut t, g32, if run32 { k32 + 1.0 } else { k32 - 1.0 });
        let gt = w(&t, 0xae0);
        let exact = 0.5 + k as f32 / 8.0;
        let kx = f32::from_bits(t.u32(w(&t, 0xaf0)));
        let gval = if bits & 0x10 != 0 {
            exact * kx
        } else if go {
            1000.0
        } else {
            -1000.0
        };
        set_f(&mut t, gt, gval);
        // (0x32's word is 0xa's `G` too: 0xb and 0x32 cases set it last)
        if state == 0xb || state == 0x32 {
            set_f(&mut t, g32, if run32 { k32 + 1.0 } else { k32 - 1.0 });
        }
        // the eight parts: every other one with a parent, its flag bit 2
        // by the case
        let p0 = w(&t, 0x3cf);
        for i in 0..8u32 {
            let part = p0 + 0x100 * i;
            let n = part - 4;
            let par = SCRATCH + 0x6000 + 0x40 * i;
            t.write(par, &[0; 0x40]);
            if i % 2 == 0 {
                t.set_u32(n + 0x14, par);
                t.set_u32(n + 4, par);
                t.set_u32(n + 8, par);
                t.set_u32(par + 0x10, n);
                t.set_u32(par + 0xc, n);
                t.set_u16(par + 0x18, 1);
            } else {
                t.set_u32(n + 0x14, 0);
            }
            t.set_u32(part + 0x4c, 0x4c00 + i + (k << 8));
            t.set_u8(part + 0xe4, if i % 3 == 0 { bits_flag } else { 0 });
        }
        let env = SCRATCH + 0x7a80;
        for op in [0x3e1u32, 0x449] {
            t.set_u32(w(&t, op), env);
        }
        t.set_u32(env, 0x1111);
        junk_floats(
            &mut t,
            f,
            &[0x398, 0x5bf, 0x5ee, 0x8d7, 0x906, 0x2d0, 0x72e, 0xa46],
            k,
        );
        ownb::dirty_stack(&mut t, 0x76e0 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0071` 0x100084c0 (`r46::scene_task_0071`): state 0 into 0xa,
/// 0xb and 0x14 on both sides of their waits (0x14 into 0x15), the aim's
/// length 0 and not and its floor both ways, slot 411 answering and not
/// for 0x15, 0x3c, 0x3d and 10000, the states between and past; `P`'s
/// list empty and not.
#[test]
fn scene_task_0071() {
    let Some(mut t) = twin(71) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x84c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 411, 1, move |m, _| m.r32(answer));
    // slot 268 fills the vector it is handed
    rec(&mut t, 268, 3, |m, a| {
        for (i, x) in (0u32..).zip([1.5f32, -2.0, 4.0]) {
            wf(m, a[2] + 4 * i, x);
        }
        0x268
    });
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 + (a[1] & 7) as f32);
        }
        0x237
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let p = w(&t, 0x139);
    // (state, the wait `R + 0x2c`, slot 411, the variant: bit 0 the aim's
    // length 0, bit 1 its floor wins, lists empty)
    let mut cases: Vec<(u32, i32, u32, u32, bool)> = vec![
        (0, 0, 0, 0, true),
        (0, 0, 0, 0, false),
        (0xa, 0, 0, 0, true),
        (0xb, 0x27, 0, 0, true),
        (0xb, 0x28, 0, 0, true),
        (0x14, 0x13, 0, 0, true),
        (0x14, 0x14, 0, 0, true),
        (0x14, 0x14, 1, 0, false),
        (0x14, 0x14, 1, 1, true),
        (0x14, 0x14, 1, 2, true),
        (0x14, 0x14, 1, 3, false),
        (0x15, 0, 0, 0, true),
        (0x15, 0, 1, 0, true),
        (0x3c, 0, 0, 0, true),
        (0x3c, 0, 1, 0, true),
        (0x3d, 0, 0, 0, true),
        (0x3d, 0, 1, 0, true),
        (0x2710, 0, 0, 0, true),
        (0x2710, 0, 1, 0, true),
        (5, 0, 0, 0, true),
        (0x13, 0, 0, 0, true),
        (0x16, 0, 0, 0, true),
        (0x3e, 0, 0, 0, true),
        (0x2711, 0, 0, 0, true),
    ];
    cases.push((0x1000, 0, 0, 0, true));
    for (k, (state, wait, ans, v, empty)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x60, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x2c, wait.cast_unsigned());
        t.set_u32(answer, ans);
        t.set_u32(a, SCRATCH + 0x3600);
        t.set_u32(a + 4, r);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        let blk = SCRATCH + 0x7800;
        junk(&mut t, blk, 0x20, k);
        t.set_u32(p + 0x10, if empty { p } else { blk });
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 3);
        let qblk = SCRATCH + 0x7900;
        junk(&mut t, qblk, 0x20, k);
        t.set_u32(w(&t, 0x9f), qblk);
        for op in [0x51fu32, 0xa33, 0xb95] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        junk_floats(
            &mut t,
            f,
            &[
                0x55d, 0x585, 0x5c0, 0x5fb, 0x61d, 0x630, 0x640, 0x648, 0x65b, 0x7b9, 0x7ec, 0x827,
            ],
            k,
        );
        junk_floats(
            &mut t,
            f,
            &[0x861, 0x89c, 0x8b4, 0x8bc, 0x8cc, 0x8d4],
            k + 1,
        );
        // the flag words (bit 0 set, junk around)
        for (i, op) in (0u32..).zip([0xe6u32, 0x562, 0x7c4, 0xa69]) {
            t.set_u32(w(&t, op), 0x5a5a_0f0f ^ (k << 4) ^ (i << 8));
        }
        t.set_u16(w(&t, 0x31d), 0x5a5e ^ (k as u16));
        // the fade's words the states compute (junk first)
        for (i, op) in (0u32..).zip([
            0x57au32, 0x5b5, 0x5f0, 0x612, 0x625, 0x638, 0x650, 0x663, 0x7e1, 0x81c, 0x856, 0x891,
            0x8a9, 0x8c4, 0x8dc, 0x8e4,
        ]) {
            t.set_u32(w(&t, op), 0x6e6e_0000 ^ (k << 8) ^ i);
        }
        // the target: on `V` (length 0) or off it; the floor above `K''
        // + dy` or not
        let (tx, tz) = if v & 1 != 0 {
            (1.5f32, 4.0f32)
        } else {
            (7.25, -3.5)
        };
        let (gx, gz) = (w(&t, 0x156), w(&t, 0x16c));
        set_f(&mut t, gx, tx);
        set_f(&mut t, gz, tz);
        let gy = w(&t, 0x174);
        set_f(&mut t, gy, if v & 2 != 0 { -1.0e4 } else { 1.0e4 });
        ownb::dirty_stack(&mut t, 0x84c0 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0070` 0x1000b6e0 (`r46::main_task_0070`): every state on both
/// sides of its wait (and the chains: 0 into 0xa, 0x68 into 0x5a, 0x60
/// into 10000, 0x3c into 0x3d, 0x48 into 0x49), slots 411 and 313 both
/// ways, the float bound of 0x28, the level word below 4, the end
/// (10000..10002), the states between and past the table; the lists
/// empty and not.
#[test]
fn main_task_0070() {
    let Some(mut t) = twin(70) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0xb6e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 411, 1, move |m, _| m.r32(answer) & 1);
    rec(&mut t, 313, 1, move |m, _| (m.r32(answer) >> 1) & 1);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + 1.0;
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 + (a[1] & 7) as f32);
        }
        0x237
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    t.stub_edx(771, |_, a| 0x0077_1000 ^ a[0]);
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (277, 4.0)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the owned nodes' vtable visits through a stub
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let owned = t.img.rel32_target(f + 0x423);
    let vt = t.img.word(owned + 5);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    // the DLL's other node classes' visits (overlay code the original
    // runs inline: the sub walk 0x10006950, the visitors 0x100059d0 and
    // 0x10008460)
    for rva in [
        0x1_0534u32,
        0x1_0358,
        0x1_03b4,
        0x1_0454,
        0x1_04f4,
        0x1_0630,
        0x1_06b0,
        0x1_055c,
    ] {
        t.set_u32(t.at(rva), magic::TRAPS + 16 * visit);
    }
    // the summon bones' function table (behind the bone rate's call)
    let bone = t.img.rel32_target(t.img.rel32_target(f + 0xed0) + 0x15);
    let table = t.img.word(bone + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let lists = [w(&t, 0x383), w(&t, 0x56d), w(&t, 0x616), w(&t, 0x8fa)];
    // (state, the wait `R + 0x30`, the answers (bit 0 slot 411, bit 1
    // slot 313), the variant (bit 0 the level word low, bit 1 0x28's
    // float below its bound), lists empty)
    let mut cases: Vec<(u32, i32, u32, u32, bool)> = vec![
        (0, 0, 0, 0, true),
        (0, 0, 0, 0, false),
        (0, 0, 0, 1, true),
        (0xa, 0, 0, 0, true),
        (0xb, 0, 0, 0, true),
        (0xc, 0, 0, 0, true),
        (0xc, 0, 1, 0, true),
        (0xc, 0, 1, 0, false),
        (0x14, 0, 0, 0, true),
        (0x15, 1, 0, 0, true),
        (0x15, 2, 0, 0, true),
        (0x16, 0x58, 0, 0, true),
        (0x16, 0x59, 0, 0, true),
        (0x1e, 0x4f, 0, 0, true),
        (0x1e, 0x50, 0, 0, false),
        (0x1f, 0x4f, 0, 0, true),
        (0x1f, 0x50, 0, 0, true),
        (0x20, 0x59, 0, 0, true),
        (0x20, 0x5a, 0, 0, true),
        (0x28, 0, 2, 0, true),
        (0x28, 0, 0, 2, true),
        (0x28, 0, 0, 0, true),
        (0x29, 0, 0, 0, true),
        (0x29, 0, 1, 0, true),
        (0x32, 0x13, 0, 0, true),
        (0x32, 0x14, 0, 0, true),
        (0x33, 0x1d, 0, 0, true),
        (0x33, 0x1e, 0, 0, true),
        (0x34, 0x20, 0, 0, true),
        (0x34, 0x21, 0, 0, false),
        (0x3c, 0x74, 1, 0, true),
        (0x3c, 0x75, 1, 0, true),
        (0x3c, 0x75, 0, 0, true),
        (0x3d, 0, 0, 0, true),
        (0x3d, 0, 1, 0, true),
        (0x3e, 7, 0, 0, true),
        (0x3e, 8, 0, 0, false),
        (0x46, 0x27, 0, 0, true),
        (0x46, 0x28, 0, 0, true),
        (0x47, 0x35, 0, 0, true),
        (0x47, 0x36, 0, 0, false),
        (0x48, 0x6f, 0, 0, true),
        (0x48, 0x70, 0, 0, true),
        (0x48, 0x74, 0, 0, true),
        (0x49, 0x73, 0, 0, true),
        (0x49, 0x74, 0, 0, true),
        (0x50, 0, 0, 0, true),
        (0x51, 0, 0, 0, true),
        (0x51, 0, 1, 0, true),
        (0x5a, 0, 0, 0, true),
        (0x5b, 0, 0, 0, true),
        (0x5c, 0x1f, 0, 0, true),
        (0x5c, 0x20, 0, 0, false),
        (0x5d, 0x34, 0, 0, true),
        (0x5d, 0x35, 0, 0, true),
        (0x5e, 0x3b, 0, 0, true),
        (0x5e, 0x3c, 0, 0, true),
        (0x5f, 0x63, 0, 0, true),
        (0x5f, 0x64, 0, 0, true),
        (0x60, 0x7c, 0, 0, true),
        (0x60, 0x7d, 0, 0, true),
        (0x64, 0, 0, 0, true),
        (0x65, 0, 0, 0, true),
        (0x65, 1, 0, 0, true),
        (0x66, 0xb, 0, 0, true),
        (0x66, 0xc, 0, 0, true),
        (0x67, 0xb, 0, 0, true),
        (0x67, 0xc, 0, 0, true),
        (0x68, 0x1d, 0, 0, true),
        (0x68, 0x1e, 0, 0, true),
        (0x2710, 0, 0, 0, true),
        (0x2711, 0, 0, 0, true),
        (0x2712, 0, 0, 0, true),
        (0x2713, 0, 0, 0, true),
        (5, 0, 0, 0, true),
        (0x69, 0, 0, 0, true),
    ];
    cases.push((0x4e20, 0, 0, 0, true));
    for (k, (state, wait, ans, v, empty)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x60, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x30, wait.cast_unsigned());
        set_f(&mut t, r + 0x20, 1.25 + k as f32 / 16.0);
        set_f(&mut t, r + 0x24, -0.75 + k as f32 / 32.0);
        t.set_u32(answer, ans);
        t.set_u32(w(&t, 0x16), if v & 1 != 0 { 3 } else { 4 + (k & 3) });
        t.set_u32(a, SCRATCH + 0x3600);
        t.set_u32(a + 4, r);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        for (i, &l) in (0u32..).zip(&lists) {
            let blk = SCRATCH + 0x7800 + 0x40 * i;
            junk(&mut t, blk, 0x20, k + i);
            t.set_u32(l + 0x10, if empty { l } else { blk });
            t.set_u32(l + 0xc, l);
            t.set_u16(l + 0x18, 1 + i as u16);
        }
        // 0x28's float on either side of its bound
        let kb = f32::from_bits(t.u32(w(&t, 0xdb6)));
        let gb = w(&t, 0xdaf);
        set_f(&mut t, gb, if v & 2 != 0 { kb - 1.0 } else { kb + 1.0 });
        junk_floats(
            &mut t,
            f,
            &[
                0x5e, 0x6e, 0x395, 0x3a5, 0x3cc, 0x4d3, 0x565, 0x588, 0x654, 0x108e, 0x12ac,
                0x12d8, 0x12fc,
            ],
            k,
        );
        for (i, op) in (0u32..).zip([0x1fau32, 0x255, 0x6fc, 0x8f4, 0xfb4, 0x10ec, 0x10f3, 0x1166])
        {
            t.set_u32(w(&t, op), 0x5a5a_0f0f ^ (k << 4) ^ (i << 8));
        }
        ownb::dirty_stack(&mut t, 0xb6e0 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}
