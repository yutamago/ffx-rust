//! `magic_0429.dll` against its Rust ports (`ffx_magic::m0429`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 429;

/// 0x100014f0: not started, the timeline running and its last frame (the
/// stop and slot 214).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x14f0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 253, 0, |_, _| 1);
    for (s, n) in [(723u32, 1usize), (724, 2), (214, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let p = SCRATCH + 0x600;
    let q = SCRATCH + 0x610;
    t.set_u32(w(&t, 0x0d), p);
    t.set_u32(w(&t, 0x3f), q);
    t.set_u32(w(&t, 0x13), 0x00ab_0013);
    t.set_u32(w(&t, 0x31), 0x00ab_0031);
    t.set_u32(w(&t, 0x62), 0x00ab_0062);
    t.set_u32(w(&t, 0x44), 0x00ab_0044);
    let tl = t.img.rel32_target(at + 0x4e);
    let live = t.img.word(tl + 0x06);
    // no events: a live timeline only asks slot 253
    t.set_u32(t.img.word(tl + 0x25), 0);
    for (started, n_live) in [(0u32, 1u32), (1, 1), (1, 0), (2, 3), (2, 0)] {
        t.set_u32(w(&t, 0x02), started);
        t.set_u32(live, n_live);
        t.set_u32(p, 0x77);
        t.set_u32(q, 0x88);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
