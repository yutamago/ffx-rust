//! `magic_0166.dll` against its Rust port: `DllMain` against every bound
//! port, the record reset and the pool. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use ffx_magic::{Ret, m0032, m0166};
use magic::{Call, Eax, SCRATCH};

const N: u16 = 166;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn reset_record() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let this = SCRATCH + 0x100;
    t.write(this, &[0xa5; 0x140]);
    t.check(
        t.at(0x1ec0),
        Call {
            eax: 1,
            ecx: this,
            edx: 2,
            ..Call::default()
        },
        &[],
        Eax::Checked,
        |g, img, fr| Ret::eax(m0166::reset_record(g, img, fr.at, fr.ecx)),
    );
}

#[test]
fn pool_take() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x2680);
    let count = t.img.word(f + 0x04);
    let head = t.img.word(f + 0x13);
    // a ring of three nodes, 0x20 bytes apart
    let node = |i: u32| SCRATCH + 0x1000 + 0x20 * i;
    for n in [0u16, 1, 2, 3, 0x8000, 0xffff] {
        for i in 0..3 {
            t.write(node(i), &[0xcc; 0x20]);
            t.set_u32(node(i), node((i + 1) % 3));
            t.set_u32(node(i) + 4, node((i + 2) % 3));
        }
        t.set_u16(count, n);
        t.set_u32(head, node(1));
        t.check(
            f,
            Call {
                eax: 0x99,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0032::pop_node(g, img, fr.at)),
        );
    }
}
