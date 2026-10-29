//! `magic_0009.dll` against its Rust port (`ffx_magic::m0009`): `DllMain`
//! against every bound port, the effect set-up and the model fill (both
//! internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0009};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, rec, recs, twin};

const N: u16 = 9;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x19d0);
    let fx = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x11), fx);
    rec(&mut t, 719, 6, |_, a| 0x0719_0000 + a[2] * 0x10 + a[1]);
    recs(&mut t, &[(235, 1), (330, 3), (328, 3), (329, 3)]);
    let (pos, from, rot) = (SCRATCH + 0x100, SCRATCH + 0x110, SCRATCH + 0x120);
    for (p, q) in [
        ([1.0f32, 2.0, 3.0, 1.0], [0.5f32, -1.0, 7.0]),
        ([-4.5, 0.25, 10.0, 0.0], [-4.5, 0.25, 10.0]),
        ([100.0, -3.0, 0.125, 2.0], [1e-3, 8.0, -6.0]),
    ] {
        for (i, v) in (0u32..).zip(p) {
            t.set_u32(pos + 4 * i, f(v));
        }
        for (i, v) in (0u32..).zip(q) {
            t.set_u32(from + 4 * i, f(v));
            t.set_u32(rot + 4 * i, f(v * 0.5 + 0.1));
        }
        // the block dirty before
        t.write(fx, &[0xa5; 0xa00]);
        let c = Call {
            ecx: pos,
            edx: from,
            ..Call::default()
        };
        t.check(at, c, &[rot], Eax::Junk, |g, img, fr| {
            m0009::setup(g, img, fr.at, fr.ecx, fr.edx, fr.arg(g, 0));
            Ret::default()
        });
    }
}

#[test]
fn fill() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2aa0);
    let lock = SCRATCH + 0x200;
    t.set_u32(t.img.word(at + 0x21), lock);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 + a[1]);
    // the flag during the call, recorded with the arguments
    rec(&mut t, 779, 4, move |m, a| {
        let v = m.r32(lock);
        m.w32(SCRATCH + 0x300 + 4 * (a[3] % 0x46), v ^ a[0]);
        0
    });
    for saved in [0u32, 7] {
        t.set_u32(lock, saved);
        let c = Call::this(SCRATCH + 0x800);
        t.check(at, c, &[], Eax::Junk, |g, img, fr| {
            m0009::fill(g, img, fr.at, fr.ecx);
            Ret::default()
        });
    }
}
