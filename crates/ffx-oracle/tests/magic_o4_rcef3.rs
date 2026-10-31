//! The rcef chains of `ffx_magic::o4_rcef3` and the static-record
//! functions of `ffx_magic::o4_misc` against their DLLs, and `DllMain` of
//! each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Xmm, o4_misc};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its label, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

/// Every slot answers a value of its first word; the create-task slot
/// (434) a task record at `SCRATCH + 0x4000`; the float slot 790 a value.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(434, |_, _| SCRATCH + 0x4000);
    t.stub_st0(790, |_, _| 0.123_456_789);
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
    }
}

const CASES: &[Case] = &[
    (
        "o4 rcef3 field_slots_seq",
        &[(361, 0x2770)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P + t.img.word(f + 0x0b), 1, c);
        },
    ),
    ("o4 rcef3 tasks_or_seq", &[(382, 0x1790)], 2, |t, f, c| {
        t.set_u32(t.img.word(f + 0x05), c);
        globals(t, f, &[0x2c], c);
    }),
    ("o4 rcef3 step_or_flag", &[(382, 0x1c70)], 2, |t, f, c| {
        t.set_u32(OBJ + 0xac, 1);
        fill(t, OBJ, 0x40, c);
        t.set_u32(OBJ + 0xac, 1);
        t.set_u32(OBJ + 0x2c + 0x10, 4);
        t.set_u32(t.img.word(f + 0x15), if c == 0 { 9 } else { 5 });
        t.set_u32(t.img.word(f + 0x21), P);
        fill(t, P + 0xd0, 4, c);
        fill(t, OBJ + 0xd0, 4, c ^ 3);
    }),
    (
        "o4 rcef3 task_keep_seq_call",
        &[(313, 0x19e0)],
        1,
        |t, f, c| {
            globals(t, f, &[0x0f, 0x23], c);
            t.set_u32(SCRATCH + 0x40ac, 1);
            fill(t, SCRATCH + 0x402c, 0x10, c);
        },
    ),
    (
        "o4 misc static_flag_slots",
        &[(343, 0x6900)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x02), 0x5a50 | c);
            globals(t, f, &[0x12, 0x33, 0x52], c);
        },
    ),
    (
        "o4 misc reset_then_float",
        &[(351, 0x2450)],
        1,
        |t, f, c| {
            globals(t, f, &[0x1f, 0x29, 0x33, 0x3d, 0x49], c);
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
fn vector_set() {
    let Some(mut t) = magic_l3::twin(343) else {
        return;
    };
    stubs(&mut t);
    let f = t.at(0x69e0);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(f, sig))
        .map(|(n, _)| *n);
    assert!(found.is_some_and(|n| n.starts_with("o4 misc vector_set")));
    for c in 0..2u32 {
        t.set_u32(t.img.word(f + 0x06), 0x5a50 | c);
        globals(&mut t, f, &[0x21, 0x60], c);
        fill(&mut t, OBJ, 8, c);
        let regs = Call {
            eax: 0x00ea_0000,
            edx: OBJ,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(2.75)]);
        t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
            o4_misc::vector_set(g, img, fr)
        });
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(&[(343, 0x69e0)]);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 109
// mutants (o4_rcef3 64, o4_misc 45), 108 killed, 1 that does not compile.
