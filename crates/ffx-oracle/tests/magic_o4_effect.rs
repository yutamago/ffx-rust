//! The effect start-ups of `ffx_magic::o4_effect` against every DLL's
//! own copy, and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH};
use o4::{copies, dlls, fill};
use ownb::rec;

const START_TURNED: &[(u16, u32)] = &[
    (596, 0x7060),
    (162, 0x29f0),
    (304, 0x1c10),
    (307, 0x2430),
    (568, 0x1860),
    (586, 0x2750),
    (587, 0x1890),
    (588, 0x1b70),
    (589, 0x1e00),
    (598, 0x7470),
    (605, 0x7db0),
];

const CASTER: u32 = SCRATCH + 0x8000;

#[test]
fn start_turned_in_every_copy() {
    for (n, rva) in copies("o4 start turned", START_TURNED) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        // the reset's camera slots and the matrix slots
        rec(&mut t, 287, 3, |_, _| 0x287);
        rec(&mut t, 283, 2, |_, a| a[1] ^ 0x283);
        rec(&mut t, 281, 1, |_, _| 0x281);
        rec(&mut t, 235, 1, |m, a| {
            m.w32(a[0], 0x3f80_0000);
            0x235
        });
        rec(&mut t, 329, 3, |m, a| {
            let v = m.r32(a[1]);
            m.w32(a[0], v ^ a[2]);
            0x329
        });
        let reset = t.img.rel32_target(f + 0x08);
        let mode = t.img.word(reset + 0x42);
        t.set_u32(t.img.word(f + 0x03), CASTER);
        // the caster's fields the start reads: the angle, the position
        let fields: Vec<u32> = [0x29, 0x4d, 0x5d, 0x6d, 0x80]
            .iter()
            .map(|&o| t.img.word(f + o))
            .collect();
        for m in [0u32, 1] {
            t.set_u32(mode, m);
            for (i, &d) in (0u32..).zip(&fields) {
                fill(&mut t, CASTER + d - 4, 3, rva ^ m ^ i << 8);
            }
            // the flag byte the start sets holds something else first
            t.set_u32(t.img.word(f + 0x13), 0x5a5a_5a5a);
            t.check_entry(f, Call::this(0x77), &[], Eax::Checked);
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    for n in dlls(&[START_TURNED]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, 13 mutants): 11
// killed; 2 survive, equivalent: the unit matrix and both matrix
// arguments of slot 329 are the same global in every copy (`M, M`), so
// swapping or exchanging those operands changes nothing.
