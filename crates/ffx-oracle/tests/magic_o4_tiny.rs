//! The smallest own functions (`ffx_magic::o4_tiny`) against every DLL's
//! own copy, and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::Xmm;
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its name, copies, how many set-ups, the stack words
/// and the set-up.
type Case = (
    &'static str,
    &'static [(u16, u32)],
    u32,
    &'static [u32],
    SetUp,
);

/// `*G` (the pointer global at `op`) = `P`, the words around `P + D` (`D`
/// at each of `fields`) filled.
fn ptr(t: &mut Twin, f: u32, op: u32, fields: &[u32], seed: u32) {
    t.set_u32(t.img.word(f + op), P);
    for &d in fields {
        fill(t, P + t.img.word(f + d) - 4, 3, seed ^ d);
    }
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
    }
}

/// The script of `run` (`sa_script::run`) ended (`mode` 0) or waiting
/// for ever (2).
fn script(t: &mut Twin, run: u32, mode: u8) {
    // the script's mode byte: 0 ended (returns 1), 2 waiting forever
    t.set_u8(t.img.word(run + 0x01), mode);
    let s = t.img.word(run + 0x1c);
    t.set_u16(s + 0x54, 0xffff);
    t.set_u32(s + 0x24, 0);
    t.set_u32(s + 0x28, 0);
}

fn none(_: &mut Twin, _: u32, _: u32) {}

