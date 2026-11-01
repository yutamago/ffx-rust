//! `magic_0686.dll` against its Rust port (`ffx_magic::m0686`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax};
use magic_owna::{rec, recs, twin};

const N: u16 = 686;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002020: the model placed, the motion script stepped (no record:
/// its done word set), `G_DONE` 0 and not.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2020);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f80_0000 + i);
        }
        0x235
    });
    recs(&mut t, &[(231, 3), (211, 2)]);
    t.set_u32(t.img.word(at + 0x5d), 0x005d_0001);
    // the step's record table: the current record's motion 0 (it stops)
    let step = t.img.rel32_target(at + 0x6d);
    let tab = magic::SCRATCH + 0x4000;
    t.set_u32(t.img.word(step + 0xb), tab);
    t.set_u32(t.img.word(step + 0x2), 1);
    t.write(tab, &[0; 0x40]);
    for done in [0u32, 1] {
        t.set_u32(t.img.word(at + 0x78), done);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
