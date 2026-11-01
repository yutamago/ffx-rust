//! `magic_0610.dll` against its Rust port (`ffx_magic::m0610`). Needs
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

use ffx_magic::{Ret, m0610};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 610;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x1000a610: no targets; a target with a listed kind; one slot 580
/// rejects; all accepted.
#[test]
fn check_targets() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xa610);
    recs(&mut t, &[(4, 1), (27, 3), (523, 2)]);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 580, 1, move |m, a| u32::from(m.r32(cell) == a[0]));
    let list = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x54), list);
    t.write(list, &[1, 2, 3, 4]);
    let objs = t.img.word(at + 0x5f);
    let kinds = t.img.word(at + 0x6a);
    let listed = t.u32(kinds + 8);
    for (k, id) in (0u32..).zip(1..5u32) {
        let o = SCRATCH + 0x400 + 0x10 * k;
        t.set_u32(objs + 4 * id, o);
        // the half-word after the kind non-zero (a kind read as u32 shows)
        t.set_u32(o, 0x5a5a_fff0 + id);
    }
    for (n, kind, reject) in [
        (0i32, None, 0u32),
        (4, Some(3u32), 0),
        (4, None, 2),
        (4, None, 0),
    ] {
        t.set_u32(t.img.word(at + 0x3b), n.cast_unsigned());
        for id in 1..5u32 {
            let o = SCRATCH + 0x400 + 0x10 * (id - 1);
            let k = if kind == Some(id) {
                listed
            } else {
                0xfff0 + id
            };
            t.set_u16(o, u16::try_from(k & 0xffff).unwrap());
        }
        t.set_u32(cell, reject);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0610::check_targets(g, img, fr))
        });
    }
}
