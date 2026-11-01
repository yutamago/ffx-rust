//! `magic_0586.dll` against its Rust port (`ffx_magic::m0586`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 586;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002ad0: the scale task and the fade node, the list empty and not.
#[test]
fn fade_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2ad0);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x13), fx);
    recs(&mut t, &[(371, 4)]);
    let st = t.img.rel32_target(at + 0x24);
    let take = t.img.rel32_target(st + 0x73);
    let take2 = t.img.rel32_target(at + 0x66);
    let last = SCRATCH + 0x4000;
    for full in [false, true] {
        for tk in [take, take2] {
            magic_hi::pool(&mut t, tk, SCRATCH + 0x6000, 4);
        }
        t.write(fx + 0x8330, &[0x3d; 0xd0]);
        t.write(last, &[0x4c; 0x1c]);
        t.set_u32(t.img.word(at + 0x6c), if full { last } else { 0 });
        t.set_u32(t.img.word(at + 0x84), if full { last } else { 0 });
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::this(0x00ec_0001), &[], Eax::Checked);
    }
}
