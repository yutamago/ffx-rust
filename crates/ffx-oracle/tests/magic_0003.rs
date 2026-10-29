//! `magic_0003.dll` against its Rust port: the family's ring walk with
//! the node clock (`tick`, its handler), and the loading steps
//! (`ffx_magic::m0003`). Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, TRAPS, Twin};

const N: u16 = 3;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// A slot no overlay uses (`Ch_Allocate`), standing in for the nodes' handlers.
const HANDLER_SLOT: u32 = 0;
const CURSOR: u32 = SCRATCH;
const TABLE: u32 = SCRATCH + 0x100;

fn node(i: u32) -> u32 {
    SCRATCH + 0x1000 + 0x100 * i
}

/// A node with `children` (nodes 1..) and the cursor on it.
fn ring(t: &mut Twin, children: u32, pass: u8, cursor_pass: u8) {
    t.set_u32(TABLE + 0x1c, TRAPS + 16 * HANDLER_SLOT);
    let root = node(0);
    for i in 0..=children {
        t.write(node(i), &[0x33; 0x40]);
        t.set_u32(node(i), TABLE);
        t.set_u8(node(i) + 0x30, 0x10 + u8::try_from(i).unwrap());
        t.set_u32(node(i) + 0x2c, 1000 + i);
    }
    t.set_u8(root + 0x1a, pass);
    // children list: root +0x10 the first, linked through +0x8, the last
    // pointing back at the root
    let first = if children == 0 { root } else { node(1) };
    t.set_u32(root + 0x10, first);
    for i in 1..=children {
        let next = if i == children { root } else { node(i + 1) };
        t.set_u32(node(i) + 8, next);
    }
    t.set_u32(CURSOR, root);
    t.set_u8(CURSOR + 4, 0x7f);
    t.set_u8(CURSOR + 5, cursor_pass);
}

#[test]
fn tick_handler_walks_and_sets_the_clock() {
    let Some(mut t) = twin() else { return };
    // the handlers record the cursor's node and answer per call
    t.stub(HANDLER_SLOT, |m, a| {
        let n = m.r32(SCRATCH + 0x800);
        m.w32(SCRATCH + 0x804 + 8 * n, a[0]);
        m.w32(SCRATCH + 0x808 + 8 * n, m.r32(a[0]));
        m.w32(SCRATCH + 0x800, n + 1);
        0x1234_5600 + n
    });
    // slot 601: the node's time through the pointer
    t.stub(601, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        let v = m.r32(SCRATCH + 0x904);
        m.w32(a[1], v);
        0
    });
    let f = t.at(0x2b10);
    for (children, pass, cpass, time) in [
        (0u32, 1u8, 1u8, 1.5f32),
        (0, 1, 2, -3.25),
        (2, 1, 2, 1000.0),
        (3, 7, 7, 1.0e12),
        (1, 0, 9, f32::NAN),
        (2, 3, 4, -1.0e-3),
    ] {
        ring(&mut t, children, pass, cpass);
        t.set_u32(SCRATCH + 0x904, time.to_bits());
        t.check_entry(
            f,
            Call {
                eax: 0xaabb_ccdd,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[CURSOR],
            Eax::Checked,
        );
    }
    // the slot writes nothing: the local still holds the cursor (push ecx)
    t.stub(601, |_, _| 0);
    ring(&mut t, 1, 1, 2);
    t.check_entry(f, Call::default(), &[CURSOR], Eax::Checked);
}

