//! The small entries of `ffx_magic::o4_small` against every DLL's own
//! copy, and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};
use ownb::{rec, rec_seq, reset_counters};

const STORE_IMM: &[(u16, u32)] = &[(168, 0x3400), (673, 0x2030), (673, 0x2060), (673, 0x2070)];
const SLOT_IMM: &[(u16, u32)] = &[(303, 0x14b0), (473, 0x20c0), (511, 0x2b20)];
const SLOT_IMM_RET1: &[(u16, u32)] = &[(443, 0x1c00), (540, 0x1030), (540, 0x1060)];
const COPY_TWICE: &[(u16, u32)] = &[(162, 0x39c0), (304, 0x3790), (586, 0x35d0)];
const SLOT237_SCALE: &[(u16, u32)] = &[(276, 0x32f0), (305, 0x1cb0), (417, 0x48e0)];
const SLOT719_306: &[(u16, u32)] = &[(417, 0x4920), (419, 0x1d10), (432, 0x20d0), (572, 0x1630)];
const SLOT_SEQ_WAIT: &[(u16, u32)] = &[(378, 0x1040), (521, 0x1130), (525, 0x1150)];
const SEQ_KEEP_447: &[(u16, u32)] = &[(193, 0x1c70), (300, 0x1280), (525, 0x15a0), (529, 0x15c0)];
const RECORD_NODE: &[(u16, u32)] = &[(596, 0x3900), (348, 0x29a0), (353, 0x4a00), (612, 0x3e60)];
const STATIC_RECORD_NODE: &[(u16, u32)] =
    &[(163, 0x2410), (253, 0x48d0), (298, 0x2850), (508, 0x2b80)];
const PART_AND_VECTORS: &[(u16, u32)] =
    &[(477, 0x11c0), (477, 0x13a0), (486, 0x11c0), (486, 0x1360)];

const FADE_START: &[(u16, u32)] = &[(596, 0x70f0), (602, 0x8460), (606, 0x77b0)];

const ALL: &[&[(u16, u32)]] = &[
    FADE_START,
    STORE_IMM,
    SLOT_IMM,
    SLOT_IMM_RET1,
    COPY_TWICE,
    SLOT237_SCALE,
    SLOT719_306,
    SLOT_SEQ_WAIT,
    SEQ_KEEP_447,
    RECORD_NODE,
    STATIC_RECORD_NODE,
    PART_AND_VECTORS,
];

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// Every slot these functions call, each logging `n` words and answering
/// a value of its first word.
fn stubs(t: &mut Twin) {
    for (s, n) in [
        (237u32, 4usize),
        (306, 6),
        (719, 3),
        (433, 1),
        (444, 1),
        (447, 2),
        (449, 6),
        (450, 3),
        (451, 5),
        (941, 1),
        (714, 5),
        (268, 7),
    ] {
        rec(t, s, n, move |_, a| a[0] ^ (s << 20));
    }
}

/// Each copy of `list` checked with `cases` set-ups (`set(t, f, case)`
/// before each call) and the call's stack words `args`.
fn each(
    label: &str,
    list: &[(u16, u32)],
    cases: u32,
    args: &[u32],
    mut set: impl FnMut(&mut Twin, u32, u32),
) {
    for (n, rva) in copies(label, list) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        for case in 0..cases {
            set(&mut t, f, case);
            t.check_entry(
                f,
                Call {
                    eax: 0x00ea_0000 + case,
                    ecx: OBJ + 0x800,
                    edx: 0x0ed0,
                    ..Call::default()
                },
                args,
                Eax::Checked,
            );
        }
    }
}

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

#[test]
fn stores_and_copies() {
    each("o4 store imm", STORE_IMM, 1, &[], |t, f, _| {
        t.set_u32(t.img.word(f + 0x01), P);
        fill(t, P + t.img.word(f + 0x07) - 4, 3, f);
    });
    each("o4 copy twice", COPY_TWICE, 2, &[], |t, f, case| {
        t.set_u32(t.img.word(f + 0x01), P);
        for op in [0x09, 0x11, 0x19] {
            fill(t, P + t.img.word(f + op) - 4, 3, f ^ op ^ case);
        }
    });
}

