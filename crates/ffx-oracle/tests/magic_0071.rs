//! `magic_0071.dll` against its Rust port: the node set-up that stores
//! its handler table last (`ffx_magic::m0070::node_init_big_table`).
//! Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

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
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::node_init_big_table(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The own node set-ups (internal): under no parent, an empty parent
/// and one with a child; the root's list empty and not.
#[test]
fn own_node_inits() {
    use ffx_magic::m0071;
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let (args, node, parent, last) = (SCRATCH, SCRATCH + 0x100, SCRATCH + 0x400, SCRATCH + 0x600);
    t.stub(325, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        m.w32(SCRATCH + 0x904, a[1]);
        0x325
    });
    for (p, l) in [(0, 0), (parent, parent), (parent, last)] {
        t.write(node, &[0xc3; 0x80]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(parent + 0x10, l);
        for (i, v) in (0u32..).zip([node, p, 1, 2, 0x0bad_cafe]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(
            t.at(0x13a0),
            Call::this(args),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0071::node_init_vec(g, img, fr.at, fr.ecx)),
        );
    }
    let f = t.at(0x7670);
    let root = t.img.word(f + 0x15);
    for l in [root, last] {
        t.write(node, &[0xc3; 0x80]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(t.img.word(f + 0x21), l);
        t.check(f, Call::this(node), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0071::root_node(g, img, fr.at, fr.ecx))
        });
        t.set_u32(t.img.word(f + 0x21), l);
        t.check(
            t.at(0x7570),
            Call::default(),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0071::static_node(g, img, fr.at)),
        );
    }
}

/// The spinning node's visit: the mode already equal; a walk over two
/// children (methods: slot 950) and the inner node's two children; the
/// angle below and at its limit.
#[test]
fn spin_visit() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let f = t.at(0x3f00);
    let this = SCRATCH + 0x100;
    let owner = SCRATCH + 0x200;
    let inner = SCRATCH + 0x300;
    let kids = [SCRATCH + 0x400, SCRATCH + 0x480];
    let ikids = [SCRATCH + 0x500, SCRATCH + 0x580];
    let vt = SCRATCH + 0x700;
    let table = SCRATCH + 0x1_0000;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 950);
    t.stub(950, |m, a| {
        let k = m.r32(SCRATCH + 0x800);
        m.w32(SCRATCH + 0x800, k + 1);
        m.w32(SCRATCH + 0x804 + 4 * (k % 16), a[0]);
        0x950
    });
    for i in 0..0x1000u32 {
        #[allow(clippy::cast_precision_loss)]
        t.set_u32(table + 4 * i, (i as f32 * 0.001 - 2.0).to_bits());
    }
    t.set_u32(t.img.word(f + 0xe0), table);
    let limit = f32::from_bits(t.u32(t.img.word(f + 0xc1)));
    for (mode, angle) in [
        (2u8, 0.25f32),
        (1, 0.5),
        (1, -3.0),
        (1, limit.sqrt()),
        (1, 1e5),
    ] {
        t.set_u32(this, owner);
        t.set_u8(this + 4, 0x55);
        t.set_u8(this + 5, mode);
        t.set_u8(owner + 0x1a, 2);
        t.set_u32(owner + 0x10, kids[0]);
        t.set_u32(owner + 0x2c, inner);
        t.set_u8(inner + 0x1a, 3);
        t.set_u32(inner + 0x10, ikids[0]);
        for (list, end) in [(kids, owner), (ikids, inner)] {
            t.set_u32(list[0], vt);
            t.set_u32(list[0] + 8, list[1]);
            t.set_u32(list[1], vt);
            t.set_u32(list[1] + 8, end);
        }
        t.set_u32(owner + 0x30, angle.to_bits());
        t.set_u32(owner + 0x34, 0.125f32.to_bits());
        t.set_u32(owner + 0x38, 0.9f32.to_bits());
        for k in 0..12u32 {
            #[allow(clippy::cast_precision_loss)]
            t.set_u32(inner + 0x1c + 4 * k, (1.5 + k as f32).to_bits());
        }
        let c = Call {
            eax: 0x1234_5678,
            ecx: this,
            ..Call::default()
        };
        t.check_entry(f, c, &[this], Eax::Checked);
    }
}
