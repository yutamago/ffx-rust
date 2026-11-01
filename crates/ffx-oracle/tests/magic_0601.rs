//! `magic_0601.dll` against its Rust port (`ffx_magic::m0601`). Needs
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

use ffx_magic::m0601;
use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 601;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10007d30: no parent, an empty parent and one with a child.
#[test]
fn static_node() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7d30);
    let (parent, child) = (SCRATCH + 0x100, SCRATCH + 0x200);
    for case in 0..3 {
        t.write(parent, &[0x2b; 0x20]);
        t.write(child, &[0x2c; 0x20]);
        t.set_u32(parent + 0x10, if case == 2 { child } else { parent });
        let c = Call {
            edx: if case == 0 { 0 } else { parent },
            ..Call::default()
        };
        t.check(
            at,
            c,
            &[0x00a0_0000, 0x00b0_0000],
            Eax::Checked,
            |g, img, fr| ffx_magic::Ret::eax(m0601::static_node(g, img, fr)),
        );
    }
}

/// 0x1000a3b0: one set of globals.
#[test]
fn end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xa3b0);
    magic_owna::recs(
        &mut t,
        &[
            (694, 4),
            (384, 5),
            (24, 7),
            (25, 9),
            (211, 11),
            (688, 14),
            (213, 16),
        ],
    );
    for (i, op) in (0u32..).zip([0x1c, 0x53, 0x68, 0x7b]) {
        t.set_u32(t.img.word(at + op), 0x00a0_0000 + i);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
