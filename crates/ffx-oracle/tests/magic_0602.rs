//! `magic_0602.dll` against its Rust port (`ffx_magic::m0602`). Needs
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

use ffx_magic::{Ret, m0602};
use magic::{Call, Eax, SCRATCH};
use magic_hi::lone_node;
use magic_owna::{rec, recs, twin};

const N: u16 = 602;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10006a20: each of the six answers non-zero in turn, then none.
#[test]
fn sounds_free() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6a20);
    let cell = SCRATCH + 0x10;
    // the answers: bit 2k for slot 5 on the k-th sound, bit 2k + 1 for 28
    rec(&mut t, 5, 1, move |m, a| {
        m.r32(cell) >> (2 * (a[0] - 0x301a)) & 1
    });
    rec(&mut t, 28, 2, move |m, a| {
        m.r32(cell) >> (2 * (a[0] - 0x301a) + 1) & 1
    });
    for bit in [
        Some(0u32),
        Some(1),
        Some(2),
        Some(3),
        Some(4),
        Some(5),
        None,
    ] {
        t.set_u32(cell, bit.map_or(0, |b| 1 << b));
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0602::sounds_free(g, img, fr))
        });
    }
}

/// 0x100077c0: no node and a node to give back.
#[test]
fn sounds_stop() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x77c0);
    recs(&mut t, &[(384, 1), (25, 4), (24, 5), (271, 16)]);
    let e = SCRATCH + 0x80;
    t.set_u32(t.img.word(at + 0x1), e);
    t.write(e, &[0, 1, 2, 3, 5, 7, 0, 0]);
    let table = t.img.word(at + 0xc);
    for k in 0..8 {
        t.set_u32(table + 4 * k, 0x00ac_0000 + k);
    }
    let give = t.img.rel32_target(at + 0xda);
    let node = SCRATCH + 0x1000;
    for full in [false, true] {
        if full {
            lone_node(&mut t, give, node);
        }
        t.set_u32(t.img.word(at + 0xce), if full { node } else { 0 });
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0602::sounds_stop(g, img, fr))
        });
    }
}
