//! `magic_0032.dll` against its Rust port: the family's internal helpers
//! (`ffx_magic::m0032`). Needs `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0032};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 32;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

#[test]
fn pop_node_unlinks_the_head() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x2200);
    let head = t.img.word(f + 0x13);
    let count = t.img.word(f + 0x04);
    // a ring of three nodes, filled with junk the reset must clear
    let node = |i: u32| SCRATCH + 0x100 + 0x40 * i;
    for (cnt, at) in [(3u16, 0u32), (2, 1), (1, 2), (0, 0), (0x8001, 1)] {
        for i in 0..3 {
            t.write(node(i), &[0xa5; 0x20]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(head, node(at));
        t.set_u16(count, cnt);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0032::pop_node(g, img, fr.at))
        });
    }
    // a node alone in its ring
    t.set_u32(node(0), node(0));
    t.set_u32(node(0) + 4, node(0));
    t.set_u32(head, node(0));
    t.set_u16(count, 1);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0032::pop_node(g, img, fr.at))
    });
}

#[test]
fn reset_state_writes_the_block() {
    let Some(mut t) = twin() else { return };
    let this = SCRATCH + 0x1000;
    t.write(this + 0xb0, &[0x5a; 0x90]);
    let f = t.at(0x1a10);
    // eax is the 0x8000 the u16 store leaves (no caller reads it)
    t.check(
        f,
        Call {
            eax: 0,
            ecx: this,
            edx: 0,
            ..Call::default()
        },
        &[0x1111, 0x1000_2130],
        Eax::Junk,
        |g, _, fr| {
            let arg = fr.arg(g, 1);
            m0032::reset_state(g, fr.ecx, arg, 0, 0x14);
            Ret::default()
        },
    );
}

#[test]
fn the_helpers_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let cov = |n: u16, rva: u32| {
        let name = format!("magic_{n:04}.dll");
        let file = std::fs::read(dir.join(&name)).unwrap();
        let img = ffx_magic::Image::new(&name, &file, 0x1000_0000).unwrap();
        ffx_magic::covered(&img, 0x1000_0000 + rva)
    };
    for (n, rva) in [
        (32, 0x2200),
        (40, 0x2200),
        (46, 0x2200),
        (48, 0x2200),
        (50, 0x2200),
        (54, 0x2230),
        (197, 0x2190),
        (217, 0x2200),
        (219, 0x2200),
        (231, 0x2300),
        (338, 0x2660),
        (365, 0x1d50),
        (470, 0x23c0),
        (554, 0x23c0),
        (590, 0x23c0),
    ] {
        assert_eq!(cov(n, rva), Some("m0032 free-list pop"), "{n}");
    }
    for (n, rva) in [
        (32, 0x1a10),
        (40, 0x1a10),
        (48, 0x1a10),
        (50, 0x1a10),
        (84, 0x25a0),
        (85, 0x1b80),
        (231, 0x1b10),
    ] {
        assert_eq!(cov(n, rva), Some("m0032 state reset (0x14)"), "{n}");
    }
    assert_eq!(cov(46, 0x1a10), Some("m0032 state reset (0)"));
}
