//! `magic_0090.dll` against its Rust port (`ffx_magic::m0090`): `DllMain`
//! against every bound port and the staged task step. Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, rec, twin};

const N: u16 = 90;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// Every stage below and past its time, and a stage past the last.
#[test]
fn step() {
    let Some(mut t) = twin(N) else { return };
    let e = t.at(0x66c0);
    rec(&mut t, 239, 3, |_, a| a[0] ^ 0x239);
    t.set_u32(t.img.word(e + 0x195), 0x55);
    // the fade block's inputs
    for op in [0x5cu32, 0x8d, 0xc8, 0xfc, 0x117, 0x12a, 0x132, 0x142, 0x14a] {
        let a = t.img.word(e + op);
        t.set_u32(a, f(0.25 + op as f32 / 1024.0));
    }
    let task = SCRATCH + 0x100;
    let obj = SCRATCH + 0x200;
    let (node, prev, next) = (SCRATCH + 0x300, SCRATCH + 0x340, SCRATCH + 0x380);
    t.set_u32(task, node);
    t.set_u32(task + 4, obj);
    t.set_u32(t.img.word(e + 0x66), 3);
    let d = f32::from_bits(t.u32(t.img.word(e + 0x16)));
    for (stage, time) in [(0u16, 0x1abu32), (1, 0x17e), (2, 0x43), (5, 0x43)] {
        let k = f32::from_bits(t.u32(t.img.word(e + time)));
        // far below, just past (by half a unit) and far past the time
        for clock in [-100.0f32, k - d + 0.5, 1e6] {
            t.set_u32(node + 4, prev);
            t.set_u32(node + 8, next);
            t.set_u16(obj + 0x2c8, stage);
            t.set_u32(obj + 0x2c4, f(clock));
            t.check_entry(e, Call::default(), &[task], Eax::Checked);
        }
    }
}
