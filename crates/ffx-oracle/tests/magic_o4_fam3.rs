//! The families of `ffx_magic::o4_fam3` against every DLL's own copy,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_fam3};
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

const RECORD_COPY: &[(u16, u32)] = &[(480, 0x2880), (480, 0x2910)];

/// Every slot answers a value of its first word; slot 335 fills the list
/// it is handed (the count at `SCRATCH + 0x700`), slot 229 a vector.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(335, |m, a| {
        let n = m.r32(SCRATCH + 0x700);
        for i in 0..n {
            m.w32(a[0] + 4 * i, 0x3350_0000 + i);
        }
        n
    });
    t.stub(229, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    t.stub(327, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, a[1] ^ i);
        }
        0x327
    });
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
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

const CASES: &[Case] = &[
    (
        "part_vectors",
        &[(370, 0x1550), (370, 0x18c0)],
        1,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x01), SCRATCH + 0x200);
            t.set_u32(t.img.word(f + 0x0c), SCRATCH + 0x300);
            t.set_u32(SCRATCH + 0x310, 0x00ab_cd00 ^ f);
            let list = SCRATCH + 0x400;
            fill(t, list, 16, f ^ c);
            t.stub(941, move |_, _| list);
            let wrapper = t.img.rel32_target(f + 0x32);
            t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
            t.set_u32(SCRATCH + 0x204, 9);
            for op in [0x40, 0x50, 0x61] {
                fill(t, t.img.word(f + op), 4, f ^ op);
            }
        },
    ),
    (
        "static_node",
        &[(267, 0x37f0), (311, 0x24c0)],
        2,
        &[],
        |t, f, c| {
            ring(t, t.img.rel32_target(f + 0x24));
            globals(t, f, &[0x07, 0x11, 0x1b], c);
            let node8 = SCRATCH + 0x1200;
            t.write(node8, &[0x77; 0x1c]);
            t.set_u32(t.img.word(f + 0x2a), if c == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(f + 0x41), node8);
        },
    ),
    (
        "sub_stores8",
        &[(307, 0x2a30), (563, 0x2530)],
        1,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x01), P);
            t.set_u32(P + t.img.word(f + 0x09), 2.75f32.to_bits());
            for k in 0..8 {
                fill(t, P + t.img.word(f + 0x17 + 10 * k), 1, c ^ k);
            }
            fill(t, P + t.img.word(f + 0x69), 1, 9);
        },
    ),
    (
        "slot_list",
        &[(519, 0x2a50), (519, 0x2ac0)],
        3,
        &[],
        |t, _, c| {
            t.set_u32(SCRATCH + 0x700, [0u32, 1, 5][c as usize]);
        },
    ),
    (
        "two_tasks_seq_wait",
        &[(194, 0x10f0), (300, 0x10a0)],
        1,
        &[OBJ],
        |t, f, c| {
            globals(t, f, &[0x2d, 0x45], c);
        },
    ),
    (
        "vector_globals",
        &[(165, 0x4750), (563, 0x5670)],
        1,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x13), P);
            fill(t, P + t.img.word(f + 0x2f), 4, c);
            globals(t, f, &[0x58, 0x65, 0x71], c);
        },
    ),
    (
        "release_and_reset",
        &[(162, 0x3900), (586, 0x3510)],
        2,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x03), P);
            let field = P + t.img.word(f + 0x09);
            let node = SCRATCH + 0x3000;
            t.write(node, &[0; 0x1c]);
            t.set_u32(node + 8, node);
            let give = t.img.rel32_target(f + 0x12);
            for op in [0x48, 0x55, 0x64, 0x74] {
                t.set_u32(t.img.word(give + op), SCRATCH + 0x3400 + op);
            }
            t.set_u16(t.img.word(give + 0x8e), 7);
            t.set_u32(field, if c == 0 { 0 } else { node });
            globals(t, f, &[0x27], c);
            t.set_u32(t.img.word(f + 0x4b), SCRATCH + 0x3800);
            t.set_u8(SCRATCH + 0x3801, 3);
            fill(t, t.img.word(f + 0x5a), 6, c);
        },
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(name, list, cases, args, set) in CASES {
        let label = format!("o4 fam3 {name}");
        for (n, rva) in copies(&label, list) {
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
                t.check_entry(f, regs, args, Eax::Checked);
            }
        }
    }
}

#[test]
fn record_copy_in_every_copy() {
    for &(n, rva) in RECORD_COPY {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let a = t.at(rva);
        let found = ffx_magic::INTERNAL
            .iter()
            .find(|(_, sig)| t.img.matches(a, sig))
            .map(|(n, _)| *n);
        assert!(found.is_some_and(|n| n.starts_with("o4 fam3 record_copy")));
        let src = SCRATCH + 0x100;
        fill(&mut t, src, 4, rva);
        for i in [0u32, 3] {
            let base = t.img.word(f + 0x13) + 0x38 * i;
            fill(&mut t, base, 14, i ^ rva);
            let regs = Call {
                edx: i,
                ..Call::default()
            };
            t.check(f, regs, &[src], Eax::Checked, |g, img, fr| {
                let src = fr.arg(g, 0);
                Ret::eax(o4_fam3::record_copy(g, img, fr.at, fr.edx, src))
            });
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(RECORD_COPY);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line): 122 mutants, 115 killed, 4 that do not compile. The 3
// survivors are equivalent: `static_node` clearing `+0x4` that the pool's
// `take` has just cleared (the original repeats it), and an extra leading
// 0 for `two_tasks_seq_wait`'s first slot (`push 0`).
