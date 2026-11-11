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
use summon_stubs::{all_stubs, prime, rf, visit_stub, wf};

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
        t.set_u8(w(&t, 0x113), cs.flag);
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
            set_f(&mut t, at, 1.5 * i as f32 - 4.0 + 0.125 * k as f32);
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
