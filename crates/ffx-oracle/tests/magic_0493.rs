//! `magic_0493.dll` against its Rust port (`ffx_magic::m0493`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, Xmm, m0493};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, twin};

const N: u16 = 493;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10006ae0: the node list empty and not.
#[test]
fn init() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6ae0);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, f(i as f32 * 2.5 - 1.0));
        }
        0x268
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let (obj, e) = (SCRATCH + 0x1000, SCRATCH + 0x3000);
    let ring = SCRATCH + 0x4000;
    let last = SCRATCH + 0x4100;
    for full in [false, true] {
        t.write(obj, &[0x4e; 0xb40]);
        t.write(e, &[0x2d; 0x60]);
        t.set_u32(e + 0x54, 0x00e5_0004);
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(w(&t, 0x27), ring);
        let sentinel = w(&t, 0x7b);
        t.write(last, &[0x1c; 0x20]);
        t.set_u32(w(&t, 0x6b), if full { last } else { sentinel });
        t.set_u16(w(&t, 0x8f), 3);
        t.set_u32(w(&t, 0x45), 0x8765_4321);
        let c = Call {
            ecx: obj,
            edx: e,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(1.0), Xmm::f32(2.0), Xmm::f32(-0.75)]);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0493::init(g, img, fr))
        });
    }
}