const CASES: &[Case] = &[
    ("set_bit_global", &[(307, 0x5220)], 1, &[], |t, f, _| {
        t.set_u32(t.img.word(f + 0x03), 0x5a5a_1234);
    }),
    ("copy_global", &[(269, 0x1740)], 1, &[], |t, f, c| {
        globals(t, f, &[0x01, 0x06], c);
    }),
    ("set_one_ret0", &[(671, 0x1330)], 1, &[], |t, f, c| {
        globals(t, f, &[0x02], c);
    }),
    ("set_bit_field", &[(563, 0x50f0)], 1, &[], |t, f, c| {
        ptr(t, f, 0x01, &[0x08], c);
        t.set_u32(P + t.img.word(f + 0x08), 0x5a5a_1234);
    }),
    ("clear_bit_field", &[(563, 0x5100)], 1, &[], |t, f, c| {
        ptr(t, f, 0x01, &[0x0d], c);
    }),
    (
        "is_zero_field",
        &[(434, 0x95c0), (610, 0xd490)],
        2,
        &[],
        |t, f, c| {
            ptr(t, f, 0x02, &[0x0a], c);
            if c == 1 {
                t.set_u32(P + t.img.word(f + 0x0a), 0);
            }
        },
    ),
    ("slot_ecx", &[(577, 0xeca0)], 1, &[], none),
    ("slot_imm8", &[(577, 0xec80)], 1, &[], none),
    ("slot_imm8x3", &[(519, 0x4e40)], 1, &[], none),
    ("copy_to_arg", &[(673, 0x1b40)], 1, &[OBJ], |t, f, c| {
        ptr(t, f, 0x04, &[0x0a], c);
        fill(t, OBJ + t.img.word(f + 0x13) - 4, 3, c);
    }),
    ("two_stores", &[(350, 0x6300)], 1, &[], |t, f, c| {
        ptr(t, f, 0x01, &[0x07, 0x11], c);
    }),
    ("slot_7_0_arg", &[(150, 0xeaf0)], 1, &[0x00a1_0000], none),
    ("set_callback", &[(353, 0x2440)], 1, &[OBJ], |t, f, c| {
        fill(t, OBJ + t.img.word(f + 0x08) - 4, 3, c);
        fill(t, OBJ + t.img.word(f + 0x12) - 4, 3, c);
    }),
    ("slot_then_flag", &[(195, 0x19c0)], 1, &[OBJ], |t, _, c| {
        fill(t, OBJ, 2, c);
    }),
    ("slot_two_globals", &[(483, 0x2650)], 1, &[], |t, f, c| {
        globals(t, f, &[0x02, 0x0d], c);
    }),
    (
        "add_const",
        &[(568, 0x2de0), (673, 0x2040)],
        1,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x01), P);
            t.set_u32(P + t.img.word(f + 0x09), (1.5f32 + c as f32).to_bits());
        },
    ),
    ("script_then_end", &[(671, 0x1310)], 2, &[], |t, f, c| {
        globals(t, f, &[0x11], c);
        script(t, t.img.rel32_target(f + 0x01), if c == 0 { 0 } else { 2 });
    }),
    ("slot_float", &[(263, 0x8980)], 1, &[], none),
    (
        "slot237_edx",
        &[(483, 0x26a0)],
        1,
        &[0x1111, 0x2222],
        |t, f, c| {
            globals(t, f, &[0x16], c);
        },
    ),
    ("slot_keep_nonzero", &[(661, 0x15b0)], 2, &[], |t, f, c| {
        globals(t, f, &[0x1a], c);
        // slot 260's answer: its word, or 0
        t.set_u32(SCRATCH + 0x700, c);
    }),
    (
        "slot237_arg",
        &[(343, 0x6970)],
        1,
        &[0x00a2_0000],
        |t, f, c| {
            globals(t, f, &[0x17], c);
        },
    ),
    (
        "slot_then_store_ret1",
        &[(443, 0x1ab0)],
        1,
        &[],
        |t, f, c| {
            globals(t, f, &[0x14], c);
        },
    ),
    ("slot_then_clear", &[(499, 0x13c0)], 1, &[], |t, f, c| {
        globals(t, f, &[0x07], c);
    }),
    ("set_bits_indexed", &[(165, 0x36f0)], 2, &[], |t, f, c| {
        ptr(t, f, 0x02, &[0x08, 0x1e], c);
        t.set_u32(P + t.img.word(f + 0x08), c * 3);
        let rec = P + c * 3 * 0x130 + t.img.word(f + 0x16);
        fill(t, rec - 4, 3, c);
        t.set_u32(rec, 0x5a5a_1234);
        t.set_u32(P + t.img.word(f + 0x1e), 0x5a5a_4320);
    }),
    ("clear_bits_indexed", &[(165, 0x3720)], 2, &[], |t, f, c| {
        ptr(t, f, 0x02, &[0x0d, 0x22], c);
        t.set_u32(P + t.img.word(f + 0x0d), c * 3);
        let rec = P + c * 3 * 0x130 + t.img.word(f + 0x1b);
        fill(t, rec - 4, 3, c);
    }),
    ("slot_obj_word", &[(540, 0x1000)], 1, &[OBJ], |t, _, c| {
        fill(t, OBJ, 1, c);
    }),
    ("set_four", &[(263, 0x6f40)], 1, &[], |t, f, c| {
        globals(t, f, &[0x02, 0x0c, 0x12, 0x1c], c);
    }),
    (
        "add_const_twice",
        &[(162, 0x3990), (586, 0x35a0)],
        1,
        &[],
        |t, f, c| {
            ptr(t, f, 0x01, &[0x09, 0x21], c);
            t.set_u32(P + t.img.word(f + 0x09), (-2.5f32).to_bits());
        },
    ),
    ("sub_const_twice", &[(304, 0x2df0)], 1, &[], |t, f, c| {
        ptr(t, f, 0x01, &[0x09, 0x21], c);
        t.set_u32(P + t.img.word(f + 0x09), 7.25f32.to_bits());
    }),
    (
        "field_slot_then_flag",
        &[(525, 0x1750)],
        1,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x04, &[0x0a], c);
            fill(t, OBJ, 2, c);
        },
    ),
    (
        "script_then_give_back",
        &[(638, 0x3170), (689, 0x3170)],
        2,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x06, &[0x19], c);
            script(t, t.img.rel32_target(f + 0x0b), if c == 0 { 0 } else { 2 });
            // the node: alone, its own child, nothing chained
            let give = t.img.rel32_target(f + 0x20);
            t.write(OBJ, &[0; 0x1c]);
            t.set_u32(OBJ + 8, OBJ);
            for op in [0x48, 0x55, 0x64, 0x74] {
                t.set_u32(t.img.word(give + op), SCRATCH + 0x3000 + op);
            }
            t.set_u32(SCRATCH + 0x3074, SCRATCH + 0x3074);
            t.set_u32(SCRATCH + 0x3078, SCRATCH + 0x3074);
            t.set_u16(t.img.word(give + 0x8e), 7);
        },
    ),
    (
        "slot_keep_store_ret1",
        &[(661, 0x1500)],
        1,
        &[],
        |t, f, c| {
            globals(t, f, &[0x10, 0x19], c);
        },
    ),
];

/// A free ring of three pool nodes for the pool's `take` at `take`.
fn ring(t: &mut Twin, take: u32) {
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    for i in 1..=3 {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
    }
    t.set_u32(free, node(1));
    t.set_u16(count, 3);
}

