//! The rcef chains and slot steps of `ffx_magic::o4_rcef4` against their
//! DLLs, and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

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

/// Every slot answers a value of its first word.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
    }
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

/// A list `L` holding `obj` alone (`held`) or empty.
fn list_of(t: &mut Twin, list: u32, obj: u32, held: bool) {
    let node = SCRATCH + 0x3000;
    t.write(node, &[0; 0x10]);
    t.write(list, &[0; 0xc]);
    if held {
        t.set_u32(node + 8, obj);
        t.set_u32(list, node);
        t.set_u32(list + 4, node);
        t.set_u32(list + 8, node);
    }
}

const CASES: &[Case] = &[
    (
        "o4 rcef4 effect_task_seq_wait",
        &[(360, 0x10a0)],
        1,
        |t, f, c| {
            globals(t, f, &[0x35], c);
        },
    ),
    (
        "o4 rcef4 effect_keep_seq_call",
        &[(525, 0x1000)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P, 2, c + 3);
            fill(t, P + 0x1a0, 1, c);
            globals(t, f, &[0x2b], c);
        },
    ),
    ("o4 rcef4 seq_by_count", &[(534, 0x1040)], 2, |t, f, c| {
        t.set_u32(t.img.word(f + 0x14), P);
        t.set_u32(P + 0x19c, 1 + c);
    }),
    ("o4 rcef4 step_capped", &[(536, 0x11a0)], 3, |t, f, c| {
        let x = [0.5f32, 2.0, f32::NAN][c as usize];
        t.set_u32(t.img.word(f + 0x04), x.to_bits());
        globals(t, f, &[0x45], c);
    }),
    ("o4 rcef4 free_three", &[(185, 0x1500)], 1, |t, f, _| {
        t.set_u32(t.img.word(f + 0x01), SCRATCH + 0x200);
    }),
    (
        "o4 rcef4 countdown_then_done",
        &[(163, 0x1cb0)],
        4,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), [0u32, 1, 3, 1][c as usize]);
            objects(t, t.img.rel32_target(f + 0x40), c == 3);
            fill(t, OBJ + 0x10, 2, c);
            t.set_u32(OBJ + 8, SCRATCH + 0x3800);
            fill(t, SCRATCH + 0x3810, 1, c);
        },
    ),
    (
        "o4 rcef4 remove_and_release",
        &[(353, 0x2e00)],
        3,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x03), P);
            t.set_u16(P + 0x124, 0x8123);
            list_of(t, t.img.word(f + 0x0a), P, c != 0);
            // the pool node: none (case 1), else a lone one
            let node = SCRATCH + 0x3400;
            t.write(node, &[0; 0x1c]);
            t.set_u32(node + 8, node);
            let give = t.img.rel32_target(f + 0x4a);
            for op in [0x48, 0x55, 0x64, 0x74] {
                t.set_u32(t.img.word(give + op), SCRATCH + 0x3800 + op);
            }
            t.set_u16(t.img.word(give + 0x8e), 7);
            t.set_u32(P + 0x13e0, if c == 1 { 0 } else { node });
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
// code line): 86 mutants, 83 killed, 2 that do not compile. The survivor
// is equivalent: an extra leading 0 for slot 433 (`push 0`).
