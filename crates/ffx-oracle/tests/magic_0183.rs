//! `magic_0183.dll` against its Rust port (`ffx_magic::m0183`): the
//! camera point (internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0183};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, rec, twin};

const N: u16 = 183;

/// 0x100012e0: two points.
#[test]
fn camera_point() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x12e0);
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..3 {
            let v = m.r32(a[0] + 4 * i) ^ a[2];
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x233
    });
    t.set_u32(t.img.word(at + 0x04), f(0.75));
    let v = SCRATCH + 0x100;
    for k in [0u32, 1] {
        for i in 0..4 {
            t.set_u32(
                v + 4 * i,
                f(1.5 + f32::from(u16::try_from(k * 4 + i).unwrap())),
            );
        }
        t.check(at, Call::this(v), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0183::camera_point(g, img, fr.at, fr.ecx))
        });
    }
}
