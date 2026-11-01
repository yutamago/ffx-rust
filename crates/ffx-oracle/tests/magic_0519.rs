//! `magic_0519.dll` against its Rust port (`ffx_magic::m0519`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0519};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{recs, twin};

const N: u16 = 519;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100022b0, 0x10002600 (levels 0, 1, 7, -3), 0x100026b0 (frames
/// around both ramps and past them).
#[test]
fn internals() {
    let Some(mut t) = twin(N) else { return };
    recs(
        &mut t,
        &[
            (247, 2),
            (327, 2),
            (821, 1),
            (822, 4),
            (823, 2),
            (824, 4),
            (776, 1),
            (915, 5),
        ],
    );
    let p = SCRATCH + 0x100;
    for i in 0..4u32 {
        t.set_u32(p + 4 * i, f(i as f32 * 2.5 - 1.0));
    }
    let at = t.at(0x22b0);
    // the flag byte's neighbours non-zero (a wider store shows)
    let flag = t.img.word(at + 0x8);
    t.set_u32(flag, 0x5a5a_5a5a);
    t.check(at, Call::this(p), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0519::place(g, img, fr.at, fr.ecx))
    });
    let at = t.at(0x2600);
    for n in [0u32, 1, 7, 0xffff_fffd] {
        let c = Call {
            eax: 0x00ea_0000,
            ecx: n,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0519::filter(g, img, fr))
        });
    }
    let at = t.at(0x26b0);
    for n in [0u32, 3, 4, 5, 18, 19, 40, 0xffff_fff0] {
        t.check(at, Call::this(n), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0519::fade(g, img, fr.at, fr.ecx))
        });
    }
}
