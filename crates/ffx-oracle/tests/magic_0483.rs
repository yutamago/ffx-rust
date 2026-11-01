//! `magic_0483.dll` against its Rust port (`ffx_magic::m0483`). Needs
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

use ffx_magic::{Ret, Xmm, m0483};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 483;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001040 and 0x10001140: the start and the end.
#[test]
fn start_and_end() {
    let Some(mut t) = twin(N) else { return };
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(
        &mut t,
        &[
            (861, 3),
            (259, 3),
            (718, 1),
            (222, 4),
            (221, 4),
            (713, 1),
            (804, 1),
        ],
    );
    let start = t.at(0x1040);
    for op in [0xb0, 0xba, 0xc4, 0xce, 0xd8, 0xe2, 0xec] {
        let a = t.img.word(start + op);
        t.set_u32(a, 0x77);
    }
    t.check_entry(start, Call::default(), &[0x00a6_0000], Eax::Checked);
    t.check_entry(t.at(0x1140), Call::default(), &[], Eax::Checked);
}

/// 0x10001260: the part off, on with slot 712 saying 0 and 1.
#[test]
fn draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1260);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.25 - 0.5));
        }
        0x237
    });
    rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(&mut t, &[(258, 1), (231, 3), (230, 2)]);
    let part = SCRATCH + 0x1000;
    for (on, answer) in [(0u16, 0u32), (1, 0), (0x8000, 1)] {
        t.write(part, &[0x2f; 0x80]);
        t.set_u16(part + 0x1e, on);
        t.set_u32(cell, answer);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: part,
            edx: 0x0bed_0000,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(1.0), Xmm::f32(2.0), Xmm::f32(-3.5)]);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0483::draw(g, img, fr))
        });
    }
}