#[test]
fn slot_calls() {
    each("o4 slot imm", SLOT_IMM, 1, &[], |_, _, _| {});
    each("o4 slot imm, 1", SLOT_IMM_RET1, 1, &[], |_, _, _| {});
    each("o4 slot 237, scale", SLOT237_SCALE, 1, &[OBJ], |t, f, _| {
        t.set_u32(t.img.word(f + 0x1f), 0x0237_0000 ^ f);
        fill(t, OBJ + 0xd0, 8, f);
    });
    each("o4 slots 719, 306", SLOT719_306, 1, &[OBJ], |t, f, _| {
        fill(t, OBJ + 0x100, 4, f);
    });
    each("o4 slot, seq, wait", SLOT_SEQ_WAIT, 1, &[OBJ], |_, _, _| {});
    each(
        "o4 seq keep (447)",
        SEQ_KEEP_447,
        2,
        &[OBJ],
        |t, f, case| {
            t.set_u32(t.img.word(f + 0x04), P);
            t.set_u32(P + t.img.word(f + 0x0c), 0x4470_0000 ^ f);
            t.set_u32(OBJ + 0xac, case + 1);
            fill(t, OBJ + 0x2c, 0x10, f ^ case);
        },
    );
}

#[test]
fn pool_records() {
    each("o4 record node", RECORD_NODE, 2, &[], |t, f, case| {
        ring(t, t.img.rel32_target(f + 0x31));
        let node8 = SCRATCH + 0x1200;
        t.write(node8, &[0x77; 0x1c]);
        t.set_u32(t.img.word(f + 0x37), if case == 0 { 0 } else { node8 });
        t.set_u32(t.img.word(f + 0x4f), node8);
        fill(t, OBJ + 0x800, 12, f ^ case);
    });
    each(
        "o4 static record node",
        STATIC_RECORD_NODE,
        2,
        &[],
        |t, f, case| {
            ring(t, t.img.rel32_target(f + 0x40));
            let node8 = SCRATCH + 0x1200;
            t.write(node8, &[0x77; 0x1c]);
            t.set_u32(t.img.word(f + 0x46), if case == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(f + 0x61), node8);
            for op in [0x09, 0x13, 0x1d, 0x27, 0x31, 0x3b] {
                t.set_u32(t.img.word(f + op), 0x5a5a_0000 ^ op);
            }
        },
    );
}

#[test]
fn part_and_vectors() {
    each(
        "o4 part and vectors",
        PART_AND_VECTORS,
        1,
        &[],
        |t, f, _| {
            t.set_u32(t.img.word(f + 0x01), SCRATCH + 0x200);
            t.set_u32(t.img.word(f + 0x0c), SCRATCH + 0x300);
            t.set_u32(SCRATCH + 0x310, 0x00ab_cd00 ^ f);
            let list = SCRATCH + 0x400;
            fill(t, list, 16, f);
            rec(t, 941, 1, move |_, _| list);
            let wrapper = t.img.rel32_target(f + 0x32);
            t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
            t.set_u32(SCRATCH + 0x204, 9);
            t.set_u32(t.img.word(f + 0x69), 0x0268_0000 ^ f);
            for op in [0x45, 0x55, 0x70] {
                fill(t, t.img.word(f + op), 4, f ^ op);
            }
        },
    );
}

#[test]
fn fade_start() {
    each("o4 fade start", FADE_START, 2, &[], |t, f, case| {
        reset_counters(t);
        rec_seq(t, 371, &[2, 4], |_, a| a[0] ^ 0x371);
        t.set_u32(t.img.word(f + 0x0e), P);
        let rec_at = P + t.img.word(f + 0x43);
        fill(t, rec_at, 16, f ^ case);
        let fade = t.img.rel32_target(f + 0x4d);
        ring(t, t.img.rel32_target(fade + 0x75));
        let head = t.img.word(fade + 0x7b);
        let node8 = SCRATCH + 0x1200;
        t.write(node8, &[0x77; 0x1c]);
        t.set_u32(head, if case == 0 { 0 } else { node8 });
        t.set_u32(t.img.word(fade + 0x93), node8);
        // the floats the fade keeps: one not above 0 the second time
        let (a, b) = (t.img.word(f + 0x3b), t.img.word(f + 0x33));
        t.set_u32(a, if case == 0 { 0.5f32 } else { -1.0 }.to_bits());
        t.set_u32(b, 2.0f32.to_bits());
    });
}

#[test]
fn dll_main_writes_only_crt_state() {
    for n in dlls(ALL) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py): 57 mutants, all
// killed.
