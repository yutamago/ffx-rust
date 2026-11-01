//! `magic_0597.dll` against its Rust port (`ffx_magic::m0597`). Needs
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

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{recs, twin};

const N: u16 = 597;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x1000aee0: below and from phase 4, the flag clear and set, no, one
/// and two children, the halfword 0 and not.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let f = t.at(0xaee0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let bind = t.img.rel32_target(f + 0x23);
    t.set_u32(t.img.word(bind + 0x7e), SCRATCH + 0x500);
    recs(&mut t, &[(723, 1), (724, 1), (230, 1), (253, 0), (214, 1)]);
    // the children's method logs its cursor's node and, on the last
    // child, empties the root's list
    let first = w(&t, 0x5e);
    let root = w(&t, 0x66);
    let log = SCRATCH + 0x2_f000;
    t.stub(1000, move |m, a| {
        let k = m.r32(log + 0x7c).wrapping_add(1);
        m.w32(log + 0x7c, k);
        m.w32(log + 4 * (k % 16), m.r32(a[0]) ^ (m.r32(a[0] + 4) & 0xff00));
        if m.r32(m.r32(a[0]) + 8) == root {
            m.w32(first, root);
        }
        0x1000
    });
    let vt = SCRATCH + 0x800;
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 1000);
    t.set_u32(w(&t, 0x18), SCRATCH + 0x400);
    let pass = w(&t, 0x32);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    for (phase, flag, kids, half) in [
        (3u32, 0u32, 0u32, 0u16),
        (4, 0, 2, 0),
        (5, 0, 0, 0),
        (5, 1, 2, 0),
        (5, 1, 0, 1),
        (6, 0, 1, 0),
    ] {
        t.set_u32(w(&t, 0x0b), phase);
        t.set_u32(w(&t, 0x57), flag);
        t.set_u32(w(&t, 0xa0), 0x10 | (phase & 1));
        t.set_u32(w(&t, 0xe3), 0xff);
        t.set_u32(w(&t, 0x28), 0x0023_0000 + phase);
        t.set_u16(w(&t, 0xf7), half);
        t.set_u32(w(&t, 0x105), 0x00fd_0000);
        t.set_u8(pass, 0x41);
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { root } else { kid(i + 1) });
        }
        t.set_u32(first, if kids == 0 { root } else { kid(0) });
        let bone = w(&t, 0x48);
        t.set_u32(bone + 0x10, bone);
        t.set_u8(bone + 0x1a, 0x41);
        for at in [w(&t, 0xb9), w(&t, 0xd9)] {
            t.set_u32(at + 0x10, at);
            t.set_u8(at + 0x1a, 0);
            t.set_u8(at + 0x1b, 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