/// `run_objects` at `ro` finds no object (it returns 1), or one paused
/// object (`busy`).
fn objects(t: &mut Twin, ro: u32, busy: bool) {
    let list = t.img.word(ro + 0x7c);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(node + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    t.set_u32(list + 8, if busy { node } else { 0 });
}

/// The task `OBJ` belongs to (`OBJ + 8`) and its fields.
fn task(t: &mut Twin, c: u32) {
    fill(t, OBJ + 0x10, 2, c);
    t.set_u32(OBJ + 8, SCRATCH + 0x3800);
    fill(t, SCRATCH + 0x3810, 1, c);
}

/// The node `OBJ` alone (no siblings, nothing chained) for the pool's
/// `give_back` at `give`.
fn lone_node(t: &mut Twin, give: u32) {
    t.write(OBJ, &[0; 0x1c]);
    t.set_u32(OBJ + 8, OBJ);
    for op in [0x48, 0x55, 0x64, 0x74] {
        t.set_u32(t.img.word(give + op), SCRATCH + 0x3000 + op);
    }
    t.set_u32(SCRATCH + 0x3074, SCRATCH + 0x3074);
    t.set_u32(SCRATCH + 0x3078, SCRATCH + 0x3074);
    t.set_u16(t.img.word(give + 0x8e), 7);
}

const CASES2: &[Case] = &[
    ("handle_release", &[(391, 0x31a0)], 1, &[], |t, f, c| {
        ptr(t, f, 0x08, &[0x14], c);
    }),
    (
        "handle_start",
        &[(391, 0x1680), (392, 0x1e10)],
        1,
        &[],
        |t, f, c| {
            ptr(t, f, 0x0e, &[0x2b], c);
            globals(t, f, &[0x20], c);
        },
    ),
    ("code_slot", &[(275, 0x4160)], 2, &[], |t, f, c| {
        t.set_u32(t.img.word(f + 0x01), P);
        t.set_u8(P, if c == 0 { 0xd } else { 0xc });
        t.set_u8(P + 1, 2);
        fill(t, t.img.word(f + 0x15), 4, c);
    }),
    (
        "fade_at",
        &[(511, 0x2af0)],
        2,
        &[0xa1, 0xa2, 0xa3],
        |t, f, c| {
            // junk below the entry: the stack word the fade reads is pushed
            ownb::dirty_stack(t, 0x2af0 + c, &[]);
            t.set_u32(t.img.word(f + 0x02), P);
            fill(t, P + t.img.word(f + 0x1a), 16, c);
            let fade = t.img.rel32_target(f + 0x24);
            ring(t, t.img.rel32_target(fade + 0x75));
            let node8 = SCRATCH + 0x1200;
            t.write(node8, &[0x77; 0x1c]);
            t.set_u32(t.img.word(fade + 0x7b), if c == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(fade + 0x93), node8);
        },
    ),
    ("slot_pair_k", &[(186, 0x5360)], 1, &[OBJ], |t, f, c| {
        t.set_u32(OBJ, P);
        for op in [0x12, 0x18] {
            fill(t, P + t.img.word(f + op), 1, c ^ op);
        }
    }),
    (
        "slot_field_flag",
        &[(416, 0x1d80), (521, 0x1400)],
        1,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x04, &[0x0f], c);
            t.set_u32(OBJ, 0x1230);
        },
    ),
    ("slot_then_cb", &[(193, 0x14a0)], 1, &[OBJ], none),
    (
        "byte_slot_chain",
        &[(304, 0x37b0), (304, 0x37e0)],
        1,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x01), P);
            t.set_u8(P, 7 + c as u8);
        },
    ),
    ("slot3_then_store", &[(669, 0x1000)], 1, &[], |t, f, c| {
        globals(t, f, &[0x1e], c);
    }),
    ("seq_wait", &[(525, 0x1190)], 1, &[OBJ], none),
    ("done_when_flag", &[(101, 0x31a0)], 3, &[OBJ], |t, f, c| {
        objects(t, t.img.rel32_target(f + 0x04), c == 2);
        let flag = t.img.word(f + 0x0f);
        t.set_u32(flag, if c == 1 { 0x7fff } else { 0x8000 });
        task(t, c);
    }),
    ("store_then_slot4", &[(661, 0x1670)], 1, &[], |t, f, c| {
        globals(t, f, &[0x1e], c);
    }),
    ("count_down", &[(661, 0x16b0)], 6, &[], |t, f, c| {
        let v = [0u32, 1, 2, 3, 0x100, 0x82][c as usize];
        t.set_u32(t.img.word(f + 0x01), v);
    }),
    (
        "script_motion_give_back",
        &[(672, 0x1480)],
        3,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x07, &[0x21], c);
            script(t, t.img.rel32_target(f + 0x0c), if c == 1 { 2 } else { 0 });
            // the motion is through (its done word 1), or waits
            let m = t.img.rel32_target(f + 0x13);
            t.set_u32(t.img.word(m + 0x02), u32::from(c != 2));
            t.set_u16(t.img.word(m + 0x11), 5);
            lone_node(t, t.img.rel32_target(f + 0x29));
        },
    ),
    (
        "sub_chain",
        &[(168, 0x33c0), (587, 0x2ee0)],
        1,
        &[],
        |t, f, _| {
            t.set_u32(t.img.word(f + 0x01), P);
            for (i, op) in [0x09u32, 0x11, 0x19].into_iter().enumerate() {
                t.set_u32(
                    P + t.img.word(f + op),
                    (1.5f32 * (i as f32 + 1.0)).to_bits(),
                );
            }
        },
    ),
    ("add_chain", &[(563, 0x24f0)], 1, &[], |t, f, _| {
        t.set_u32(t.img.word(f + 0x01), P);
        for (i, op) in [0x09u32, 0x11, 0x19].into_iter().enumerate() {
            t.set_u32(P + t.img.word(f + op), (0.75f32 - i as f32).to_bits());
        }
    }),
    ("add_chain_stored", &[(307, 0x29f0)], 1, &[], |t, f, _| {
        t.set_u32(t.img.word(f + 0x01), P);
        for (i, op) in [0x09u32, 0x11, 0x21].into_iter().enumerate() {
            t.set_u32(P + t.img.word(f + op), (2.5f32 + i as f32).to_bits());
        }
    }),
];

