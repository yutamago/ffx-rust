//! The map effects' task functions of `ffx_magic::o4_rcef` (and the
//! copies of `rcef::create_task_wait` with other wait times) against
//! every DLL's own copy, and `DllMain` of each of those DLLs. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::map::{self, Var};
use magic::{Call, Eax, SCRATCH};
use o4::{copies, dlls};
use ownb::rec;

const SEQ_WAIT_447: &[(u16, u32)] = &[
    (191, 0x1110),
    (357, 0x10a0),
    (360, 0x1110),
    (360, 0x1210),
    (361, 0x1210),
    (361, 0x1490),
    (384, 0x12a0),
    (384, 0x13b0),
    (421, 0x12b0),
    (522, 0x1780),
    (525, 0x1410),
    (535, 0x12a0),
];
const CREATE_TASK_WAIT: &[(u16, u32)] = &[
    (191, 0x1160),
    (300, 0x1120),
    (301, 0x1a30),
    (378, 0x1080),
    (416, 0x10a0),
    (521, 0x1090),
    (522, 0x11b0),
];

#[test]
fn seq_wait_447_in_every_copy() {
    let list = copies("o4 seq wait (447)", SEQ_WAIT_447);
    for n in dlls(&[SEQ_WAIT_447]) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        rec(&mut t, 447, 2, |_, a| a[0] ^ 0x447);
        rec(&mut t, 450, 4, |_, a| a[1] ^ 0x450);
        rec(&mut t, 451, 6, |_, a| a[1] ^ 0x451_0000);
        for &(_, rva) in list.iter().filter(|c| c.0 == n) {
            let f = t.at(rva);
            let g = t.img.word(f + 0x04);
            for k in 0..2u32 {
                let obj = SCRATCH + 0x2000 + 0x1000 * k;
                let p = SCRATCH + 0x4000 + 0x400 * k;
                t.set_u32(g, p);
                t.set_u32(p + 0x198, 0x1980_0000 + rva + k);
                t.check_entry(f, Call::this(0x4444), &[obj], Eax::Checked);
            }
        }
    }
}

#[test]
fn create_task_wait_in_every_copy() {
    let list = copies("o4 create task wait", CREATE_TASK_WAIT);
    for &(n, rva) in &list {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        map::stubs(&mut t);
        map::check(&mut t, rva, Var::Plain(0xf));
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    for n in dlls(&[SEQ_WAIT_447, CREATE_TASK_WAIT]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py): 15 mutants of
// `o4_rcef` and the generalised wait time of `rcef::create_task_wait`,
// all killed.
