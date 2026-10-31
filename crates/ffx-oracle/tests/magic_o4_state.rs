//! The state block resets of `ffx_magic::o4_state` (internal functions)
//! against every DLL's own copy, and `DllMain` of each of those DLLs.
//! Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_state};
use magic::{Call, Eax, SCRATCH};
use o4::dlls;

const ARG: &[(u16, u32)] = &[
    (182, 0x1a50),
    (197, 0x19b0),
    (199, 0x2b20),
    (252, 0x1ee0),
    (266, 0x3900),
    (311, 0x2b40),
    (365, 0x1560),
    (447, 0x3200),
    (475, 0x26c0),
];
const BARE: &[(u16, u32)] = &[
    (245, 0x23f0),
    (298, 0x2ce0),
    (308, 0x1ec0),
    (435, 0x2790),
    (671, 0x2b50),
];
const EDX: &[(u16, u32)] = &[(108, 0x1af0), (221, 0x1af0), (419, 0x37c0)];

/// The copy at `rva` is the internal function `name` registers.
fn registered(t: &magic::Twin, rva: u32, name: &str) {
    let a = t.at(rva);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(a, sig))
        .map(|(n, _)| *n);
    assert_eq!(
        found,
        Some(format!("{name} {:04}@{:04x}", t.img.number, rva & 0xffff).as_str())
    );
}

#[test]
fn resets_in_every_copy() {
    let this = SCRATCH + 0x1000;
    let regs = Call {
        eax: 0x1234,
        ecx: this,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    for n in dlls(&[ARG, BARE, EDX]) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        for &(_, rva) in ARG.iter().filter(|c| c.0 == n) {
            registered(&t, rva, "o4 state reset (arg)");
            t.write(this + 0xb0, &[0x5a; 0x90]);
            t.check(
                t.at(rva),
                regs,
                &[0x1111, 0x2222],
                Eax::Junk,
                |g, img, fr| {
                    let arg = fr.arg(g, 1);
                    o4_state::reset_state_arg(g, img, fr.at, fr.ecx, arg);
                    Ret::default()
                },
            );
        }
        for &(_, rva) in BARE.iter().filter(|c| c.0 == n) {
            registered(&t, rva, "o4 state reset (bare)");
            t.write(this + 0xb0, &[0xa5; 0x90]);
            t.check(
                t.at(rva),
                regs,
                &[0x1111, 0x2222],
                Eax::Junk,
                |g, img, fr| {
                    o4_state::reset_state_bare(g, img, fr.at, fr.ecx);
                    Ret::default()
                },
            );
        }
        for &(_, rva) in EDX.iter().filter(|c| c.0 == n) {
            registered(&t, rva, "o4 state reset (edx)");
            t.write(this + 0xb0, &[0x3c; 0x90]);
            t.check(
                t.at(rva),
                regs,
                &[0x1111, 0x2222],
                Eax::Junk,
                |g, img, fr| {
                    let arg = fr.arg(g, 1);
                    o4_state::reset_state_edx(g, img, fr.at, fr.ecx, fr.edx, arg);
                    Ret::default()
                },
            );
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    for n in dlls(&[ARG, BARE, EDX]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, 6 mutants): 5
// killed; the bare reset's `+0x11c` read as 0 survives, equivalent (the
// constant is 0 in all five copies).