/// The motion step at `m` through (its done word 1) or waiting.
fn motion(t: &mut Twin, m: u32, done: bool) {
    t.set_u32(t.img.word(m + 0x02), u32::from(done));
    t.set_u16(t.img.word(m + 0x11), 5);
}

/// The grid draw at `d`: one record (`n` = 1) under the game lock.
fn grid(t: &mut Twin, d: u32, case: u32) {
    let w = |t: &Twin, off: u32| t.img.word(d + off);
    let (n, recs, lock) = (w(t, 0x08), w(t, 0x0f), w(t, 0x53));
    t.set_u32(n, 1);
    t.set_u32(recs, SCRATCH + 0x4400);
    fill(t, SCRATCH + 0x4400, 4, case);
    t.set_u32(lock, SCRATCH + 0x800);
    t.set_u32(SCRATCH + 0x800, 0x5555);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
}

/// The part registration at `r`: two parts, no banks, the lock.
fn register(t: &mut Twin, r: u32) {
    let w = |t: &Twin, off: u32| t.img.word(r + off);
    let (count, table, lock, banks) = (w(t, 0x0e), w(t, 0x16), w(t, 0x43), w(t, 0x76));
    let recs = SCRATCH + 0x4000;
    t.set_u32(table, recs);
    for i in 0..2 {
        t.set_u32(recs + 0x74 * i + 0x30, 0x0300_0000 + i);
    }
    t.set_u32(lock, SCRATCH + 0x800);
    t.set_u32(SCRATCH + 0x800, 0x5555);
    t.set_u32(count, 2);
    t.set_u32(banks, 0);
    // the 64-bit handle's high half
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
}

/// Each float field of `P` at the operands `ops` set to `v`, `v + 1`, ...
fn floats(t: &mut Twin, f: u32, ops: &[u32], v: f32) {
    t.set_u32(t.img.word(f + 0x01), P);
    for (i, &op) in (0u32..).zip(ops) {
        let x = v + i as f32;
        t.set_u32(P + t.img.word(f + op), x.to_bits());
    }
}

