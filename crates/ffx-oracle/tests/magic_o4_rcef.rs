//! The map effects' task functions of `ffx_magic::o4_rcef` (and the
//! copies of `rcef::create_task_wait` with other wait times) against
//! every DLL's own copy, and `DllMain` of each of those DLLs. Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

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

const ATTACH_SET_PAR: &[(u16, u32)] = &[
    (357, 0x18e0),
    (359, 0x1820),
    (361, 0x1be0),
    (416, 0x1710),
    (521, 0x1200),
    (521, 0x12d0),
    (522, 0x1290),
    (526, 0x1380),
];

#[test]
fn attach_set_par_in_every_copy() {
    let list = copies("o4 attach set par", ATTACH_SET_PAR);
    for n in dlls(&[ATTACH_SET_PAR]) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let moved = SCRATCH + 0x5000;
        for s in [441u32, 442] {
            rec(&mut t, s, 3, move |_, a| a[1] ^ s);
        }
        rec(&mut t, 440, 9, |_, a| a[0] ^ 0x440);
        rec(&mut t, 450, 11, |_, a| a[1] ^ 0x450);
        rec(&mut t, 449, 13, |_, a| a[1] ^ 0x449_0000);
        for &(_, rva) in list.iter().filter(|c| c.0 == n) {
            let f = t.at(rva);
            let g = t.img.word(f + 0x04);
            // slot 443 moves the game object: the set-par call reads it again
            rec(&mut t, 443, 5, move |m, a| {
                m.w32(g, moved);
                a[1] ^ 0x443
            });
            let d = t.img.word(f + 0x10);
            for k in 0..2u32 {
                let obj = SCRATCH + 0x2000 + 0x1000 * k;
                let p = SCRATCH + 0x4000 + 0x400 * k;
                t.set_u32(g, p);
                t.set_u32(p + d, 0x1980_0000 + rva + k);
                for i in 0..8 {
                    let field = t.img.word(f + 0x42 + 10 * i + 2);
                    t.set_u32(obj + field, 0x5a5a_0000 + i);
                }
                t.check_entry(f, Call::this(0x4444), &[obj], Eax::Checked);
            }
        }
    }
}

const ATTACH_PLAIN: &[(u16, u32, u32)] = &[
    (300, 0x1300, 8),
    (301, 0x1de0, 8),
    (510, 0x15c0, 8),
    (529, 0x1270, 8),
    (301, 0x1d00, 12),
    (360, 0x1400, 12),
    (360, 0x14e0, 12),
    (510, 0x14e0, 12),
];

#[test]
fn attach_plain_in_every_copy() {
    for &(n, rva, stores) in ATTACH_PLAIN {
        let label = format!("o4 attach plain {stores}");
        if copies(&label, &[(n, rva)]).is_empty() {
            return;
        }
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        rec(&mut t, 440, 7, |_, a| a[0] ^ 0x440);
        rec(&mut t, 450, 9, |_, a| a[1] ^ 0x450);
        rec(&mut t, 449, 11, |_, a| a[1] ^ 0x449_0000);
        let f = t.at(rva);
        let g = t.img.word(f + 0x04);
        let moved = SCRATCH + 0x5000;
        // the attach moves the game object: the set-par call reads it again
        for s in [441u32, 442] {
            rec(&mut t, s, 3, move |m, a| {
                m.w32(g, moved);
                a[1] ^ s
            });
        }
        let d = t.img.word(f + 0x10);
        for k in 0..2u32 {
            let obj = SCRATCH + 0x2000 + 0x1000 * k;
            let p = SCRATCH + 0x4000 + 0x400 * k;
            t.set_u32(g, p);
            t.set_u32(p + d, 0x1980_0000 + rva + k);
            for i in 0..stores {
                let field = t.img.word(f + 0x32 + 10 * i + 2);
                t.set_u32(obj + field, 0x5a5a_0000 + i);
            }
            t.check_entry(f, Call::this(0x4444), &[obj], Eax::Checked);
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let plain: Vec<(u16, u32)> = ATTACH_PLAIN.iter().map(|c| (c.0, c.1)).collect();
    for n in dlls(&[SEQ_WAIT_447, CREATE_TASK_WAIT, ATTACH_SET_PAR, &plain]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py): 15 mutants of
// `o4_rcef` and the generalised wait time of `rcef::create_task_wait`,
// all killed; `attach_set_par` and `attach_plain`: 21 more, all killed.
