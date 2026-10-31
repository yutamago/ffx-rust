//! The single functions of `ffx_magic::o4_single3` against their DLLs,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Xmm, o4_single3};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its label, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

/// Every slot answers a value of its first word; slot 262 fills the
/// three floats it is handed.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(262, |m, a| {
        for (i, v) in [1.5f32, 2.5, 3.5].into_iter().enumerate() {
            m.w32(a[1 + i], v.to_bits());
        }
        0x262
    });
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
    }
}

const CASES: &[Case] = &[
    (
        "o4 single3 object_task_head",
        &[(577, 0xb130)],
        1,
        |t, f, c| {
            let obj = 0x00ec_0000 + 0xb130;
            fill(t, obj, 0x20, c);
            let head = SCRATCH + 0x5000;
            fill(t, head, 4, c);
            t.set_u32(t.img.word(f + 0x1f), head);
            globals(t, f, &[0x37], c);
        },
    ),
    (
        "o4 single3 model_node_tail",
        &[(577, 0x8d70)],
        2,
        |t, f, c| {
            let n = 0x00ec_0000 + 0x8d70;
            fill(t, n, 0x30, c);
            let anchor = t.img.word(f + 0x31);
            let tail = SCRATCH + 0x5000;
            fill(t, tail, 4, c);
            t.set_u32(t.img.word(f + 0x24), if c == 0 { anchor } else { tail });
            globals(t, f, &[0x39], c);
            t.set_u16(t.img.word(f + 0x45), 5);
        },
    ),
    (
        "o4 single3 slot_seq_call_keep",
        &[(194, 0x11c0)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P + 0x198, 4, c);
            t.set_u32(OBJ + 0xac, 2);
            fill(t, OBJ + 0x2c, 0x10, c);
        },
    ),
    (
        "o4 single3 place_from_slot",
        &[(303, 0x14d0)],
        1,
        |t, f, c| {
            globals(t, f, &[0x23], c);
            // the placement in a battle, not grounded: it reads the vector
            let place = t.img.rel32_target(f + 0x4f);
            t.set_u32(t.img.word(place + 0x02), 1);
            t.stub(11, |_, _| 0);
        },
    ),
    (
        "o4 single3 task_node_tail",
        &[(379, 0x6910)],
        2,
        |t, f, c| {
            let n = 0x00ec_0000 + 0x6910;
            fill(t, n, 0x10, c);
            let anchor = t.img.word(f + 0x28);
            let tail = SCRATCH + 0x5000;
            fill(t, tail, 4, c);
            t.set_u32(t.img.word(f + 0x1b), if c == 0 { anchor } else { tail });
            globals(t, f, &[0x30], c);
            t.set_u16(t.img.word(f + 0x3c), 5);
        },
    ),
    (
        "o4 single3 model_flag_step",
        &[(263, 0x6be0)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x08), P);
            fill(t, P + 0x4c, 0x10, c);
            t.set_u32(P + 0x90, 0x5a5a_5a50 | c);
        },
    ),
    ("o4 single3 drop_current", &[(263, 0x6b30)], 4, |t, f, c| {
        let l = P;
        t.set_u32(t.img.word(f + 0x02), l);
        let node = SCRATCH + 0x5000;
        fill(t, node, 8, c);
        let (a, b) = (SCRATCH + 0x5100, SCRATCH + 0x5200);
        fill(t, a, 4, 1);
        fill(t, b, 4, 2);
        // no node (0); the node at both ends (1), at neither (2), first (3)
        t.set_u32(l + 0x14, if c == 0 { 0 } else { node });
        let (prev, next) = [(a, b), (node, node), (a, b), (node, b)][c as usize];
        t.set_u32(l + 4, prev);
        t.set_u32(l + 8, next);
    }),
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
fn fade_setup() {
    let Some(mut t) = magic_l3::twin(577) else {
        return;
    };
    stubs(&mut t);
    let f = t.at(0x9170);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(f, sig))
        .map(|(n, _)| *n);
    assert!(found.is_some_and(|n| n.starts_with("o4 single3 fade_setup")));
    fill(&mut t, OBJ, 0x50, 3);
    t.set_u32(OBJ + 0x64, 4.25f32.to_bits());
    let regs = Call::this(OBJ).with_xmm(&[Xmm::f32(0.0), Xmm::f32(1.5), Xmm::f32(-2.0)]);
    t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
        o4_single3::fade_setup(g, img, fr)
    });
}

#[test]
fn head_node() {
    let Some(mut t) = magic_l3::twin(577) else {
        return;
    };
    stubs(&mut t);
    let f = t.at(0xac20);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(f, sig))
        .map(|(n, _)| *n);
    assert!(found.is_some_and(|n| n.starts_with("o4 single3 head_node")));
    let node = SCRATCH + 0x7000;
    fill(&mut t, node, 0x10, 1);
    fill(&mut t, OBJ, 0x20, 2);
    let head = SCRATCH + 0x5000;
    fill(&mut t, head, 4, 3);
    t.set_u32(t.img.word(f + 0x1d), head);
    globals(&mut t, f, &[0x37], 4);
    let regs = Call {
        ecx: node,
        edx: OBJ,
        ..Call::default()
    };
    t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(o4_single3::head_node(g, img, fr.at, fr.ecx, fr.edx))
    });
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(&[(577, 0x9170), (577, 0xac20)]);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 182
// mutants, 172 killed, 2 that do not compile. The survivors are
// equivalent: the nodes' `+0x4`/`+0x8` set to themselves just before
// they are overwritten (four), `model_node_tail`'s first vtable store,
// which the second replaces (two), and `place_from_slot`'s `v.w`, which
// the placement never reads (two).