const CASES3: &[Case] = &[
    ("done_field", &[(473, 0x17c0)], 3, &[OBJ], |t, f, c| {
        ptr(t, f, 0x06, &[0x16], c);
        t.set_u32(P + t.img.word(f + 0x16), u32::from(c != 1));
        objects(t, t.img.rel32_target(f + 0x0b), c == 2);
        task(t, c);
    }),
    (
        "done_field_register",
        &[(446, 0x4ba0)],
        2,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x06, &[0x16], c);
            t.set_u32(P + t.img.word(f + 0x16), u32::from(c != 1));
            objects(t, t.img.rel32_target(f + 0x0b), false);
            task(t, c);
            register(t, t.img.rel32_target(f + 0x33));
        },
    ),
    (
        "end_when_idle",
        &[(391, 0x1380), (448, 0x4c80)],
        4,
        &[],
        |t, f, c| {
            ptr(t, f, 0x03, &[0x1c], c);
            t.set_u32(P + t.img.word(f + 0x1c), u32::from(c != 1));
            t.set_u32(t.img.word(f + 0x13), u32::from(c == 2));
            objects(t, t.img.rel32_target(f + 0x08), c == 3);
            globals(t, f, &[0x29], c);
        },
    ),
    ("end_register", &[(606, 0x69e0)], 2, &[], |t, f, c| {
        ptr(t, f, 0x03, &[0x13], c);
        t.set_u32(P + t.img.word(f + 0x13), u32::from(c != 1));
        objects(t, t.img.rel32_target(f + 0x08), false);
        register(t, t.img.rel32_target(f + 0x1a));
        globals(t, f, &[0x25], c);
    }),
    ("done_count_flag", &[(100, 0x1680)], 4, &[OBJ], |t, f, c| {
        objects(t, t.img.rel32_target(f + 0x04), c == 3);
        t.set_u32(t.img.word(f + 0x0f), if c == 1 { 5 } else { 0 });
        t.set_u32(t.img.word(f + 0x18), if c == 2 { 0x7fff } else { 0x8000 });
        task(t, c);
    }),
    (
        "done_motion_word",
        &[(246, 0x17d0), (276, 0x32b0)],
        4,
        &[OBJ],
        |t, f, c| {
            motion(t, t.img.rel32_target(f + 0x05), c != 1);
            objects(t, t.img.rel32_target(f + 0x0c), c == 3);
            t.set_u32(t.img.word(f + 0x1b), u32::from(c != 2));
            task(t, c);
        },
    ),
    (
        "done_motion_field",
        &[(275, 0x3a00)],
        3,
        &[OBJ],
        |t, f, c| {
            motion(t, t.img.rel32_target(f + 0x05), c != 1);
            objects(t, t.img.rel32_target(f + 0x0c), false);
            t.set_u32(t.img.word(f + 0x1a), P);
            t.set_u32(P + t.img.word(f + 0x20), u32::from(c != 2));
            task(t, c);
        },
    ),
    ("done_then_grid", &[(243, 0x1840)], 3, &[OBJ], |t, f, c| {
        objects(t, t.img.rel32_target(f + 0x05), c == 2);
        motion(t, t.img.rel32_target(f + 0x0c), c != 1);
        task(t, c);
        grid(t, t.img.rel32_target(f + 0x30), c);
    }),
    (
        "script_motion_release",
        &[(432, 0x1310)],
        3,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x07, &[0x24], c);
            script(t, t.img.rel32_target(f + 0x0c), if c == 1 { 2 } else { 0 });
            motion(t, t.img.rel32_target(f + 0x13), c != 2);
            lone_node(t, t.img.rel32_target(f + 0x30));
        },
    ),
    (
        "slot237_scale32",
        &[(165, 0x2ed0), (590, 0x1640)],
        1,
        &[OBJ],
        |t, f, c| {
            fill(t, OBJ + 0xd0, 8, c);
            globals(t, f, &[0x22], c);
        },
    ),
    ("task_then_cb", &[(526, 0x1100)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x0f], c);
    }),
    ("two_tasks_flag", &[(301, 0x1cc0)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x0f, 0x27], c);
        t.set_u32(OBJ, 0x1230);
    }),
    ("clear_bit_four", &[(307, 0x5230)], 1, &[], |t, f, c| {
        globals(t, f, &[0x0e, 0x18, 0x22, 0x2c], c);
        t.set_u32(t.img.word(f + 0x08), 0x5a5a_1237);
    }),
    (
        "six_stores",
        &[(303, 0x1660), (303, 0x16a0)],
        1,
        &[],
        |t, f, c| {
            globals(t, f, &[0x02, 0x0c, 0x16, 0x20, 0x2a, 0x34], c);
        },
    ),
    ("record_six", &[(253, 0x18a0)], 1, &[], |t, f, c| {
        t.set_u32(t.img.word(f + 0x01), 2);
        let rec = 2 * t.img.word(f + 0x07) + t.img.word(f + 0x0c);
        fill(t, rec + 0x4c, 8, c);
    }),
    ("grow_after", &[(307, 0x4da0)], 2, &[OBJ], |t, _, c| {
        fill(t, OBJ + 0x5c, 6, c);
        t.set_u32(OBJ + 0x60, 1.5f32.to_bits());
        t.set_u32(OBJ + 0x104, 0x0104);
        t.set_u32(OBJ + 0x118, if c == 0 { 0x51 } else { 0x50 });
    }),
    ("sub_two", &[(304, 0x2ef0)], 1, &[], |t, f, _| {
        floats(t, f, &[0x09, 0x21], 4.5);
        for op in [0x31, 0x39] {
            fill(t, P + t.img.word(f + op), 1, op);
        }
    }),
    ("add_two", &[(304, 0x3500)], 1, &[], |t, f, _| {
        floats(t, f, &[0x09, 0x11], -0.25);
        fill(t, P + t.img.word(f + 0x39), 1, 9);
    }),
    ("sub_copy", &[(304, 0x34c0)], 1, &[], |t, f, c| {
        t.set_u32(t.img.word(f + 0x02), P);
        t.set_u32(P + t.img.word(f + 0x0a), 3.0f32.to_bits());
        for op in [0x18, 0x20, 0x28, 0x2e, 0x34, 0x3a] {
            fill(t, P + t.img.word(f + op), 1, c ^ op);
        }
    }),
    (
        "slot_keep_two",
        &[(430, 0x28c0), (588, 0x2090)],
        1,
        &[],
        |t, f, c| {
            ptr(t, f, 0x0e, &[0x24, 0x2e], c);
            globals(t, f, &[0x37], c);
        },
    ),
    ("three_slots", &[(416, 0x1530)], 1, &[OBJ], none),
    ("count_then_task", &[(360, 0x18f0)], 2, &[OBJ], |t, f, c| {
        t.set_u32(t.img.word(f + 0x05), 1 - c);
        globals(t, f, &[0x1d], c);
    }),
    ("free_part_of", &[(537, 0x1260)], 1, &[OBJ], |t, f, _| {
        t.set_u32(t.img.word(f + 0x04), SCRATCH + 0x200);
    }),
    ("free_parts", &[(641, 0x1970)], 1, &[OBJ], |t, f, _| {
        t.set_u32(t.img.word(f + 0x11), SCRATCH + 0x200);
    }),
    ("float_product", &[(253, 0x2bf0)], 1, &[], |t, f, c| {
        globals(t, f, &[0x0e], c);
        t.stub_st0(364, |_, a| f64::from(a[0] & 0xff) * 0.5);
        t.stub_st0(16, |_, a| 1.25 + f64::from(a[0] & 0xf));
    }),
];

