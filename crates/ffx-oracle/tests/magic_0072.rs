//! `magic_0072.dll` against its Rust port (`ffx_magic::m0072`): `DllMain`
//! against every bound port, the sprite pool (internal) and the placing
//! callback. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0072};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{dump, f, filler, keeper, rec, recs, twin};

const N: u16 = 72;
const BUF: u32 = SCRATCH + 0x8000;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The set-up with two, one and no free task nodes (the last writes
/// through node 0 in the original too, so it runs only on a mapped page:
/// skipped), the list empty and not; then takes until the pool is full.
#[test]
fn pool() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2610);
    let (free, nfree, head, tail) = (
        t.img.word(at + 0x3e),
        t.img.word(at + 0x20),
        t.img.word(at + 0xa6),
        t.img.word(at + 0xb7),
    );
    let nodes = [SCRATCH + 0x200, SCRATCH + 0x240, SCRATCH + 0x280];
    for (n, h) in [(2u16, 0u32), (1, SCRATCH + 0x300), (3, 0)] {
        t.write(BUF, &vec![0xa5; 0x200 * 0x6c]);
        t.write(nodes[0], &[0x3c; 0xc0]);
        // a ring of free nodes: 0 <-> 1 <-> 2
        t.set_u32(nodes[0], nodes[1]);
        t.set_u32(nodes[0] + 4, nodes[2]);
        t.set_u32(nodes[1], nodes[2]);
        t.set_u32(nodes[1] + 4, nodes[0]);
        t.set_u32(nodes[2], nodes[0]);
        t.set_u32(nodes[2] + 4, nodes[1]);
        t.set_u32(free, nodes[0]);
        t.set_u16(nfree, n);
        t.set_u32(head, h);
        t.set_u32(tail, SCRATCH + 0x300);
        t.check(at, Call::this(BUF), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0072::pool_setup(g, img, fr.at, fr.ecx))
        });
    }
    let take = t.at(0x26f0);
    let cap = t.img.word(take + 0x0a);
    let rover = t.img.word(take + 0x02);
    for (c, r, busy) in [
        (0x200u32, 0u32, 0u32),
        (4, 2, 3),
        (4, 3, 4),
        (1, 0, 0),
        (0, 0, 0),
    ] {
        t.set_u32(cap, c);
        t.set_u32(rover, r);
        for i in 0..8u32 {
            t.set_u32(BUF + 0x68 + 0x6c * i, u32::from(i < busy));
        }
        t.check(take, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0072::pool_take(g, img, fr.at))
        });
    }
}

#[test]
fn place() {
    let Some(mut t) = twin(N) else { return };
    filler(&mut t, 235, 1, 16);
    recs(&mut t, &[(328, 3), (329, 3)]);
    rec(&mut t, 236, 6, |m, a| {
        for i in 0..16 {
            m.w32(a[2] + 4 * i, f(1.25 + i as f32 * 0.5));
        }
        0x236
    });
    keeper(&mut t, 231, 10, 0, 16, 0x100);
    rec(&mut t, 229, 13, |m, a| {
        dump(m, 0x140, a[0], 4);
        dump(m, 0x150, a[1], 16);
        for i in 0..4 {
            m.w32(a[0] + 4 * i, f(0.75 - i as f32));
        }
        0x229
    });
    let obj = SCRATCH + 0x2000;
    for s in [2.0f64, -0.5, 0.0] {
        t.stub_st0(277, move |_, _| s);
        t.write(obj + 0xd0, &[0x5a; 0x10]);
        t.check_entry(t.at(0x2af0), Call::default(), &[obj], Eax::Checked);
    }
}
