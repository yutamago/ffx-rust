//! The node constructors of `ffx_magic::o4_ctor` against their DLLs,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use magic::{Call, Eax, SCRATCH};
use o4::{copies, dlls, fill};

const OBJ: u32 = SCRATCH + 0x2000;

/// Each constructor: its name, copy, `ecx` (the object for a method).
const CASES: &[(&str, (u16, u32), u32)] = &[
    ("effect_ctor_arg", (159, 0x6700), 0),
    ("effect_ctor_arg2", (202, 0x6f10), 0),
    ("effect_ctor_this", (388, 0x6d70), OBJ),
    ("big_ctor_this", (597, 0x8f40), OBJ),
    ("big_table_ctor", (262, 0x6c10), 0),
    ("colour_ctor", (70, 0x97b0), 0),
    ("vec_ctor", (71, 0x7530), 0),
];

#[test]
fn constructors() {
    for &(name, (n, rva), ecx) in CASES {
        let label = format!("o4 ctor {name}");
        if copies(&label, &[(n, rva)]).is_empty() {
            return;
        }
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        for s in 0..1024u32 {
            t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
        }
        let f = t.at(rva);
        // every word a record names: the nodes and their lists
        for k in 0..0x40u32 {
            let a = t.img.word(f + k);
            if (t.img.at(0)..t.img.at(0) + 0x0100_0000).contains(&a) && !t.img.in_text(a) {
                fill(&mut t, a & !3, 0x40, a);
            }
        }
        fill(&mut t, OBJ, 0x40, rva);
        let regs = Call {
            eax: 0x00ea_0000,
            ecx: if ecx == 0 { 0x00ec_0000 } else { ecx },
            edx: 0x00ed_0000,
            ..Call::default()
        };
        let args = [OBJ + 0x100, OBJ + 0x200, OBJ + 0x300, OBJ + 0x400];
        t.check_entry(f, regs, &args, Eax::Checked);
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let lists: Vec<(u16, u32)> = CASES.iter().map(|c| c.1).collect();
    for n in dlls(&[&lists]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line): 47 mutants, all killed.