const CASES4: &[Case] = &[
    ("index_slot", &[(351, 0x2730)], 3, &[2], |t, f, c| {
        t.set_u32(t.img.word(f + 0x04), if c == 2 { 2 } else { 5 });
        let tbl = t.img.word(f + 0x13);
        fill(t, tbl, 4, c);
        if c == 1 {
            t.set_u32(tbl + 8, 0);
        }
        t.set_u32(t.img.word(f + 0x1b), P);
        fill(t, P, 2, c + 7);
    }),
    ("slot_three_words", &[(708, 0x1830)], 1, &[], |t, f, c| {
        globals(t, f, &[0x07, 0x1a, 0x2d], c);
    }),
    ("slot_by_count", &[(197, 0x1500)], 3, &[], |t, f, c| {
        t.set_u32(t.img.word(f + 0x02), u32::from(c == 1));
        t.set_u32(t.img.word(f + 0x0a), if c == 2 { 0x20 } else { 7 });
        globals(t, f, &[0x30], c);
    }),
    ("record_float", &[(273, 0x7690)], 1, &[], |t, f, c| {
        globals(t, f, &[0x02, 0x0e, 0x17, 0x1d, 0x27, 0x2d, 0x37], c);
        t.set_u32(t.img.word(f + 0x02), 0x5a5a_5a50);
    }),
    ("slots_g_g_none", &[(497, 0x1150)], 1, &[], |t, f, c| {
        globals(t, f, &[0x09, 0x1e], c);
    }),
    ("three_slots_m", &[(514, 0x3180)], 1, &[], |t, f, _| {
        // `this` is the call's ecx (0x00ec0000 + rva)
        let this = 0x00ec_0000 + 0x3180;
        let rec = SCRATCH + 0x5000;
        t.set_u32(this + t.img.word(f + 0x02) % 0x100, rec);
        t.set_u32(rec + t.img.word(f + 0x06) % 0x100, 0x00ab_c000);
    }),
    ("add_and_stores", &[(303, 0x16e0)], 1, &[], |t, f, c| {
        globals(t, f, &[0x12, 0x1c, 0x26, 0x30], c);
        t.set_u32(t.img.word(f + 0x04), 2.5f32.to_bits());
    }),
    ("slot_seq_call", &[(361, 0x17a0)], 1, &[OBJ], none),
    ("field_slots_seq", &[(191, 0x12f0)], 1, &[OBJ], |t, f, c| {
        ptr(t, f, 0x04, &[0x0a], c);
    }),
    ("slot237_obj", &[(348, 0x6df0)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x24], c);
        for op in [0x15, 0x1e, 0x36] {
            fill(t, OBJ + t.img.word(f + op), 4, c ^ op);
        }
    }),
    (
        "done_two_fields_register",
        &[(205, 0x15c0)],
        4,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x06, &[0x16, 0x1e], c);
            t.set_u32(P + t.img.word(f + 0x16), u32::from(c != 1));
            t.set_u32(P + t.img.word(f + 0x1e), if c == 2 { 3 } else { 0 });
            objects(t, t.img.rel32_target(f + 0x0b), c == 3);
            task(t, c);
            register(t, t.img.rel32_target(f + 0x3c));
        },
    ),
    ("push_task", &[(146, 0x7170)], 1, &[], |t, f, c| {
        let this = 0x00ec_0000 + 0x7170;
        fill(t, this, 8, c);
        let head = SCRATCH + 0x5000;
        fill(t, head, 4, c);
        t.set_u32(t.img.word(f + 0x17), head);
        t.set_u32(t.img.word(f + 0x2f), 0x0000_0f0f);
    }),
    (
        "task_while_count",
        &[(359, 0x1530)],
        2,
        &[OBJ],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x05), if c == 0 { 3 } else { 0 });
            globals(t, f, &[0x1e], c);
        },
    ),
    ("slot_task_cb", &[(528, 0x10d0)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x1e], c);
    }),
    ("done_all_grid", &[(270, 0x1410)], 4, &[OBJ], |t, f, c| {
        motion(t, t.img.rel32_target(f + 0x06), c != 1);
        script(t, t.img.rel32_target(f + 0x0d), if c == 2 { 2 } else { 0 });
        objects(t, t.img.rel32_target(f + 0x14), c == 3);
        task(t, c);
        grid(t, t.img.rel32_target(f + 0x3d), c);
    }),
    (
        "set_two_seq_call",
        &[(359, 0x1000), (360, 0x1000)],
        1,
        &[OBJ],
        |t, f, c| {
            globals(t, f, &[0x18, 0x22], c);
        },
    ),
    ("byte_slot_part", &[(430, 0x2450)], 1, &[OBJ], |t, f, c| {
        t.set_u32(t.img.word(f + 0x04), P);
        t.set_u8(P, 9);
        fill(t, OBJ + t.img.word(f + 0x3b), 1, c);
    }),
    ("vectors_then_float", &[(388, 0x6810)], 1, &[], |t, _, c| {
        let this = 0x00ec_0000 + 0x6810;
        fill(t, this + 0x10, 0x18, c);
    }),
];

