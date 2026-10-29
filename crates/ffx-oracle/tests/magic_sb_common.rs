//! Shared functions outside the node family the shared-B list ported
//! (`ffx_magic::sb_common`), each against the first DLL that carries it.
//! Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;
mod magic_l3;

use ffx_magic::{Ret, sb_common};
use magic::{Call, Eax, SCRATCH};

const LOG: u32 = SCRATCH + 0x4000;

/// Each slot logs its first `n` words in its own row and answers
/// `0x100 + slot`.
fn record(t: &mut magic::Twin, slots: &[(u32, usize)]) {
    for (k, &(slot, n)) in (0u32..).zip(slots) {
        let row = LOG + 0x40 * k;
        t.stub(slot, move |m, a| {
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, *w);
            }
            0x0100 + slot
        });
    }
}

#[test]
fn dll_main_0018() {
    magic_l3::dll_main(18);
}

#[test]
fn set_speed() {
    let Some(mut t) = magic_l3::twin(18) else {
        return;
    };
    record(&mut t, &[(530, 2), (249, 2)]);
    let f = t.at(0x2bc0);
    let w = |o: u32| t.img.word(f + o);
    let (mode, bytes_p, dflt, chr, chrs) = (w(0x02), w(0x0a), w(0x11), w(0x47), w(0x67));
    let bytes = SCRATCH + 0x100;
    t.write(bytes, &[3, 0x1f, 0x20, 0xff, 0]);
    t.set_u32(bytes_p, bytes);
    t.set_u32(chr, 0xc0c0_0001);
    for i in 0..0x100u32 {
        t.set_u32(chrs + 4 * i, 0xc4c4_0000 + i);
    }
    for m in [0u32, 1] {
        for d in [5u32, 0x1f, 0x20, 0xffff_fff0] {
            for index in [u32::MAX, 0, 1, 2, 3] {
                for speed in [0u32, 0x80, 0xffff_ffff, 0x8000_0000] {
                    t.set_u32(mode, m);
                    t.set_u32(dflt, d);
                    t.write(LOG, &[0; 0x80]);
                    t.check(
                        f,
                        Call {
                            eax: 0x11,
                            ecx: index,
                            edx: speed,
                            ..Call::default()
                        },
                        &[],
                        Eax::Checked,
                        |g, img, fr| Ret::eax(sb_common::set_speed(g, img, fr.at, fr.ecx, fr.edx)),
                    );
                }
            }
        }
    }
}