#[test]
fn load_steps_run_until_not_ready() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x6ab0);
    let step = t.img.word(f + 0x01);
    let file = t.img.word(f + 0x6a);
    t.set_u32(file, 0x00f1_1e00);
    // each slot answers the word at SCRATCH + 4 * k and logs its call
    for (k, slot) in [(0u32, 225u32), (1, 313), (2, 549), (3, 226), (4, 227)] {
        t.stub(slot, move |m, a| {
            let n = m.r32(SCRATCH + 0x100);
            m.w32(SCRATCH + 0x104 + 12 * n, slot);
            m.w32(SCRATCH + 0x108 + 12 * n, a[0]);
            // slot 226 takes two words, the others one
            if slot == 226 {
                m.w32(SCRATCH + 0x10c + 12 * n, a[1]);
            }
            m.w32(SCRATCH + 0x100, n + 1);
            m.r32(SCRATCH + 4 * k)
        });
    }
    for s in [0u32, 1, 2, 3, 4, 9, 0xffff_ffff] {
        for answers in [
            [0u32, 0, 0, 0, 1],
            [1, 0, 0, 0, 0],
            [0, 1, 0, 0, 2],
            [0, 0, 1, 0, 0xffff_ffff],
            [0, 0, 0, 0, 0],
        ] {
            for (k, v) in (0u32..).zip(answers) {
                t.set_u32(SCRATCH + 4 * k, v);
            }
            t.set_u32(step, s);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Where each port binds: (port, [(dll, rva)]) from the census.
const CARRIERS: &[(&str, &[(u16, u32)])] = &[
    (
        "m0003 tick",
        &[
            (3, 0x2a80),
            (4, 0x2a80),
            (5, 0x2a80),
            (7, 0x2a80),
            (8, 0x2a80),
            (42, 0x44f0),
            (70, 0x4f60),
            (71, 0x3770),
            (76, 0x2a80),
            (77, 0x2a80),
            (94, 0x2cc0),
            (152, 0x41a0),
            (203, 0x2d80),
            (330, 0x2a80),
            (343, 0x2d60),
            (411, 0x2cc0),
            (412, 0x2cc0),
            (425, 0x2cc0),
            (439, 0x2cc0),
            (493, 0x2a80),
            (495, 0x2a80),
            (514, 0x2d70),
            (533, 0x41b0),
            (570, 0x4f60),
            (597, 0x4bc0),
            (618, 0x2cc0),
            (620, 0x2cc0),
            (674, 0x2cc0),
            (676, 0x2cc0),
            (678, 0x2cc0),
            (679, 0x2cc0),
            (682, 0x2cc0),
        ],
    ),
    (
        "m0003 load steps",
        &[
            (3, 0x6ab0),
            (4, 0x6ac0),
            (5, 0x6ac0),
            (6, 0x6bc0),
            (7, 0x6bb0),
            (8, 0x6b90),
            (42, 0xb0b0),
            (76, 0x6b90),
            (77, 0x6b80),
            (89, 0x6f80),
            (90, 0x6eb0),
            (146, 0x7c00),
            (330, 0x6d40),
            (342, 0x7380),
            (398, 0x6f90),
            (399, 0x7090),
            (402, 0x6f90),
            (459, 0x6fa0),
            (621, 0x7390),
            (677, 0x73a0),
            (683, 0x74a0),
            (684, 0x7390),
        ],
    ),
    (
        "m0003 follow handler",
        &[
            (6, 0x1190),
            (7, 0x1190),
            (8, 0x1190),
            (42, 0x1fe0),
            (76, 0x1190),
            (77, 0x1190),
            (179, 0x1190),
            (330, 0x1190),
            (342, 0x1f10),
            (493, 0x1190),
            (573, 0x1190),
        ],
    ),
    (
        "m0003 node init",
        &[
            (4, 0x30f0),
            (5, 0x30f0),
            (6, 0x30f0),
            (144, 0x3330),
            (173, 0x3170),
            (186, 0x33d0),
            (207, 0x33a0),
            (374, 0x3fe0),
            (379, 0x33d0),
            (513, 0x33e0),
        ],
    ),
];

#[test]
fn the_family_binds_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let mut handlers = 0;
    for &(port, carriers) in CARRIERS {
        for &(n, rva) in carriers {
            let name = format!("magic_{n:04}.dll");
            let file = std::fs::read(dir.join(&name)).unwrap();
            let img = ffx_magic::Image::new(&name, &file, 0x1000_0000).unwrap();
            assert_eq!(
                ffx_magic::covered(&img, 0x1000_0000 + rva),
                Some(port),
                "{name} {rva:#x}"
            );
            // the tick's handler thunks: each `jmp` into it binds
            if port == "m0003 tick" {
                let text = img.text;
                for a in (text.0..text.1).step_by(0x10) {
                    let at = 0x1000_0000 + a;
                    if ffx_magic::bind(&img, at)
                        .is_some_and(|i| ffx_magic::PORTS[i].0 == "m0003 tick handler")
                    {
                        assert_eq!(img.rel32_target(at + 8), 0x1000_0000 + rva);
                        handlers += 1;
                    }
                }
            }
        }
    }
    assert!(handlers >= 32, "{handlers} tick handlers");
}