const CASES5: &[Case] = &[
    (
        "task_keep_seq_wait",
        &[(382, 0x1170)],
        1,
        &[OBJ],
        |t, f, c| {
            globals(t, f, &[0x0f, 0x1c], c);
        },
    ),
    ("field_seq_call", &[(522, 0x1200)], 1, &[OBJ], |t, f, c| {
        ptr(t, f, 0x04, &[0x0c], c);
    }),
    ("two_tasks_seq", &[(195, 0x1300)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x0f, 0x27], c);
    }),
    ("slot9_seq_wait", &[(360, 0x1300)], 1, &[OBJ], none),
    ("task_field_cb", &[(195, 0x1270)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x0f], c);
        ptr(t, f, 0x1c, &[0x24], c);
    }),
    ("pair_seq_call", &[(378, 0x1120)], 1, &[OBJ], |t, f, c| {
        ptr(t, f, 0x04, &[0x0c], c);
        fill(t, P + (t.img.word(f + 0x12) & 0xff), 1, c ^ 0x12);
    }),
    (
        "seq_wait_short",
        &[(194, 0x10a0), (525, 0x1600)],
        1,
        &[OBJ],
        |t, f, c| {
            ptr(t, f, 0x04, &[0x0c], c);
        },
    ),
    ("slot3_seq_wait", &[(356, 0x17a0)], 1, &[OBJ], |t, f, c| {
        ptr(t, f, 0x04, &[0x0e], c);
    }),
    ("seven_stores", &[(380, 0x1b10)], 1, &[], |t, f, c| {
        globals(t, f, &[0x02, 0x0c, 0x16, 0x20, 0x2a, 0x34, 0x3e], c);
    }),
    ("task_seq_call", &[(521, 0x1170)], 1, &[OBJ], |t, f, c| {
        globals(t, f, &[0x0f], c);
    }),
    ("two_slots_seq", &[(529, 0x1210)], 1, &[OBJ], |t, f, c| {
        ptr(t, f, 0x04, &[0x0b], c);
    }),
    ("done_slot_h", &[(252, 0x1540)], 3, &[OBJ], |t, f, c| {
        motion(t, t.img.rel32_target(f + 0x05), c != 1);
        objects(t, t.img.rel32_target(f + 0x0c), c == 2);
        globals(t, f, &[0x22], c);
        task(t, c);
    }),
    ("done_count_bit", &[(492, 0x31b0)], 4, &[OBJ], |t, f, c| {
        motion(t, t.img.rel32_target(f + 0x05), c != 1);
        objects(t, t.img.rel32_target(f + 0x0c), false);
        t.set_u32(t.img.word(f + 0x1b), if c == 2 { 4 } else { 0 });
        t.set_u32(t.img.word(f + 0x23), P);
        t.set_u32(P + 0x30, if c == 3 { 0x7fff } else { 0x8001 });
        task(t, c);
    }),
    ("three_slots_obj", &[(514, 0x3520)], 1, &[OBJ], |t, f, _| {
        let a = SCRATCH + 0x5000;
        let rec = SCRATCH + 0x5800;
        t.set_u32(OBJ, a);
        t.set_u32(a + (t.img.word(f + 0x0d) & 0xff), rec);
        t.set_u32(rec + (t.img.word(f + 0x12) & 0xff), 0x00ab_c000);
    }),
];

