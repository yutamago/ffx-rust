//! `magic_0070.dll` against its Rust port (`ffx_magic::m0070`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use ffx_magic::{Ret, m0070};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 70;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

const NODE: u32 = SCRATCH + 0x100;
const PARENT: u32 = SCRATCH + 0x400;
const LAST: u32 = SCRATCH + 0x600;

/// The node and its parent filled with junk; the parent's last child
/// `last` (itself: no children).
fn nodes(t: &mut Twin, last: u32) {
    t.write(NODE, &[0xc3; 0x150]);
    t.write(PARENT, &[0x3c; 0x20]);
    t.write(LAST, &[0x5a; 0x10]);
    t.set_u32(PARENT + 0x10, last);
    t.set_u16(PARENT + 0x18, 0xffff);
}

#[test]
fn node_inits() {
    let Some(mut t) = twin() else { return };
    let args = SCRATCH;
    for (p, l) in [(0, 0), (PARENT, PARENT), (PARENT, LAST)] {
        nodes(&mut t, l);
        for (i, v) in (0u32..).zip([NODE, p, 0xa, 0xb, 0xc]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(
            t.at(0x3150),
            Call {
                eax: 0,
                ecx: args,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, _, fr| Ret::eax(m0070::node_init_big(g, fr.ecx)),
        );
        nodes(&mut t, l);
        t.check(
            t.at(0x99f0),
            Call {
                eax: 0,
                ecx: NODE,
                edx: p,
                ..Call::default()
            },
            &[0x00aa_0001],
            Eax::Checked,
            |g, img, fr| {
                let a = fr.arg(g, 0);
                Ret::eax(m0070::node_init_small(g, img, fr.at, fr.ecx, fr.edx, a))
            },
        );
    }
    // the root node: no children yet, then one
    let f = t.at(0x98e0);
    let root = t.img.word(f + 0x15);
    t.write(root, &[0; 0x20]);
    t.set_u32(root + 0x10, root);
    for _ in 0..2 {
        t.write(NODE, &[0xc3; 0x60]);
        t.check(
            f,
            Call {
                eax: 0,
                ecx: NODE,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::node_init_root(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn free_part_unless_kept() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x9a80);
    let envp = t.img.word(f + 0x0a);
    t.set_u32(envp, SCRATCH + 0x800);
    t.stub(713, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        m.w32(SCRATCH + 0x904, m.r32(SCRATCH + 0x800));
        0x0713
    });
    let part = SCRATCH + 0x100;
    for flags in [0u8, 4, 0xfb, 0xff] {
        t.write(part, &[0x11; 0x100]);
        t.set_u8(part + 0xe8, flags);
        t.check(
            f,
            Call {
                eax: 0x00ea_0000,
                ecx: part,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::free_part(g, img, fr.at, fr.ecx, fr.eax)),
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
            "m0070 node init (big)",
            &[
                (70, 0x3150),
                (150, 0x25b0),
                (387, 0x21a0),
                (570, 0x3150),
                (577, 0x2c50),
                (597, 0x30b0),
                (603, 0x25b0),
                (609, 0x25e0),
                (613, 0x25c0),
            ],
        ),
        (
            "m0070 node init (root)",
            &[
                (70, 0x98e0),
                (71, 0x7600),
                (106, 0x6b70),
                (388, 0x67a0),
                (445, 0x6a90),
                (570, 0x95d0),
                (577, 0xc6e0),
                (578, 0x80a0),
            ],
        ),
        (
            "m0070 part free",
            &[
                (70, 0x9a80),
                (155, 0x9270),
                (164, 0x92d0),
                (186, 0x76d0),
                (570, 0x9770),
                (577, 0x9070),
                (609, 0x9360),
                (613, 0x92a0),
            ],
        ),
        (
            "m0070 node init (small)",
            &[
                (70, 0x99f0),
                (155, 0x91e0),
                (164, 0x9240),
                (570, 0x96e0),
                (577, 0x8fe0),
                (597, 0x8f80),
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

/// Slot stubs that log their words in a row each and answer `eax` (and
/// ST0 for the float slots).
fn loggers(t: &mut Twin, slots: &[(u32, usize)]) {
    for (k, &(slot, n)) in (0u32..).zip(slots) {
        let row = SCRATCH + 0x2_0000 + 0x100 * k;
        t.stub(slot, move |m, a| {
            let c = m.r32(row);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 + 0x10 * (c % 8) + 4 * i, *w);
            }
            m.w32(row, c + 1);
            m.r32(SCRATCH + 0x3_0000 + 4 * slot)
        });
    }
}

#[test]
fn model_state_reads_the_model() {
    let Some(mut t) = twin() else { return };
    loggers(&mut t, &[(15, 1), (16, 2), (17, 3), (277, 1), (290, 2)]);
    for (slot, v) in [(15u32, 1.25f64), (16, -0.0), (17, 1.0e30), (277, 0.3)] {
        t.stub_st0(slot, move |_, _| v);
    }
    let this = SCRATCH;
    let obj = SCRATCH + 0x100;
    let out = SCRATCH + 0x400;
    t.set_u32(this, obj);
    t.set_u32(this + 4, out);
    t.set_u32(obj + 0x50, 0x0050_0050);
    t.set_u32(obj + 0x54, 0x0054_0054);
    for bits in [0u16, 1, 0xfffe, 0xffff] {
        t.write(out, &[0x99; 0x10]);
        t.set_u16(obj + 0x90, bits);
        t.check(
            t.at(0x46e0),
            Call {
                eax: 0,
                ecx: this,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::model_state(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn show_and_hide() {
    let Some(mut t) = twin() else { return };
    loggers(&mut t, &[(276, 2), (219, 2), (548, 1)]);
    let this = SCRATCH;
    let obj = SCRATCH + 0x100;
    t.set_u32(this, obj);
    t.set_u32(obj + 0x50, 0x0050_0050);
    t.set_u32(obj + 0x54, 0x0054_0054);
    // slot 219's answers: (part 0, part 1)
    for (on, bits, parts) in [
        (1u8, 0u16, 0u32),
        (1, 0, 0x0a0a),
        (0, 0x0800, 0x0a0a),
        (2, 1, 0x0a0a),
        (0, 0xffff, 0),
    ] {
        t.set_u8(this + 4, on);
        t.set_u16(obj + 0x90, bits);
        t.set_u32(SCRATCH + 0x3_0000 + 4 * 219, parts);
        t.check(
            t.at(0x4790),
            Call {
                eax: 0,
                ecx: this,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::show(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn the_state_helpers_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let lists: [(&str, &[(u16, u32)]); 3] = [
        (
            "m0070 model state",
            &[
                (42, 0x3d30),
                (70, 0x46e0),
                (478, 0x2630),
                (514, 0x2630),
                (570, 0x46e0),
                (573, 0x2460),
                (577, 0x3df0),
                (585, 0x23f0),
                (597, 0x4400),
                (645, 0x23f0),
                (649, 0x23f0),
            ],
        ),
        (
            "m0070 show",
            &[
                (70, 0x4790),
                (201, 0x24f0),
                (203, 0x2630),
                (272, 0x23f0),
                (273, 0x2970),
                (570, 0x4790),
                (573, 0x2510),
            ],
        ),
        (
            "m0070 node init (big, table)",
            &[
                (71, 0x2fe0),
                (153, 0x3680),
                (186, 0x2560),
                (207, 0x25a0),
                (262, 0x2360),
                (263, 0x2360),
                (512, 0x25a0),
                (573, 0x2360),
                (578, 0x2360),
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
fn ring_take_wraps() {
    let Some(mut t) = twin() else { return };
    let req = SCRATCH;
    let ring = SCRATCH + 0x100;
    // (count, capacity, head, used, start)
    for (n, cap, head, used, start) in [
        (4u16, 16u16, 0u16, 0u16, 0u16),
        (4, 16, 12, 3, 0),
        (4, 16, 14, 16, 9),
        (17, 16, 0, 0, 0),
        (16, 16, 5, 15, 5),
        (3, 16, 2, 14, 10),
        (0, 0, 0, 0, 0),
        (0xffff, 0xffff, 0xfff0, 0xfff0, 0x20),
        (6, 16, 10, 16, 12),
    ] {
        t.write(req, &[0x5a; 0x20]);
        t.write(ring, &[0xa5; 0x40]);
        t.set_u32(req, ring);
        t.set_u16(req + 4, n);
        t.set_u16(ring + 0x1c, cap);
        t.set_u16(ring + 0x1e, 0x24);
        t.set_u32(ring + 0x20, 0x0080_0000);
        t.set_u16(ring + 0x30, head);
        t.set_u16(ring + 0x32, used);
        t.set_u16(ring + 0x34, start);
        t.check(
            t.at(0x1740),
            Call {
                eax: 0,
                ecx: req,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, _, fr| Ret::eax(m0070::ring_take(g, fr.ecx)),
        );
    }
}

#[test]
fn ring_take_binds_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    for (n, rva) in [
        (70u16, 0x1740u32),
        (71, 0x12a0),
        (153, 0x1800),
        (342, 0x1830),
        (389, 0x1830),
        (570, 0x1740),
        (597, 0x1740),
    ] {
        let name = format!("magic_{n:04}.dll");
        let file = std::fs::read(dir.join(&name)).unwrap();
        let img = ffx_magic::Image::new(&name, &file, 0x1000_0000).unwrap();
        assert_eq!(
            ffx_magic::covered(&img, 0x1000_0000 + rva),
            Some("m0070 ring take"),
            "{name}"
        );
    }
}

#[test]
fn small_model_calls() {
    let Some(mut t) = twin() else { return };
    loggers(
        &mut t,
        &[(255, 2), (531, 2), (262, 4), (12, 5), (257, 2), (296, 2)],
    );
    t.stub_st0(12, |_, _| -7.5);
    for slot in [255u32, 531, 12, 257, 296] {
        t.set_u32(SCRATCH + 0x3_0000 + 4 * slot, 0x0100_0000 + slot);
    }
    let obj = SCRATCH + 0x100;
    t.set_u32(obj + 0x50, 0x0050_0050);
    t.set_u32(obj + 0x54, 0x0054_0054);
    for bits in [0u8, 1, 0xfe] {
        t.set_u8(obj + 0x90, bits);
        for (rva, slot) in [(0x9500u32, 255u32), (0x9590, 531)] {
            t.check(
                t.at(rva),
                Call {
                    eax: 0x00ea_00ea,
                    ecx: obj,
                    edx: 0x00ed_00ed,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(m0070::part_call(
                        g,
                        img,
                        fr.at,
                        slot,
                        [fr.eax, fr.ecx, fr.edx],
                    ))
                },
            );
        }
        t.check(
            t.at(0x9620),
            Call {
                eax: 0,
                ecx: obj,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0070::hide_model(g, img, fr.at, fr.ecx)),
        );
    }
    let f = t.at(0x95b0);
    t.set_u32(t.img.word(f + 0x17), 0x00ad_e100);
    let v = SCRATCH + 0x200;
    t.write(v, &[0x3f; 12]);
    t.check(
        f,
        Call {
            eax: 0,
            ecx: 0,
            edx: v,
            ..Call::default()
        },
        &[],
        Eax::Checked,
        |g, img, fr| Ret::eax(m0070::model_position(g, img, fr.at, fr.edx)),
    );
}

#[test]
fn small_model_calls_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let lists: [(&str, &[(u16, u32)]); 4] = [
        (
            "m0070 part call (255)",
            &[(70, 0x9500), (570, 0x9380), (597, 0x8e60)],
        ),
        (
            "m0070 part call (531)",
            &[(70, 0x9590), (570, 0x9410), (597, 0x8ef0)],
        ),
        (
            "m0070 model position",
            &[(70, 0x95b0), (343, 0x69a0), (570, 0x9430)],
        ),
        (
            "m0070 hide model",
            &[(70, 0x9620), (570, 0x94a0), (597, 0x8f10)],
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

/// The second round's own node set-ups (`node_init_scaled`, `root_child`,
/// `static_child`) under no parent, an empty parent and one with a child.
#[test]
fn own_node_inits() {
    let Some(mut t) = twin() else { return };
    let args = SCRATCH;
    for (p, l) in [(0, 0), (PARENT, PARENT), (PARENT, LAST)] {
        nodes(&mut t, l);
        for (i, v) in (0u32..).zip([NODE, p, 0x0bad_cafe]) {
            t.set_u32(args + 4 * i, v);
        }
        t.check(
            t.at(0x5a50),
            Call::this(args),
            &[],
            Eax::Checked,
            |g, _, fr| Ret::eax(m0070::node_init_scaled(g, fr.ecx)),
        );
    }
    // the root's children: none, then one
    let f = t.at(0x9750);
    let root = t.img.word(f + 0x18);
    for last in [root, LAST] {
        t.write(NODE, &[0xc3; 0x150]);
        t.write(LAST, &[0x5a; 0x10]);
        t.set_u32(t.img.word(f + 0x24), last);
        t.check(
            f,
            Call::this(NODE),
            &[0x11, 0x22],
            Eax::Checked,
            |g, img, fr| {
                let (a, b) = (fr.arg(g, 0), fr.arg(g, 1));
                Ret::eax(m0070::root_child(g, img, fr.at, fr.ecx, a, b))
            },
        );
        t.check(
            t.at(0x9800),
            Call::default(),
            &[0x33],
            Eax::Checked,
            |g, img, fr| {
                let a = fr.arg(g, 0);
                Ret::eax(m0070::static_child(g, img, fr.at, a))
            },
        );
    }
}

/// The step at 0x1000d3c0.
#[test]
fn step_d3c0() {
    let Some(mut t) = twin() else { return };
    for (s, n) in [(215u32, 1usize), (276, 3), (381, 4)] {
        let row = SCRATCH + 0x3_0000 + 0x40 * s;
        t.stub(s, move |m, a| {
            let k = m.r32(row);
            m.w32(row, k + 1);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 + 4 * i, *w);
            }
            0x0100 + s
        });
    }
    t.set_u32(t.img.word(t.at(0xd3c0) + 0x07), 0x77);
    t.check_entry(t.at(0xd3c0), Call::default(), &[], Eax::Checked);
}

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The step at 0x1000d410: each wait state and its answer, the node list
/// run (two nodes, handlers slot 950) or held, and the two gated visits
/// (their nodes without children).
#[test]
fn step_d410() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0xd410);
    let cfg = SCRATCH + 0x200;
    for (k, s) in [(0u32, 225u32), (1, 313), (2, 549), (3, 227)] {
        let row = SCRATCH + 0x3_0000 + 0x40 * s;
        t.stub(s, move |m, a| {
            let c = m.r32(row);
            m.w32(row, c + 1);
            m.w32(row + 4, a[0]);
            m.r32(cfg + 4 * k)
        });
    }
    for s in [226u32, 673, 950] {
        let row = SCRATCH + 0x3_0000 + 0x40 * s;
        t.stub(s, move |m, a| {
            let c = m.r32(row);
            m.w32(row, c + 1);
            m.w32(row + 4 + 4 * (c % 8), a.first().copied().unwrap_or(0));
            0
        });
    }
    let state = t.img.word(f + 0x04);
    let hold = t.img.word(f + 0xf8);
    let list = t.img.word(f + 0xe6);
    let end = t.img.word(f + 0x116);
    let gp = t.img.word(f + 0xcf);
    t.set_u32(gp, SCRATCH + 0x300);
    for op in [0x161, 0x17c] {
        let n = t.img.word(f + op);
        t.set_u32(n + 0x10, n);
    }
    let vt = SCRATCH + 0x400;
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 950);
    let (n1, n2) = (SCRATCH + 0x500, SCRATCH + 0x540);
    t.set_u32(n1, vt);
    t.set_u32(n1 + 8, n2);
    t.set_u32(n2, vt);
    t.set_u32(n2 + 8, end);
    for (st, ans, h, first) in [
        (0u32, [1u32, 0, 0, 0], 0u32, n1),
        (0, [0, 0, 0, 1], 0, n1),
        (1, [0, 2, 0, 1], 1, n1),
        (2, [0, 0, 1, 1], 0, end),
        (2, [0, 0, 0, 0], 0, n1),
        (3, [0, 0, 0, 1], 0, end),
        (4, [0, 0, 0, 0], 1, end),
        (4, [0, 0, 0, 0], 1, n1),
        (9, [0, 0, 0, 0], 0, n1),
    ] {
        t.set_u32(state, st);
        for (i, a) in (0u32..).zip(ans) {
            t.set_u32(cfg + 4 * i, a);
        }
        t.set_u32(hold, h);
        t.set_u32(list, first);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// 0x1000cee0: the summon's frame before its phase, with and without
/// children under the root, with the flag set, and with the done bit's
/// slot calls held back (bit clear, halfword set) or made.
#[test]
fn summon_frame_376() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0xcee0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let bind = t.img.rel32_target(f + 0x23);
    t.set_u32(t.img.word(bind + 0x7e), SCRATCH + 0x500);
    for (k, s) in (0u32..).zip([723u32, 724, 230, 253, 214, 376]) {
        let row = SCRATCH + 0x3000 + 0x40 * k;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x3c).wrapping_add(1);
            m.w32(row + 0x3c, c);
            // slot 253 takes no word
            if s != 253 {
                m.w32(row + 4 * (c % 8), a[0]);
            }
            // slot 253 (did the binding fail?) decides the summon flag the
            // frame reads at +0x43: the case's answer
            if s == 253 {
                return m.r32(SCRATCH + 0x37f0);
            }
            0x0300_0000 + s
        });
    }
    // the children's method +0x20: slot 1000, logging its cursor's node
    // and emptying the root's list on the last child
    let first = w(&t, 0x4a);
    let root = w(&t, 0x52);
    t.stub(1000, move |m, a| {
        let k = m.r32(SCRATCH + 0x37fc).wrapping_add(1);
        m.w32(SCRATCH + 0x37fc, k);
        m.w32(SCRATCH + 0x3780 + 4 * (k % 16), m.r32(a[0]));
        if m.r32(m.r32(a[0]) + 8) == root {
            m.w32(first, root);
        }
        0x1000
    });
    let vt = SCRATCH + 0x800;
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 1000);
    t.set_u32(w(&t, 0x18), SCRATCH + 0x400);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    for (phase, flag, kids, a, half) in [
        (3u32, 0u32, 0u32, 1u32, 0u16),
        (4, 0, 2, 1, 0),
        (5, 0, 0, 1, 0),
        (5, 1, 2, 1, 0),
        (5, 1, 0, 1, 1),
        (6, 0, 1, 0, 0),
        (7, 1, 1, 3, 0),
    ] {
        t.set_u32(w(&t, 0x0b), phase);
        t.set_u32(w(&t, 0x43), flag);
        // the binding sets the flag again: slot 253 answers 0 for flag 1
        t.set_u32(SCRATCH + 0x37f0, u32::from(flag == 0));
        // the done bit starts clear except in the phase-6 case
        t.set_u32(w(&t, 0x88), 0x10 | u32::from(phase == 6));
        t.set_u32(w(&t, 0xcb), 0x20 | a);
        t.set_u16(w(&t, 0xdf), half);
        t.set_u32(w(&t, 0xed), 0x00ed_0000);
        t.set_u32(w(&t, 0x109), 0x0109_0000);
        t.set_u8(w(&t, 0x28), 0x41);
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { root } else { kid(i + 1) });
        }
        t.set_u32(first, if kids == 0 { root } else { kid(0) });
        // the bone node holds still (its pass is the frame's); the gated
        // ones move (no children)
        let bone = w(&t, 0x34);
        t.set_u32(bone + 0x10, bone);
        t.set_u8(bone + 0x1a, 0x41);
        for at in [w(&t, 0xa1), w(&t, 0xc1)] {
            t.set_u32(at + 0x10, at);
            t.set_u8(at + 0x1a, 0);
            t.set_u8(at + 0x1b, 0);
        }
        let c = Call {
            eax: 0x00ea_0000 + phase,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
}
