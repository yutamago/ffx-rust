//! `magic_0707.dll` against its Rust port (`ffx_magic::m0707`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, Xmm, m0707};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 707;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100010b0: one object with negative and fractional times.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x10b0);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    t.set_u32(t.img.word(at + 0x7), SCRATCH + 0x24);
    t.set_u32(t.img.word(at + 0x19), 0x00d0_0019);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    let obj = SCRATCH + 0x1000;
    for (t1, t2, t3) in [(1.5f32, -2.75f32, 100.9f32), (0.0, 7.0, -0.5)] {
        t.write(obj, &[0x5b; 0x80]);
        let c = Call {
            ecx: obj,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(3.25), Xmm::f32(-4.5)]);
        let args = [f(0.75), f(t1), f(t2), f(t3), f(0.1), f(0.2), f(0.3)];
        t.check(at, c, &args, Eax::Checked, |g, img, fr| {
            Ret::eax(m0707::start(g, img, fr))
        });
    }
}
