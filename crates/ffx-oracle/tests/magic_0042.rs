//! `magic_0042.dll` against its Rust port (`ffx_magic::m0042`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0042};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 42;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

#[test]
fn node_init_links_into_the_parent() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x34d0);
    let args = SCRATCH;
    let node = SCRATCH + 0x100;
    let parent = SCRATCH + 0x200;
    let last = SCRATCH + 0x300;
    for (p, l) in [(0, 0), (parent, parent), (parent, last)] {
        t.write(node, &[0xc3; 0x48]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(args, node);
        t.set_u32(args + 4, p);
        t.set_u32(args + 8, 0x0bad_cafe);
        t.set_u32(args + 0xc, 0x0bad_f00d);
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
            |g, img, fr| Ret::eax(m0042::node_init(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn release_and_set_value() {
    let Some(mut t) = twin() else { return };
    for (k, (slot, n)) in (0u32..).zip([(670u32, 2usize), (30, 1), (36, 3), (228, 2)]) {
        let row = SCRATCH + 0x1000 + 0x100 * k;
        t.stub(slot, move |m, a| {
            let c = m.r32(row);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 + 0x10 * (c % 4) + 4 * i, *w);
            }
            m.w32(row, c + 1);
            0x0700 + slot
        });
    }
    t.check(
        t.at(0xbe50),
        Call {
            eax: 0,
            ecx: 0x00ab_cdef,
            edx: 0,
        },
        &[],
        Eax::Checked,
        |g, img, fr| Ret::eax(m0042::release(g, img, fr.at, fr.ecx)),
    );
    let obj = SCRATCH + 0x100;
    for bits in [0u16, 1, 0x8001, 0xfffe, 0x0300] {
        t.write(obj, &[0x77; 0x100]);
        t.set_u16(obj + 0x90, bits);
        t.check(
            t.at(0x8400),
            Call {
                eax: 0,
                ecx: obj,
                edx: 0x00ed_0042,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0042::set_value(g, img, fr.at, fr.ecx, fr.edx)),
        );
    }
}

#[test]
fn unlink_closes_the_gap() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x5cd0);
    let lock = t.u32(t.img.word(f + 0x45));
    t.set_u32(lock, 0x0bad);
    t.stub(779, move |m, a| {
        m.w32(SCRATCH + 0x800, a[0]);
        m.w32(SCRATCH + 0x804, a[1]);
        m.w32(SCRATCH + 0x808, m.r32(lock));
        0x0779
    });
    let this = SCRATCH;
    let node = SCRATCH + 0x100;
    let parent = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x300, SCRATCH + 0x400);
    // (parent, prev, next): no parent, an only child, the first, the
    // last, a middle child
    for (p, a, b) in [
        (0, prev, next),
        (parent, parent, parent),
        (parent, parent, next),
        (parent, prev, parent),
        (parent, prev, next),
    ] {
        t.write(node, &[0x44; 0x30]);
        t.write(parent, &[0x55; 0x20]);
        t.write(prev, &[0x66; 0x10]);
        t.write(next, &[0x77; 0x10]);
        t.set_u32(this, node);
        t.set_u32(node + 0x14, p);
        t.set_u32(node + 4, a);
        t.set_u32(node + 8, b);
        t.set_u32(node + 0x20, 0x00ee_0020);
        t.set_u16(parent + 0x18, 0);
        t.check(
            f,
            Call {
                eax: 0,
                ecx: this,
                edx: 0,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0042::unlink(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn the_helpers_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let lists: [(&str, &[(u16, u32)]); 4] = [
        (
            "m0042 unlink",
            &[
                (42, 0x5cd0),
                (152, 0x5980),
                (178, 0x3e30),
                (387, 0x59c0),
                (420, 0x3e30),
                (533, 0x5990),
            ],
        ),
        (
            "m0042 node init",
            &[
                (42, 0x34d0),
                (152, 0x3290),
                (173, 0x2370),
                (186, 0x25e0),
                (262, 0x23e0),
                (263, 0x23e0),
                (387, 0x2620),
                (533, 0x3290),
                (601, 0x31d0),
            ],
        ),
        (
            "m0042 release",
            &[
                (42, 0xbe50),
                (70, 0xe3b0),
                (152, 0xd5e0),
                (533, 0xd010),
                (601, 0xb240),
            ],
        ),
        (
            "m0042 set value",
            &[
                (42, 0x8400),
                (70, 0x9520),
                (155, 0x8de0),
                (164, 0x8d70),
                (201, 0x6c00),
                (273, 0x7320),
                (388, 0x66f0),
                (570, 0x93a0),
                (609, 0x8e90),
                (613, 0x8c80),
                (645, 0x6750),
            ],
        ),
    ];
    for (port, carriers) in lists {
        for &(n, rva) in carriers {
            let name = format!("magic_{n:04}.dll");
            let file = std::fs::read(dir.join(&name)).unwrap();
            let img = ffx_magic::Image::new(&name, &file, 0x1000_0000).unwrap();
            assert_eq!(
                ffx_magic::covered(&img, 0x1000_0000 + rva),
                Some(port),
                "{name}"
            );
        }
    }
}
