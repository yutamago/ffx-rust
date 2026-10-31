//! `magic_0042.dll` against its Rust port (`ffx_magic::m0042`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

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
                ..Call::default()
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
            ..Call::default()
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
                ..Call::default()
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
                ..Call::default()
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

#[test]
fn node_init_big_and_time_left() {
    let Some(mut t) = twin() else { return };
    let args = SCRATCH;
    let node = SCRATCH + 0x100;
    let parent = SCRATCH + 0x400;
    let last = SCRATCH + 0x600;
    for (p, l) in [(0, 0), (parent, parent), (parent, last)] {
        t.write(node, &[0xc3; 0x150]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(parent + 0x10, l);
        for (i, v) in (0u32..).zip([node, p, 0xd, 0xa, 0xb, 0xc]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(
            t.at(0x35c0),
            Call {
                eax: 0,
                ecx: args,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, _, fr| Ret::eax(m0042::node_init_big(g, fr.ecx)),
        );
    }
    // time_left: slot 333 answers in ST0, slot 601 through the pointer
    t.stub(333, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        0x0333
    });
    t.stub(601, |m, a| {
        m.w32(SCRATCH + 0x904, a[0]);
        let v = m.r32(SCRATCH + 0x908);
        if v != 0 {
            m.w32(a[1], v);
        }
        0x0601
    });
    let this = SCRATCH + 0x700;
    let obj = SCRATCH + 0x800;
    let out = SCRATCH + 0xa00;
    t.set_u32(this, obj);
    t.set_u32(this + 4, out);
    t.set_u32(obj + 0x50, 0x0050_0050);
    t.set_u32(obj + 0x54, 0x0054_0054);
    for (bit, time) in [
        (1u8, 0.5f64),
        (1, -3.0e9),
        (1, f64::NAN),
        (0, 12.25),
        (0, 1.0e20),
        (0, 0.0),
        // (K0 - t) * K1 / K2 lands just on 153427: x * (K1 / K2) would not
        (0, f64::from(f32::from_bits(0xc11f_f4b2))),
    ] {
        t.stub_st0(333, move |_, _| time);
        #[allow(clippy::cast_possible_truncation)]
        let bits = if time == 0.0 {
            0
        } else {
            (time as f32).to_bits()
        };
        t.set_u32(SCRATCH + 0x908, bits);
        t.set_u8(obj + 0x90, bit);
        t.write(out, &[0x77; 12]);
        t.check(
            t.at(0x3de0),
            Call {
                eax: 0,
                ecx: this,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0042::time_left(g, img, fr.at, fr.ecx, fr.esp)),
        );
    }
    for (port, carriers) in [
        (
            "m0042 node init (big)",
            &[(42u16, 0x35c0u32), (152, 0x3380), (533, 0x3380)][..],
        ),
        (
            "m0042 time left",
            &[(42, 0x3de0), (70, 0x4850), (570, 0x4850), (597, 0x44b0)][..],
        ),
    ] {
        let dir = std::path::PathBuf::from(std::env::var_os("FFX_GAME_DIR").unwrap())
            .join("magicFiles/FFX");
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

/// The group visit with the pool step (0x10001000): the walk over two
/// children (their methods are slots 950/951), the mode already equal,
/// the owner's byte `+0x1c` clear and set.
#[test]
fn visit_pool() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x1000);
    let this = SCRATCH + 0x100;
    let owner = SCRATCH + 0x200;
    let kids = [SCRATCH + 0x300, SCRATCH + 0x380];
    let vt = SCRATCH + 0x500;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 950);
    let pool = t.img.word(f + 0x5e);
    for slot in [950u32, 245, 581] {
        let row = SCRATCH + 0x800 + 0x40 * (slot - 240);
        t.stub(slot, move |m, a| {
            let k = m.r32(row);
            m.w32(row, k + 1);
            for (i, w) in (0u32..).zip(&a[..if slot == 581 { 5 } else { 1 }]) {
                m.w32(row + 4 + 4 * i, *w ^ k);
            }
            slot
        });
    }
    for (mode, flag) in [(1u8, 0u8), (1, 1), (2, 1), (2, 0)] {
        t.set_u32(this, owner);
        t.set_u8(this + 4, 0x55);
        t.set_u8(this + 5, mode);
        t.set_u8(owner + 0x1a, 2);
        t.set_u8(owner + 0x1c, flag);
        t.set_u32(owner + 0x10, kids[0]);
        t.set_u32(kids[0], vt);
        t.set_u32(kids[0] + 8, kids[1]);
        t.set_u32(kids[1], vt);
        t.set_u32(kids[1] + 8, owner);
        t.set_u32(pool, SCRATCH + 0x4000);
        let c = Call {
            eax: 0x1234_5678,
            ecx: this,
            edx: 0,
            ..Call::default()
        };
        t.check_entry(f, c, &[this], Eax::Checked);
    }
}

/// The node with its three children (0x10003960, internal): under no
/// parent and under one with a child already.
#[test]
fn build() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x3960);
    let args = SCRATCH;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x400;
    let last = SCRATCH + 0x600;
    t.stub(281, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        0x0281
    });
    t.stub(869, |m, a| {
        m.w32(SCRATCH + 0x904, a[0]);
        0x0869
    });
    for (p, l) in [(0, 0), (parent, last)] {
        t.write(node, &[0xc3; 0x200]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(last, &[0x5a; 0x10]);
        t.set_u32(parent + 0x10, l);
        for (i, v) in (0u32..).zip([node, p, 0xd, 0xa, 0xb, 0xc, 0x61, 0x62, 0x63, 0x64]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(
            f,
            Call {
                eax: 0,
                ecx: args,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0042::build(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The start-up (0x10009040): into an empty and a filled root list (and
/// the second list empty and filled), the ring's tail anywhere.
#[test]
fn start_up() {
    let Some(mut t) = twin() else { return };
    let at = t.at(0x9040);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (slot, k) in [
        (672u32, 0u32),
        (560, 1),
        (281, 2),
        (869, 3),
        (331, 4),
        (353, 5),
        (271, 6),
    ] {
        t.stub(slot, move |m, a| {
            let n = m.r32(SCRATCH + 0x3000 + 0x40 * k + 0x3c).wrapping_add(1);
            m.w32(SCRATCH + 0x3000 + 0x40 * k + 0x3c, n);
            m.w32(SCRATCH + 0x3000 + 0x40 * k + 4 * (n % 8), a[0] ^ n);
            0x0100_0000 * (k + 1)
        });
    }
    t.stub(557, |_, _| 0x0000_00ff);
    t.stub_st0(790, |_, _| 0.25);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 3 + i);
        }
        0x558
    });
    let tail = SCRATCH + 0x500;
    let other = SCRATCH + 0x600;
    // every word the start-up stores (the conditional ones too) holds junk
    for op in [
        0x09, 0x0f, 0x19, 0x23, 0x2d, 0x43, 0x4d, 0x57, 0x62, 0x6a, 0x74, 0x7e, 0x88, 0x91, 0x9e,
        0xbe, 0xc8, 0xd2, 0xdc, 0xe7, 0xef, 0xf9, 0x103, 0x10d, 0x117, 0x129, 0x135, 0x146, 0x150,
        0x15a, 0x164, 0x16e, 0x176, 0x17e, 0x188, 0x192, 0x19c, 0x1a6, 0x1b8, 0x1c4, 0x1db, 0x1e5,
        0x206, 0x211, 0x219, 0x223, 0x22d, 0x237, 0x240, 0x24d, 0x26c, 0x276, 0x295, 0x2ef, 0x2f9,
        0x304, 0x30c, 0x316, 0x320, 0x32a, 0x333, 0x340, 0x35d, 0x367, 0x371, 0x3af, 0x3be, 0x3c8,
        0x3d2,
    ] {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    for (ring, root, second) in [
        (tail, false, false),
        (tail, true, true),
        (tail, false, true),
    ] {
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        t.set_u32(w(&t, 0x04), ring);
        let r = w(&t, 0x96);
        t.set_u32(w(&t, 0x3d), if root { other } else { r });
        let r2 = w(&t, 0x338);
        t.set_u32(w(&t, 0x2e9), if second { other + 0x10 } else { r2 });
        t.set_u16(w(&t, 0xb1), 7);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
