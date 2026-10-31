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

/// A slot that logs its first `n` words in row `k` of the log at
/// `SCRATCH + 0x3000` (each call adds its count to the top bits) and
/// answers `ret`.
fn log_slot(t: &mut Twin, slot: u32, n: usize, k: u32, ret: u32) {
    let row = SCRATCH + 0x3000 + 0x40 * k;
    t.stub(slot, move |m, a| {
        let c = m.r32(row + 0x3c).wrapping_add(1);
        m.w32(row + 0x3c, c);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 * i, w.wrapping_add(c << 28));
        }
        ret
    });
}

/// Operand offsets a spawner test needs: the tail's particle call
/// (`rel32`), its pointer global `A`, the particles' list, the ring
/// count, the model arrays' bases (`+ 0x98 * i`), the static node
/// arrays to fill with junk first (operand, bytes), the float limits
/// slot 251's value is tried at, and the limits after a factor (limit,
/// factor) with the kind that picks the factor.
struct SpawnerOps {
    part: u32,
    a: u32,
    parts: u32,
    count: u32,
    models: &'static [u32],
    /// Slot 390's answers to spawn with (the kind spawners).
    kinds: &'static [u32],
    fills: &'static [(u32, u32)],
    limits: &'static [u32],
    post: &'static [(u32, u32, u32)],
    /// The effect nodes' flag word operand (the words one and two
    /// nodes before it get junk too).
    flags: u32,
}

/// The float just at or around `x` whose product with `k` is exactly
/// `l` (the SSE `mulss`), if one is near.
fn solve(l: f32, k: f32) -> Option<f32> {
    let x = l / k;
    (-8i32..=8)
        .map(|d| f32::from_bits(x.to_bits().wrapping_add_signed(d)))
        .find(|v| (v * k).to_bits() == l.to_bits())
}

