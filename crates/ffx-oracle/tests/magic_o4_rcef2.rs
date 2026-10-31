//! The rcef chains of `ffx_magic::o4_rcef2` and the model set-up callers
//! of `ffx_magic::o4_setup` against their DLLs, and `DllMain` of each of
//! those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its label, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

/// Every slot answers a value of its first word; the create-task slot
/// (434) a task record at `SCRATCH + 0x4000`.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(434, |_, _| SCRATCH + 0x4000);
}

/// `*G` = `P` for the pointer global at `op`, the words around `P + D`
/// for each field operand filled.
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

/// The object's list (`+0x2c`, its count at `+0xac`).
fn list(t: &mut Twin, c: u32) {
    t.set_u32(OBJ + 0xac, c + 1);
    fill(t, OBJ + 0x2c, 0x10, c);
}

const CASES: &[Case] = &[
    (
        "o4 rcef2 clear_slot_seq_call",
        &[(191, 0x1000)],
        1,
        |t, f, c| {
            ptr(t, f, 0x04, &[0x14], c);
            globals(t, f, &[0x0a], c);
        },
    ),
    (
        "o4 rcef2 indexed_slot_seq_wait",
        &[(191, 0x10b0)],
        1,
        |t, f, c| {
            ptr(t, f, 0x05, &[0x20], c);
            t.set_u32(t.img.word(f + 0x0a), 2);
            fill(t, P + t.img.word(f + 0x15), 4, c);
        },
    ),
    (
        "o4 rcef2 two_tasks_seq_wait8",
        &[(300, 0x1170)],
        1,
        |t, f, c| {
            globals(t, f, &[0x0f], c);
        },
    ),
    ("o4 rcef2 slots_seq_call", &[(300, 0x11d0)], 1, |t, f, c| {
        ptr(t, f, 0x04, &[0x0b], c);
    }),
    (
        "o4 rcef2 pair_slot_seq_wait",
        &[(301, 0x1740)],
        1,
        |t, f, c| {
            ptr(t, f, 0x04, &[0x0e, 0x14], c);
        },
    ),
    (
        "o4 rcef2 field_task_seq_wait",
        &[(301, 0x17a0)],
        1,
        |t, f, c| {
            ptr(t, f, 0x04, &[0x0f], c);
        },
    ),
    ("o4 rcef2 task_seq_keep", &[(359, 0x1160)], 2, |t, f, c| {
        ptr(t, f, 0x04, &[0x0c], c);
        list(t, c);
    }),
    (
        "o4 rcef2 tasks_while_count",
        &[(359, 0x14d0)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x05), if c == 0 { 2 } else { 0 });
            globals(t, f, &[0x1e], c);
        },
    ),
    (
        "o4 rcef2 keep_task_seq_wait",
        &[(361, 0x1260)],
        1,
        |t, f, c| {
            globals(t, f, &[0x0f, 0x1c], c);
        },
    ),
    ("o4 rcef2 slot_seq_keep", &[(361, 0x12d0)], 2, |t, f, c| {
        ptr(t, f, 0x04, &[0x0c], c);
        list(t, c);
    }),
    (
        "o4 rcef2 pair_field_seq_wait",
        &[(526, 0x1090)],
        1,
        |t, f, c| {
            ptr(t, f, 0x04, &[0x0b, 0x15], c);
        },
    ),
    (
        "o4 rcef2 indexed_field_seq_wait",
        &[(526, 0x11c0)],
        1,
        |t, f, c| {
            ptr(t, f, 0x05, &[0x11], c);
            t.set_u32(t.img.word(f + 0x0a), 2);
            fill(t, P + t.img.word(f + 0x1c), 4, c);
        },
    ),
    ("o4 rcef2 task_count_cb", &[(526, 0x1270)], 2, |t, f, c| {
        globals(t, f, &[0x0f], c);
        t.set_u32(t.img.word(f + 0x31), P);
        t.set_u32(t.img.word(f + 0x23), 3);
        t.set_u32(P + 0x19c, if c == 0 { 9 } else { 4 });
        t.set_u32(SCRATCH + 0x40ac, 1);
        fill(t, SCRATCH + 0x402c, 0x10, c);
    }),
    (
        "o4 rcef2 effect_slots_seq",
        &[(529, 0x1650)],
        1,
        |t, f, c| {
            globals(t, f, &[0x16, 0x1b], c);
        },
    ),
    ("o4 setup two_setups", &[(42, 0xb060)], 1, |t, f, c| {
        globals(t, f, &[0x0d], c);
        fill(t, t.img.word(f + 0x1f), 0x30, c);
        fill(t, t.img.word(f + 0x2e), 0x30, c ^ 1);
    }),
    (
        "o4 setup setups_of_block",
        &[(262, 0x6350)],
        2,
        |t, _, c| {
            let b = SCRATCH + 0x8000;
            t.set_u32(OBJ, b);
            t.set_u32(b + 0x4dc, 2 * c);
            fill(t, b + 0x1c, 0x60, c);
        },
    ),
    (
        "o4 setup setups_of_table",
        &[(262, 0x6e50)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x09), 2 * c);
            fill(t, t.img.word(f + 0x11), 0x60, c);
        },
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(label, list, cases, set) in CASES {
        for (n, rva) in copies(label, list) {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            stubs(&mut t);
            let f = t.at(rva);
            for c in 0..cases {
                set(&mut t, f, c);
                let regs = Call {
                    eax: 0x00ea_0000 + c,
                    ecx: 0x00ec_0000 + rva,
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                };
                t.check_entry(f, regs, &[OBJ], Eax::Checked);
            }
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line): 204 mutants (o4_rcef2 177, o4_setup 27), 199 killed, 1
// that does not compile. The 4 survivors: an extra word for slot 468 (no
// arguments; equivalent), and the model set-up's frame placed higher in
// the three callers (its locals lie below the entry and the set-up paths
// these records reach hand none of them to a call).
