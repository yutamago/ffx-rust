//! `magic_0071.dll` against its Rust port: the node set-up that stores
//! its handler table last (`ffx_magic::m0070::node_init_big_table`).
//! Needs `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0070};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 71;

#[test]
fn node_init_big_table_links_and_sets_the_table() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let args = SCRATCH;
    let node = SCRATCH + 0x100;
    let parent = SCRATCH + 0x400;
    let last = SCRATCH + 0x600;
    for (p, l) in [(0, 0), (parent, parent), (parent, last)] {
        t.write(node, &[0xc3; 0x150]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(parent + 0x10, l);
        for (i, v) in (0u32..).zip([node, p, 0xa, 0xb, 0xc]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(
            t.at(0x2fe0),
            Call {
                eax: 0,
                ecx: args,
                edx: 0,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::node_init_big_table(g, img, fr.at, fr.ecx)),
        );
    }
}