/// A ring spawner of the `magic_0003` family (`ffx_magic::m0003`'s
/// `spawn_head`/`spawn_tail`) in every state: state 11 before, at and
/// after the clock's limit, spawning the first and the last ring into an
/// empty and a filled root list, with slot 251's float below 0, at both
/// zeros, inside, at and above the limits and NaN, the model's flag bit
/// 0 clear and set; then each kind (the kind spawners).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn spawner_every_state(n: u16, rva: u32, o: &SpawnerOps) {
    let Some(mut t) = Twin::load(n) else { return };
    t.init();
    let f = t.at(rva);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let fl = |t: &Twin, off: u32| f32::from_bits(t.u32(t.img.word(f + off)));
    let (task, s, ring) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x280);
    let list = SCRATCH + 0x2f0;
    t.write(list, &[2, 5, 0, 0]);
    // slot 218: the model list (no models), its count through a[1]
    t.stub(218, move |m, a| {
        m.w32(SCRATCH + 0x3300, a[0]);
        m.w32(a[1], 0);
        list
    });
    log_slot(&mut t, 0x430 / 4, 3, 0, 0x2680);
    log_slot(&mut t, 0x3ec / 4, 1, 1, 0x2510);
    t.stub_st0(0x3ec / 4, |m, _| {
        f64::from(f32::from_bits(m.r32(SCRATCH + 0x2e0)))
    });
    t.stub(941, |_, _| SCRATCH + 0x400);
    log_slot(&mut t, 325, 2, 2, 0x3250);
    log_slot(&mut t, 719, 3, 3, 0x7190);
    log_slot(&mut t, 714, 5, 4, 0x7140);
    // facing_int's slots: the angle in ST0, or through the local
    log_slot(&mut t, 333, 1, 5, 0x3330);
    t.stub_st0(333, |_, _| 0.75);
    t.stub(601, |m, a| {
        m.w32(SCRATCH + 0x3200, a[0]);
        m.w32(a[1], (-1.25f32).to_bits());
        0x6010
    });
    for k in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * k, 0x0f00_0000 + k);
    }
    // particle_node's pointer global and the owner behind *A
    let pn = t.img.rel32_target(f + o.part);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x700);
    t.set_u32(w(&t, o.a), SCRATCH + 0x600);
    t.write(SCRATCH + 0x600, &[0x31; 0x80]);
    let root = w(&t, 0x6f);
    let other = SCRATCH + 0x800;
    t.set_u32(w(&t, o.count), 2);
    // the particles' list: empty
    let parts = w(&t, o.parts);
    t.set_u32(parts + 0x10, parts);
    // slot 665 (the kind spawners' N1) logs; slot 390 answers the kind
    log_slot(&mut t, 0xa64 / 4, 2, 6, 0x6650);
    t.stub(0x618 / 4, |m, a| {
        m.w32(SCRATCH + 0x3204, a[0]);
        m.r32(SCRATCH + 0x2e4)
    });
    // the model nodes' actor
    t.set_u32(w(&t, 0x5b), 0x00a5_0000);
    // the static nodes hold junk (a float per word) before the spawn
    for &(op, len) in o.fills {
        let at = w(&t, op);
        for i in 0..len / 4 {
            #[allow(clippy::cast_precision_loss)]
            t.set_u32(at + 4 * i, (1.0 + i as f32 * 0.375).to_bits());
        }
    }
    let fw = w(&t, o.flags);
    t.set_u32(fw - 56, 0x0f0f_0f00);
    t.set_u32(fw - 112, 0x0e0e_0e00);
    // junk under the entry: the records' words must be written
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    let base = t.save();
    let mut cases = vec![
        (0u32, 7u32, 3u32, false, 1.0f32, 0x3002u32),
        (1, 0, 0, false, 1.0, 0x3002),
        (9, 3, 0, false, 1.0, 0x3002),
        (10, 7, 5, false, 1.0, 0x3002),
        (11, 13, 0, false, 1.0, 0x3002),
        (11, 14, 0, false, 1.0, 0x3002),
        (11, 29, 0, false, 1.0, 0x3002),
        (11, 44, 0, false, 1.0, 0x3002),
        (11, 45, 0, false, 1.0, 0x3002),
        (11, 46, 0, false, 50.0, 0x3002),
        (11, 46, 1, true, -2.0, 0x3002),
        (11, 100, 0, true, 150.0, 0x3002),
        (11, 0xffff_ffff, 1, false, 100.0, 0x3002),
        (11, 46, 1, false, f32::NAN, 0x3002),
        (11, 46, 0, true, 0.0, 0x3002),
        (11, 46, 1, true, -0.0, 0x3002),
        (11, 46, 1, true, 0.5, 0x3002),
        (11, 46, 0, false, 12.0, 0x3002),
        (11, 46, 1, false, 30.0, 0x3002),
        (11, 46, 0, false, 1.0e6, 0x3002),
        (11, 46, 2, true, 5.0, 0x3002),
        (12, 0, 0, false, 1.0, 0x3002),
        (9_999, 0, 0, false, 1.0, 0x3002),
        (10_000, 5, 0, false, 1.0, 0x3002),
        (10_001, 5, 0, false, 1.0, 0x3002),
    ];
    for (j, &l) in (0u32..).zip(o.limits) {
        cases.push((11, 46, j & 1, j % 2 == 0, fl(&t, l), 0x3002));
    }
    for (j, &(l, k, kind)) in (0u32..).zip(o.post) {
        if let Some(v) = solve(fl(&t, l), fl(&t, k)) {
            cases.push((11, 46, j & 1, false, v, kind));
        }
    }
    for (j, &kind) in (0u32..).zip(o.kinds) {
        let v = [5.0f32, 0.25, 40.0, f32::NAN, 1.0, 17.5][j as usize % 6];
        cases.push((11, 46 + j, j & 1, j % 3 == 0, v, kind));
    }
    for (state, clock, index, tail, v, kind) in cases {
        t.restore(&base);
        t.set_u32(SCRATCH + 0x2e4, kind);
        for &m in o.models {
            let models = w(&t, m);
            for i in 0..2 {
                t.write(models + 0x98 * i, &[0x44; 0x98]);
                t.set_u32(models + 0x98 * i + 0x50, 0x0050_0000 + i);
                t.set_u32(models + 0x98 * i + 0x54, 0x0054_0000 + i);
                t.set_u8(models + 0x98 * i + 0x90, u8::from(tail) | 0x44);
            }
        }
        t.write(s, &[0x5c; 0x40]);
        t.set_u32(s + 0x18, state);
        t.set_u32(s + 0x1c, 1.5f32.to_bits());
        t.set_u32(s + 0x20, (-2.25f32).to_bits());
        t.set_u32(s + 0x24, index);
        t.set_u32(s + 0x28, clock);
        // the task's ring node between two neighbours
        t.write(ring, &[0x66; 0x60]);
        t.set_u32(task, ring + 0x20);
        t.set_u32(task + 4, s);
        t.set_u32(ring + 0x24, ring);
        t.set_u32(ring + 0x28, ring + 0x40);
        t.set_u32(SCRATCH + 0x2e0, v.to_bits());
        // the root list: empty, or ending in another node
        t.write(other, &[0x77; 0x20]);
        t.set_u32(root + 0x10, if tail { other } else { root });
        t.set_u16(root + 0x18, 3);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn spawn_rings_every_state() {
    let o = SpawnerOps {
        part: 0x2c0,
        a: 0x286,
        parts: 0x2a3,
        count: 0x339,
        models: &[0x147],
        kinds: &[],
        fills: &[
            (0xd9, 0x84),
            (0xdf, 0x84),
            (0xc1, 0x3e4),
            (0x109, 0x300),
            (0x2df, 0xa8),
        ],
        limits: &[0x22d],
        post: &[],
        flags: 0x321,
    };
    spawner_every_state(3, 0x6540, &o);
}

#[test]
fn spawn_node_every_state() {
    let o = SpawnerOps {
        part: 0x268,
        a: 0x22e,
        parts: 0x24b,
        count: 0x2e1,
        models: &[0xe0],
        kinds: &[],
        fills: &[
            (0xc4, 0xcc),
            (0xf3, 0x3e4),
            (0xfd, 0x84),
            (0xe5, 0x300),
            (0x287, 0xa8),
        ],
        limits: &[0x1b9, 0x1d9],
        post: &[],
        flags: 0x2c9,
    };
    spawner_every_state(4, 0x65b0, &o);
    spawner_every_state(5, 0x65b0, &o);
}

#[test]
fn spawn_facing_every_state() {
    let o = SpawnerOps {
        part: 0x2d8,
        a: 0x29e,
        parts: 0x2bb,
        count: 0x351,
        models: &[0x101],
        kinds: &[],
        fills: &[
            (0xc1, 0x3e4),
            (0xdb, 0xcc),
            (0xea, 0x84),
            (0xf0, 0x84),
            (0x106, 0x300),
            (0x2f7, 0xa8),
        ],
        limits: &[0x225, 0x245],
        post: &[],
        flags: 0x339,
    };
    spawner_every_state(6, 0x6640, &o);
}

#[test]
fn spawn_kind_table_every_state() {
    let o = SpawnerOps {
        part: 0x3a8,
        a: 0x36e,
        parts: 0x38b,
        count: 0x421,
        models: &[0x140, 0x1ba],
        kinds: &[0x3003, 0x3004, 0x3005, 0x3006, 0x3007, 0x3001, 0x3008, 0],
        fills: &[
            (0xc1, 0x3e4),
            (0xd9, 0x84),
            (0xdf, 0x84),
            (0x109, 0x300),
            (0x3c7, 0xa8),
        ],
        limits: &[0x1f8, 0x218],
        post: &[],
        flags: 0x409,
    };
    spawner_every_state(7, 0x6540, &o);
}

#[test]
fn spawn_kind_branch_every_state() {
    let o = SpawnerOps {
        part: 0x3a1,
        a: 0x367,
        parts: 0x384,
        count: 0x41a,
        models: &[0x140, 0x1ba, 0x261],
        kinds: &[
            0x108c,
            0x3003,
            0x3006,
            0x3007,
            0x3004,
            0x3005,
            0x3008,
            0x108b,
            0,
            0xffff_ffff,
            0x8000_0000,
            0x108d,
        ],
        fills: &[
            (0xc1, 0x3e4),
            (0xd9, 0x84),
            (0xdf, 0x84),
            (0x109, 0x300),
            (0x3c0, 0xa8),
        ],
        limits: &[0x1f8, 0x218],
        post: &[(0x30b, 0x29d, 0), (0x320, 0x29d, 0), (0x320, 0x2ca, 0x3004)],
        flags: 0x402,
    };
    spawner_every_state(8, 0x6540, &o);
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
