//! The fade counter, halfword table and slot chain of
//! `ffx_magic::o4_more2` against every copy in its own DLL, and `DllMain`
//! of each of those DLLs. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_more2};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x5000;
const A: u32 = SCRATCH + 0x1_0000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its label, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

const CASES: &[Case] = &[
    ("o4 more2 count_fade", &[(673, 0x1c70)], 5, |t, f, c| {
        fill(t, A + 0x70, 0x30, c);
        fill(t, A + 0xe0, 0x14, c);
        // counts after the step: 0, 1, 15, 16, -5
        t.set_u32(A + 0x120, [1, 2, 16, 17, 0xffff_fffc][c as usize]);
        fill(t, P, 0x1800, c);
        t.set_u32(t.img.word(f + 0x04), P);
    }),
    (
        "o4 more2 clear_halves_slot",
        &[(519, 0x2cf0)],
        1,
        |t, f, c| {
            let base = t.img.word(f + 0x03);
            fill(t, base - 4, 0x600, c);
        },
    ),
    (
        "o4 more2 slot_chain_jump",
        &[(446, 0x4d30)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x03), c * 0x0001_0100);
            for op in [0x13, 0x2b, 0x31] {
                fill(t, t.img.word(f + op), 1, c ^ op);
            }
            // one part for the tail jump's register_parts, its lock at P + 0x200
            t.stub_edx(772, |_, a| a[1] ^ 0x0772_0000);
            let rp = t.img.rel32_target(f + 0x55);
            t.set_u32(t.img.word(rp + 0x0e), 1);
            fill(t, P + 0x100, 0x20, c);
            t.set_u32(t.img.word(rp + 0x16), P + 0x100);
            t.set_u32(t.img.word(rp + 0x43), P + 0x200);
            t.set_u32(t.img.word(rp + 0x5f), P + 0x200);
        },
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(label, list, cases, set) in CASES {
        let all = copies(label, list);
        assert!(
            std::env::var_os("FFX_GAME_DIR").is_none() || !all.is_empty(),
            "{label}"
        );
        for (n, rva) in all {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            for s in 0..1024u32 {
                t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
            }
            let f = t.at(rva);
            for c in 0..cases {
                set(&mut t, f, c);
                let regs = Call {
                    eax: 0x00ea_0000 + c,
                    ecx: 0x00ec_0000 + rva,
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                };
                t.check_entry(f, regs, &[A], Eax::Checked);
            }
        }
    }
}

#[test]
fn object_node_edx() {
    const LIST: &[(u16, u32)] = &[(307, 0xb480)];
    for &(n, rva) in LIST {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let found = ffx_magic::INTERNAL
            .iter()
            .find(|(_, sig)| t.img.matches(f, sig))
            .map(|(n, _)| *n);
        assert!(found.is_some_and(|n| n.starts_with("o4 more2 object_node_edx")));
        for c in 0..2u32 {
            // three free nodes in the take's pool, the word before them set
            let take = t.img.rel32_target(f + 0x15);
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
            for i in 1..=3 {
                t.write(node(i), &[0x5a; 0x1c]);
                t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
                t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
            }
            t.set_u32(node(1) - 4, 0x5a5a_5a5a);
            t.set_u32(free, node(1));
            t.set_u16(count, 3);
            fill(&mut t, A, 8, c);
            t.set_u32(t.img.word(f + 0x1b), c * (P + 0x40));
            fill(&mut t, P + 0x40, 4, c);
            t.set_u32(t.img.word(f + 0x33), P + 0x40);
            let regs = Call {
                ecx: A,
                edx: 0x0ed0 + c,
                ..Call::default()
            };
            t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o4_more2::object_node_edx(g, img, fr.at, fr.ecx, fr.edx))
            });
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(&[(307, 0xb480)]);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 57
// mutants, 56 killed. The survivor is equivalent: `object_node_edx`'s
// `+0x4` = 0 on the new node, which the pool's take has already done.
