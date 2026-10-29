//! The object callbacks of `ffx_magic::o4_parts` (a part started on the
//! object, then twelve constant stores) against every DLL's own copy,
//! and `DllMain` of each of those DLLs against every port bound in it.
//! Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};
use ownb::rec;

const D8_RET1: &[(u16, u32)] = &[
    (206, 0x18a0),
    (255, 0x1190),
    (371, 0x1000),
    (376, 0x1000),
    (376, 0x13e0),
    (376, 0x1a00),
    (488, 0x12b0),
    (489, 0x18e0),
    (496, 0x1270),
    (664, 0x1250),
    (664, 0x1bc0),
    (664, 0x1d80),
    (665, 0x1250),
    (665, 0x13b0),
    (665, 0x18d0),
    (665, 0x1a70),
    (666, 0x1670),
    (666, 0x1860),
    (666, 0x2500),
    (666, 0x26d0),
    (669, 0x1810),
    (669, 0x19f0),
];
const D8: &[(u16, u32)] = &[
    (375, 0x1000),
    (375, 0x17d0),
    (375, 0x1900),
    (375, 0x1a80),
    (375, 0x1b20),
    (375, 0x1dd0),
    (375, 0x1e70),
    (375, 0x1f10),
    (376, 0x1ff0),
    (376, 0x21f0),
    (487, 0x1000),
    (498, 0x1000),
];
const E0_RET1: &[(u16, u32)] = &[
    (206, 0x1650),
    (255, 0x1000),
    (376, 0x2290),
    (497, 0x1190),
    (658, 0x1000),
    (669, 0x1b90),
];
const E0: &[(u16, u32)] = &[
    (372, 0x1180),
    (498, 0x1160),
    (536, 0x1200),
    (539, 0x1000),
    (561, 0x1000),
    (616, 0x1000),
    (617, 0x1000),
];

/// A family: its port's name, its copies, the offset of its `rel32`.
type Family = (&'static str, &'static [(u16, u32)], u32);

const OBJ: u32 = SCRATCH + 0x2000;
const LIST: u32 = SCRATCH + 0x400;

/// The part start's state for the copy at `f`: `*G_P`, `*G_D + 0x10`,
/// the program list slot 941 answers, the wrapper's lock (recorded by
/// `pppStartPart`, slot 714).
fn set_up(t: &mut Twin, f: u32, call: u32, seed: u32) {
    t.set_u32(t.img.word(f + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(f + 0x10), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cd00 ^ seed);
    fill(t, LIST, 16, seed);
    let wrapper = t.img.rel32_target(f + call);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 9);
    fill(t, OBJ, 0x20, seed ^ 0x55);
}

fn stubs(t: &mut Twin) {
    rec(t, 941, 1, |_, _| LIST);
    rec(t, 714, 5, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x220, v);
        0x714
    });
}

#[test]
fn part_stores_in_every_copy() {
    let lists: [Family; 4] = [
        ("o4 part stores (d8, ret 1)", D8_RET1, 0x36),
        ("o4 part stores (d8)", D8, 0x36),
        ("o4 part stores (entry 0, ret 1)", E0_RET1, 0x35),
        ("o4 part stores (entry 0)", E0, 0x35),
    ];
    let all: Vec<Vec<(u16, u32)>> = lists.iter().map(|l| copies(l.0, l.1)).collect();
    for n in dlls(&[D8_RET1, D8, E0_RET1, E0]) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        for (copies, l) in all.iter().zip(&lists) {
            for &(_, rva) in copies.iter().filter(|c| c.0 == n) {
                let f = t.at(rva);
                for seed in [1u32, 0x77] {
                    set_up(&mut t, f, l.2, seed ^ rva);
                    t.check_entry(f, Call::this(0x0c0c_0000 | rva), &[OBJ], Eax::Checked);
                }
            }
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    for n in dlls(&[D8_RET1, D8, E0_RET1, E0]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, 16 mutants):
// 14 killed; 2 survive, both equivalent here: the displacement read
// unsigned (every field and list entry these copies use is below 0x80)
// and `ecx` 0 at slot 941 (not a method slot: the harness compares `ecx`
// only for `magic/this_slots.rs`).
