//! Round 37's ports (`ffx_magic::r37`) against each original. Needs
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
use summon_stubs::{all_stubs, prime, rf, visit_stub, wf};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn set_f(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// One case of [`camera_task_0570`]: the state, the wait `R + 0x38`, the
/// flag byte (bit 0 of the slot-666 test), the list's count, and a
/// variant `v` that moves the compared globals across their limits.
#[derive(Clone, Copy)]
struct Case0570 {
    state: u32,
    wait: i32,
    flag: u8,
    count: u32,
    v: u32,
}

const fn c570(state: u32, wait: i32) -> Case0570 {
    Case0570 {
        state,
        wait,
        flag: 0,
        count: 0,
        v: 0,
    }
}

/// `magic_0570` 0x10009b30 (`r37::camera_task_0570`): every state of the
/// three tables waiting and going on, the chains (0x96 into 0x98, 0xaa
/// into 0xac, 0x190 into 0x192, 0x19c into 0x19d, 0x3e9 into 0x3ec), the
/// slot-666 flag both ways, slot 288's list (empty, every kind of byte,
/// a count slot 296 lowers), the fade at and off 0, the light's and the
/// move's limits both ways, 0x2710, and the states between and past the
/// tables.
#[test]
fn camera_task_0570() {
    let Some(mut t) = twin(570) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x9b30);
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
    for (c, op) in [(0x13fu32, 5u32), (0xe0, 0x2f), (0x103, 2)] {
        let vt = t.img.word(call(&t, c) + op);
        t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    }
    let bone = call(&t, 0x1fd);
    let table = t.img.word(bone + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    // the move's ends read `to`'s z (E + 0x48) and, in 0x19b and 0x2bc,
    // `from`'s z (E + 0x18) before anything writes them: 0 as the port
    let junk = [
        ownb::ENTRY_ESP - (base + 0x54),
        ownb::ENTRY_ESP - (base + 0x24),
    ];
    let mut cases = vec![
        c570(0, 0),
        c570(0xb4, 0),
        c570(0x320, 0),
        c570(0xa0, 5),
        c570(0x2c0, 0),
        c570(0x97, 0),
        c570(0x98, 0),
        c570(0x98, 0x46),
        c570(0x98, 0x40),
        c570(0xa1, 0x18),
        c570(0xa1, 0x19),
        c570(0xc8, 0),
        c570(0xc9, 0),
        Case0570 {
            v: 1,
            ..c570(0xc9, 0)
        },
        Case0570 {
            v: 3,
            ..c570(0xc9, 0)
        },
        c570(0xaa, 0),
        c570(0xab, 0),
        c570(0xac, 0),
        c570(0x190, 0),
        c570(0x191, 3),
        c570(0x191, 4),
        c570(0x192, 0),
        Case0570 {
            v: 1,
            ..c570(0x192, 0)
        },
        Case0570 {
            v: 2,
            ..c570(0x192, 0)
        },
        Case0570 {
            v: 3,
            ..c570(0x192, 0)
        },
        c570(0x193, 0),
        c570(0x19a, 0),
        c570(0x19b, 9),
        c570(0x19b, 0xa),
        c570(0x19c, 0x18),
        c570(0x19c, 0x19),
        c570(0x19d, 0),
        Case0570 {
            v: 1,
            ..c570(0x19d, 0)
        },
        Case0570 {
            v: 3,
            ..c570(0x19d, 0)
        },
        c570(0x19e, 0),
        c570(0x2bc, 0),
        c570(0x2bd, 0),
        c570(0x2be, 0),
        c570(0x2bf, 0),
        c570(0x321, 0),
        c570(0x384, 0),
        c570(0x385, 0x45),
        c570(0x385, 0x46),
        c570(0x386, 0x13),
        c570(0x386, 0x14),
        c570(0x387, 0x40),
        c570(0x387, 0x41),
        c570(0x388, 0),
        c570(0x3e8, 0),
        c570(0x3e9, 9),
        c570(0x3e9, 0xa),
        c570(0x3ea, 9),
        c570(0x3ea, 0xa),
        c570(0x3eb, 0x1d),
        c570(0x3eb, 0x1e),
        c570(0x3ec, 0),
        c570(0x3ed, 0),
        c570(0x2710, 0),
        c570(0x2711, 0),
        c570(0xca, 0),
        c570(5, 0),
        c570(0xffff_fff0, 0),
    ];
    for (flag, count) in [(0u8, 0u32), (1, 0), (0, 7), (1, 9)] {
        cases.push(Case0570 {
            flag,
            count,
            ..c570(0x96, 0)
        });
    }
    // the list: the DLL's byte, the last's, 0xff, others (0x33 lowers the
    // count by 2)
    let own = t.u32(w(&t, 0x3b4)) as u8;
    let last_byte = 0x44u8;
    for (k, cs) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x100]);
        t.set_u32(r + 0x18, cs.state);
        set_f(&mut t, r + 0x1c, 0.75 + k as f32);
        t.set_u32(r + 0x34, 0x100 + k);
        t.set_u32(r + 0x38, cs.wait.cast_unsigned());
        t.set_u32(r + 0x3c, 0x30 + k);
        t.set_u8(w(&t, 0x94), cs.flag);
        t.set_u32(SCRATCH + 0x7af0, cs.count);
        t.write(
            list,
            &[
                0x21, own, last_byte, 0xff, 0x33, 0x52, 0x60, own, 0x7f, 0x80,
            ],
        );
        t.set_u32(w(&t, 0x3bf), u32::from(last_byte));
        // the flag words the states set and clear bits of
        for (i, o) in (0u32..).zip([0x221u32, 0x97f, 0xad4, 0xd79, 0x149a]) {
            t.set_u32(w(&t, o), 0x5a5a_0f0e ^ (k << 4) ^ i);
        }
        // (the climb's and 0x19d's step word is 0x192's `y`)
        t.set_u32(w(&t, 0x8c8), 0x2000 + k);
        // the move's words: 0x192's `x` above or at -0x800, `y` below or at
        // 0x5800; the light's limits (`K > L` or not)
        let (x, y) = match cs.v {
            1 => (-0x800i32, 0x5800i32),
            2 => (-0x7ff, 0x57ff),
            _ => (0x100, 0x200),
        };
        t.set_u32(w(&t, 0xa5e), x.cast_unsigned());
        t.set_u32(w(&t, 0xa72), y.cast_unsigned());
        // words the states clear or copy into, junk first
        for (i, o) in (0u32..).zip([0x11e5u32, 0x1289, 0x1337, 0xffb]) {
            t.set_u16(w(&t, o), 0x5a00 ^ (k as u16) ^ (i as u16));
        }
        for (i, o) in (0u32..).zip([0x1534u32, 0x153c, 0x14de, 0x14e6]) {
            t.set_u32(w(&t, o), 0x6b6b_0000 ^ k ^ (i << 8));
        }
        // the time left's effect and the model state's object: their parts
        for (i, o) in (0u32..).zip([0x10fu32, 0x191]) {
            let obj = w(&t, o);
            t.set_u32(obj + 0x50, 0x0700_0000 + 0x10 * i + k);
            t.set_u32(obj + 0x54, 0x0800_0000 + 0x10 * i + k);
        }
        // the move globals' floats and the light's place
        for (i, o) in (0u32..).zip([
            0x972u32, 0x97a, 0x98a, 0x99d, 0x9a5, 0x9d6, 0x9de, 0xa0f, 0xa17, 0x749, 0x93e, 0x11fb,
            0x1269, 0x1396, 0x1695, 0x16ad, 0x16c5, 0xfb1, 0x11af,
        ]) {
            let at = w(&t, o);
            set_f(&mut t, at, 1.5 * i as f32 - 4.0 + 0.125 * k as f32);
        }
        // the light's level (one global for the three states; after the
        // floats, `+0x93e` names it too): below, far
        // above, or equal to the state's limit
        let (o, lim) = match cs.state {
            0x19c | 0x19d => (0xc1bu32, 0xc0eu32),
            0xc9 => (0x777, 0x770),
            _ => (0xa44, 0xa37),
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
        let dll_list = w(&t, 0xd6);
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(dll_list + 0x10, if k % 3 == 1 { kid } else { dll_list });
        t.set_u32(dll_list + 0xc, dll_list);
        t.set_u16(dll_list + 0x18, 2);
        ownb::dirty_stack(&mut t, 0x9b30 + k, &junk);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// One case of [`main_task_0533`]: the state, the count `R + 0x2918`
/// (before the call's step), the timers `R + 0x290c`/`R + 0x2910`, the
/// step `R + 0x2914`, and a variant `v`: bit 0 slot 689 answers, bit 1
/// slot 20 answers, bit 2 the byte `R + 0xbb8`'s bit 2, bit 3 `R + 0x78`
/// negative, bit 4 slot 288's list holds bytes.
#[derive(Clone, Copy)]
struct Case0533 {
    state: u32,
    count: i32,
    t1: f32,
    t2: f32,
    step: u32,
    v: u32,
}

const fn c533(state: u32, count: i32, t1: f32, t2: f32, step: u32, v: u32) -> Case0533 {
    Case0533 {
        state,
        count,
        t1,
        t2,
        step,
        v,
    }
}

/// `magic_0533` 0x10009520 (`r37b::main_task_0533`): every state waiting
/// and going on, the chains (0 into 0x64, 0x65 into 0x5a into 0x5b into
/// 0x14, 0x20 into 0x28 into 0x32, 0x33 into 0x34 into 0x35 into 0x36,
/// 0x37 into 0x38 into 0x39 into 0x3a, 0x50 into 10000), slots 689, 20,
/// 549 (once busy) both ways, the bones' steps, slot 288's list, the
/// states past the table.
#[test]
fn main_task_0533() {
    let Some(mut t) = twin(533) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x9520);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    // slot 689 and 20 answer by the case; 549 is busy every other call
    rec(&mut t, 689, 0, move |m, _| m.r32(answer) & 1);
    rec(&mut t, 20, 1, move |m, a| {
        (m.r32(answer) >> 1 & 1) * (a[0] | 1)
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
        c533(0, 0, 0.0, 0.0, 0, 0),
        c533(0, 0, 0.0, 0.0, 0, 1),
        c533(0x64, 0, 0.0, 0.0, 0, 0),
        c533(0x65, 0x3a, 0.0, 0.0, 0, 0),
        c533(0x65, 0x3b, lo, 0.0, 0, 0),
        c533(0x65, 0x3b, hi, 0.0, 0, 0),
        c533(0x1e, 0, 0.0, 0.0, 0, 0),
        c533(0x5a, 0, lo, 0.0, 0, 0),
        c533(0x5a, 0, hi, lo, 0, 0),
        c533(0x5b, 0, lo, 0.0, 0, 0),
        c533(0x5b, 0x2c, hi, hi, 0, 0),
        c533(0x5b, 0x2c, hi, lo, 0, 0),
        c533(0x14, 0, 0.0, lo, 0, 0),
        c533(0x14, 0, 0.0, hi, 2, 0),
        c533(0x14, 0x2c, 0.0, hi, 3, 0),
        c533(0x14, 0x2c, 0.0, hi, 4, 0),
        c533(0x1f, 5, 0.0, 0.0, 0, 0),
        c533(0x1f, 6, 0.0, 0.0, 0, 0),
        c533(0x20, 0, 0.0, 0.0, 0, 0),
        c533(0x20, 0x10, 0.0, 0.0, 0, 8),
        c533(0x20, 0x76, 0.0, 0.0, 0, 8),
        c533(0x20, 0x76, 0.0, 0.0, 0, 0x18),
        c533(0x20, 0x77, 0.0, lo, 0, 8),
        c533(0x20, 0x77, 0.0, hi, 0, 0xc),
        c533(0x20, 0x77, 0.0, hi, 0, 0x8),
        c533(0x28, 0, 0.0, lo, 0, 0),
        c533(0x28, 0, 0.0, hi, 0, 0),
        c533(0x28, 0, 0.0, hi, 0, 1),
        c533(0x28, 0, 0.0, hi, 0, 3),
        c533(0x32, 0, 0.0, 0.0, 0, 0),
        c533(0x32, 0, 0.0, 0.0, 0, 1),
        c533(0x32, 0, 0.0, 0.0, 0, 2),
        c533(0x33, 5, 0.0, 0.0, 0, 0),
        c533(0x33, 0, 0.0, 0.0, 0, 0),
        c533(0x33, 1, 0.0, hi, 0, 0),
        c533(0x34, 0, 0.0, lo, 0, 0),
        c533(0x34, 0, 0.0, hi, 0, 0),
        c533(0x34, 0, 0.0, hi, 3, 0),
        c533(0x34, 0x1d, 0.0, lo, 3, 0),
        c533(0x35, 0, 0.0, hi, 0, 0),
        c533(0x35, 0, 0.0, hi, 3, 0),
        c533(0x35, 0x1d, 0.0, hi, 3, 0),
        c533(0x36, 0, 0.0, 0.0, 0, 0),
        c533(0x37, 0, 0.0, lo, 0, 0),
        c533(0x37, 0, 0.0, lo, 0, 1),
        c533(0x38, 0, 0.0, lo, 0, 0),
        c533(0x38, 0, 0.0, hi, 0, 0),
        c533(0x39, 0, 0.0, lo, 0, 0),
        c533(0x39, 0, 0.0, 2.0, 0, 0),
        c533(0x39, 0, 0.0, hi, 0, 0),
        c533(0x3a, 0, 0.0, lo, 0, 0),
        c533(0x3a, 0, 0.0, hi, 0, 0),
        c533(0x46, 0, 0.0, 0.0, 0, 0),
        c533(0x47, 0xb0, 0.0, 0.0, 0, 0),
        c533(0x47, 0xb1, 0.0, 0.0, 0, 0),
        c533(0x50, 0x7c, 0.0, 0.0, 0, 0),
        c533(0x50, 0x7d, 0.0, 0.0, 0, 0),
        c533(0x2710, 0, 0.0, 0.0, 0, 0),
        c533(0x2711, 0, 0.0, 0.0, 0, 0),
        c533(0x2712, 0, 0.0, 0.0, 0, 0),
        c533(0x66, 0, 0.0, 0.0, 0, 0),
        c533(5, 0, 0.0, 0.0, 0, 0),
        c533(0xffff_fff0, 0, 0.0, 0.0, 0, 0),
    ];
    // slot 288's list: the DLL's two bytes and others
    let own = t.u32(w(&t, 0xcf7));
    let skip = t.u32(w(&t, 0xcff)) as u8;
    cases.push(c533(0x20, 0x76, 0.0, 0.0, 0, 0x18));
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
        t.set_u32(answer, cs.v & 3);
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
            0x1a8u32, 0x1f1, 0x394, 0x39e, 0x422, 0x59b, 0x5d3, 0x631, 0x770, 0xe41, 0xf21, 0xff2,
            0x1068, 0x12eb, 0x138e, 0x140f,
        ] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, if k % 3 == 1 { kid } else { l });
            t.set_u32(l + 0xc, l);
            t.set_u16(l + 0x18, 2);
        }
        ownb::dirty_stack(&mut t, 0x9520 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}
