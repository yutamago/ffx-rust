//! `magic_0609.dll` against its Rust port (`ffx_magic::m0609`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0609};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 609;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100095d0: an object with bit 0 of `+0x90` clear and set.
#[test]
fn task_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x95d0);
    recs(&mut t, &[(30, 1), (36, 3), (228, 2)]);
    let tail = SCRATCH + 0x3800;
    let obj = SCRATCH + 0x1000;
    for bits in [0u16, 0x8001] {
        t.write(tail, &[0x1b; 0x20]);
        t.set_u32(t.img.word(at + 0x1), tail);
        // the flag word (bit 0 set, so its clear shows) and the words
        // the start clears, distinct and non-zero
        magic_h6::distinct(&mut t, at, &[0x61, 0x6b, 0x5b]);
        let flag = t.img.word(at + 0x3e);
        t.set_u32(flag, 0x00f1_a6c3);
        t.write(obj, &[0x4e; 0x100]);
        t.set_u16(obj + 0x90, bits);
        let c = Call {
            edx: obj,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0609::task_start(g, img, fr))
        });
    }
}

/// 0x10009a00: the object linked after the old tail.
#[test]
fn link_object() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x9a00);
    let tail = SCRATCH + 0x100;
    t.write(tail, &[0x3c; 0x20]);
    t.set_u32(t.img.word(at + 0x1), tail);
    for v in [0x5a5a_5a5b_u32, 0x8080_8080] {
        for op in [0x3b, 0x42, 0x5d, 0x67] {
            t.set_u32(t.img.word(at + op), v);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
