//! `magic_0480.dll` against its Rust port (`ffx_magic::m0480`). Needs
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

use ffx_magic::{Ret, m0480};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 480;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002760: no node; a lone node into an empty free ring; a node with
/// siblings and two children that is the pool's cursor, head and tail,
/// into a ring, the count reaching 0x100.
#[test]
fn clear() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2760);
    recs(&mut t, &[(590, 2), (589, 2)]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let gb = t.img.rel32_target(at + 0x84);
    let n = SCRATCH + 0x1000;
    let (prev, next, c1, c2, c3) = (
        SCRATCH + 0x1100,
        SCRATCH + 0x1200,
        SCRATCH + 0x1300,
        SCRATCH + 0x1400,
        SCRATCH + 0x1500,
    );
    let ring = SCRATCH + 0x1600;
    for case in 0..3 {
        t.write(SCRATCH + 0x1000, &[0x31; 0x700]);
        for g in [gb, at] {
            let (cur, head, tail, free, count) = if g == at {
                (0xaa, 0xb7, 0xc6, 0xd6, 0xf0)
            } else {
                (0x48, 0x55, 0x64, 0x74, 0x8e)
            };
            let [cur, head, tail, free, count] =
                [cur, head, tail, free, count].map(|op: u32| t.img.word(g + op));
            let mark = if case == 2 { n } else { SCRATCH + 0x3f00 };
            t.set_u32(cur, mark);
            t.set_u32(head, mark);
            t.set_u32(tail, mark);
            t.set_u32(free, if case == 1 { 0 } else { ring });
            t.set_u16(count, if case == 2 { 0xff } else { 4 });
        }
        t.set_u32(ring, ring);
        t.set_u32(ring + 4, ring);
        t.set_u32(w(&t, 0x5b), if case == 0 { 0 } else { n });
        if case == 2 {
            // n's children c1 -> c2 -> c3 (c3's +0xc 0 ends them)
            t.set_u32(n, prev);
            t.set_u32(n + 4, next);
            t.set_u32(n + 8, c1);
            for (c, nx) in [(c1, c2), (c2, c3), (c3, 0)] {
                t.set_u32(c, 0);
                t.set_u32(c + 4, 0);
                t.set_u32(c + 8, c);
                t.set_u32(c + 0xc, nx);
            }
        } else {
            t.set_u32(n, 0);
            t.set_u32(n + 4, 0);
            t.set_u32(n + 8, n);
            t.set_u32(n + 0xc, 0);
        }
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0480::clear(g, img, fr.at))
        });
    }
}
