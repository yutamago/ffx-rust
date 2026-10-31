//! `magic_0168.dll` against its Rust port (`ffx_magic::m0168`): the
//! camera frame. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 168;

/// 0x10003890: the countdown above 0 and at 0.
#[test]
fn camera_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3890);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 562, 0, |_, _| 0x562);
    for (s, n) in [(723u32, 1usize), (724, 2)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, s + i);
            }
            s
        });
    }
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3800 + 4 * i, v);
        }
        0x231
    });
    let p = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x1d), p);
    for n in [3u32, 0, 0x8000_0000] {
        t.set_u32(p + 0x3158, n);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