/// Every slot answers a value of its first word; slot 277 also gives a
/// float in ST0; slot 260 the word at `SCRATCH + 0x700`.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(260, |m, _| m.r32(SCRATCH + 0x700));
    t.stub_st0(277, |_, a| {
        f64::from(a.first().copied().unwrap_or(0) & 0xfff) * 0.37
    });
}

/// Every case of `cases`, each copy against its own DLL.
fn run(cases: &[Case]) {
    for &(name, list, cases, args, set) in cases {
        let label = format!("o4 tiny {name}");
        for (n, rva) in copies(&label, list) {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            stubs(&mut t);
            let f = t.at(rva);
            for c in 0..cases {
                set(&mut t, f, c);
                t.check_entry(
                    f,
                    Call {
                        eax: 0x00ea_0000 + c,
                        ecx: 0x00ec_0000 + rva,
                        edx: 0x00ed_0000 + rva,
                        ..Call::default()
                    }
                    .with_xmm(&[
                        Xmm::f32(0.5),
                        Xmm::f32(1.25),
                        Xmm::f32(-2.75),
                        Xmm::f32(3.5),
                    ]),
                    args,
                    Eax::Checked,
                );
            }
        }
    }
}

/// `o4_tiny`'s functions.
#[test]
fn tiny_1() {
    run(CASES);
}

/// `o4_tiny2`'s functions.
#[test]
fn tiny_2() {
    run(CASES2);
}

/// `o4_tiny3`'s functions.
#[test]
fn tiny_3() {
    run(CASES3);
}

/// `o4_tiny4`'s functions.
#[test]
fn tiny_4() {
    run(CASES4);
}

/// `o4_tiny5`'s functions.
#[test]
fn tiny_5() {
    run(CASES5);
}

#[test]
fn dll_main_writes_only_crt_state() {
    let lists: Vec<&[(u16, u32)]> = CASES
        .iter()
        .chain(CASES2)
        .chain(CASES3)
        .map(|c| c.1)
        .collect();
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}
