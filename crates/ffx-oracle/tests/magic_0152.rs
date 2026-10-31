//! `magic_0152.dll` against its Rust port (`ffx_magic::m0152`): the
//! effect node's children with slots 287 and 283 (internal). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0152};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 152;

/// 0x10003720: built under no parent and under one with a last child.
#[test]
fn build_287() {
    let Some(mut t) = twin(N) else { return };
    let f = t.at(0x3720);
    let args = SCRATCH;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x400;
    let last = SCRATCH + 0x600;
    rec(&mut t, 287, 3, |_, a| 0x0287_0000 ^ a[2]);
    rec(&mut t, 283, 2, |_, a| a[0] ^ a[1]);
    for (p, l) in [(0, node), (parent, last)] {
        t.write(node, &[0xc3; 0x200]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(parent + 0x10, l);
        t.set_u32(node + 0x10, l);
        for (i, v) in (0u32..).zip([node, p, 0xd, 0xa, 0xb, 0xc, 0x61, 0x62, 0x63, 0x64]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(f, Call::this(args), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0152::build_287(g, img, fr.at, fr.ecx))
        });
    }
}
