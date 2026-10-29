//! `magic_0004.dll` against its Rust port: the node set-up of the
//! `magic_0003` family (`ffx_magic::m0003::node_init`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0003};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 4;

#[test]
fn node_init_links_into_the_parent() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let f = t.at(0x30f0);
    let args = SCRATCH;
    let node = SCRATCH + 0x100;
    let parent = SCRATCH + 0x200;
    let last = SCRATCH + 0x300;
    // (parent, the parent's last child: itself = no children)
    for (p, l) in [(0, 0), (parent, parent), (parent, last)] {
        t.write(node, &[0xc3; 0x48]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(args, node);
        t.set_u32(args + 4, p);
        t.set_u32(args + 8, 0x0bad_cafe);
        if p != 0 {
            t.set_u32(parent + 0x10, l);
            t.set_u16(parent + 0x18, 0xffff);
        }
        t.check(
            f,
            Call {
                eax: 0,
                ecx: args,
                edx: 0,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0003::node_init(g, img, fr.at, fr.ecx)),
        );
    }
}
