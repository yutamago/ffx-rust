//! The timer, table reset and flag step of `ffx_magic::o4_list` against
//! every copy in its own DLL, and `DllMain` of each of those DLLs. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const S: u32 = SCRATCH + 0x5000;
const A: u32 = SCRATCH + 0x8000;
const RING: u32 = SCRATCH + 0x9000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its label, copies, how many set-ups, the set-up and
/// whether `ecx` is the object.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp, bool);

/// The timer cases: idle with the bit clear, idle with it set, running
/// at 10000, at another value.
fn timer(t: &mut Twin, _f: u32, c: u32) {
    fill(t, S, 0xc0, c);
    fill(t, RING, 0x10, c);
    // the node and its two neighbours
    for (n, next, prev) in [(0, 0x10, 0x20), (0x10, 0x20, 0), (0x20, 0, 0x10)] {
        t.set_u32(RING + n + 4, RING + next);
        t.set_u32(RING + n + 8, RING + prev);
    }
    t.set_u32(A, RING);
    t.set_u32(A + 4, S);
    t.set_u32(S + 0x18, [0, 0, 10000, 777][c as usize]);
    t.set_u32(S + 0x2d8, [0xfb, 4, 0, 0][c as usize]);
}

const CASES: &[Case] = &[
    ("o4 list timer_unlink", &[(513, 0x6d50)], 4, timer, false),
    (
        "o4 list table_reset",
        &[(523, 0x21b0)],
        1,
        |t, f, c| {
            fill(t, t.img.word(f + 0x08), 0x44, c);
            let end = t.img.word(f + 0x4a);
            fill(t, end, 4, c);
        },
        false,
    ),
    (
        "o4 list this_flag_step",
        &[(273, 0x7390)],
        2,
        |t, _f, c| {
            fill(t, A, 0x30, c);
            // bits 0x200 and 0x120 clear, so both `or`s show
            t.set_u32(A + 0x90, 0x5a5a_5850 | c);
        },
        true,
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(label, list, cases, set, this) in CASES {
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
                    ecx: if this { A } else { 0x00ec_0000 + rva },
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                };
                t.check_entry(f, regs, &[A], Eax::Checked);
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
// code line, survivors rerun after the tests were strengthened): 58
// mutants, 56 killed. The survivors are equivalent: the timer started at
// 10000 is always set to 20000 a few instructions later (two).
