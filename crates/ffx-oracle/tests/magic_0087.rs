//! `magic_0087.dll` against its Rust ports: the node lists the overlays
//! share (`ffx_magic::nodes`, 100 to 116 DLLs) and `xor eax, eax; ret`
//! (402 DLLs). Needs `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, nodes};
use magic::{Call, Eax, SCRATCH, TRAPS, Twin};

const N: u16 = 87;

/// The group object: `+0` the owner node, `+4` the flag, `+5` the mode.
const GROUP: u32 = SCRATCH + 0x1000;
/// The owner node: `+0x10` its first child, `+0x14` its parent.
const OWNER: u32 = SCRATCH + 0x2000;
/// The owner's parent: the walks add into its counters.
const PARENT: u32 = SCRATCH + 0x2400;
/// Children, 0x100 bytes apart.
const KIDS: u32 = SCRATCH + 0x3000;
/// Vtables, 0x40 bytes apart: `+0x1c` is the method the walk calls.
const VTABLES: u32 = SCRATCH + 0x800;
/// What the stubs record.
const LOG: u32 = SCRATCH + 0x4000;
/// The unlink's parent and children.
const PAR: u32 = SCRATCH + 0x5000;
const NODES: u32 = SCRATCH + 0x6000;

/// Slot `s`'s trap: a stub answers it (or, without one, 0).
fn trap(s: u32) -> u32 {
    TRAPS + 16 * s
}

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// The stubs: the walk's method (1 argument, the group), slot 248
/// (`FFXVu0AddVector`, 3 words), slot 268 (3 words), slot 713
/// (`pppAllFreePObject`, the tail jump's) and slot 791
/// (`graphicSetGlobalAmbientColor`, 3 words). Each records what it was given
/// and answers something the port must pass on.
fn stubs(t: &mut Twin) {
    // the walk's method slots: table words, so the original reports them
    // as call-outs to slots like the port does
    for s in [1000, 1001, 1002, 1003] {
        t.set_u32(magic::TABLE + 4 * s, trap(s));
    }
    t.stub(1000, |m, a| {
        m.w32(LOG, a[0]);
        // the word the argument points to (a stack slot's node, or a group's)
        m.w32(LOG + 0x18, m.r32(a[0]));
        let n = m.r32(LOG + 8).wrapping_add(1);
        m.w32(LOG + 8, n);
        0x5a00_0000 + n
    });
    // 1001 changes the group's flag and mode bytes: the walk's tail reads
    // the mode again after its calls
    t.stub(1001, |m, a| {
        m.w32(LOG + 0x10, a[0]);
        m.w32(GROUP + 4, 0x0000_3c00);
        0xc0de_0000
    });
    // 1002 has no stub: it answers 0
    // 1003 (a pointer's target that leaves the group alone) records the
    // second word of its argument
    t.stub(1003, |m, a| {
        m.w32(LOG + 0x1c, m.r32(a[0] + 4));
        0x1003_0003
    });
    t.stub(248, |m, a| {
        for i in 0..3 {
            m.w32(LOG + 0x20 + 4 * i, a[i as usize]);
        }
        0x2480_0001
    });
    t.stub(268, |m, a| {
        for i in 0..3 {
            m.w32(LOG + 0x60 + 4 * i, a[i as usize]);
        }
        0x2680_0001
    });
    t.stub(713, |m, a| {
        m.w32(LOG + 0x80, a[0]);
        0x7130_0003
    });
    t.stub(791, |m, a| {
        for i in 0..3 {
            m.w32(LOG + 0x40 + 4 * i, a[i as usize]);
        }
        0x7910_0002
    });
}

/// The group at `GROUP` (flag, mode), its owner (mode `owner_mode`, parent
/// `PARENT` with counters) and one child per method, each with its own
/// vtable. No methods: the owner alone in its list.
fn group(t: &mut Twin, flag: u8, mode: u8, owner_mode: u8, methods: &[u32]) {
    t.set_u32(GROUP, OWNER);
    t.set_u8(GROUP + 4, flag);
    t.set_u8(GROUP + 5, mode);
    t.set_u8(OWNER + 0x1a, owner_mode);
    t.set_u32(OWNER + 0x14, PARENT);
    for (k, &m) in methods.iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let c = KIDS + 0x100 * k;
        let vt = VTABLES + 0x40 * k;
        t.set_u32(vt + 0x1c, m);
        t.set_u32(c, vt);
        let prev = if k == 0 { OWNER } else { c - 0x100 };
        let next = if k as usize + 1 == methods.len() {
            OWNER
        } else {
            c + 0x100
        };
        t.set_u32(c + 4, prev);
        t.set_u32(c + 8, next);
        t.set_u32(c + 0x14, OWNER);
        t.set_u8(c + 0x1a, 0x30 + u8::try_from(k).unwrap());
    }
    t.set_u32(OWNER + 0x10, if methods.is_empty() { OWNER } else { KIDS });
    for i in 0..4 {
        t.set_u32(OWNER + 0x1c + 4 * i, 0x1000 * (i + 1) + 0x11);
        t.set_u32(PARENT + 0x1c + 4 * i, 0x100 * (i + 1) + 0x77);
    }
}

#[test]
fn visit_nodes_all_paths() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let method_sets: [&[u32]; 4] = [
        &[],
        &[trap(1000)],
        &[trap(1001), trap(1002), trap(1000)],
        &[trap(1002)],
    ];
    // (flag, mode, owner's +0x1a): equal modes take the early exit
    for (flag, mode, owner_mode) in [
        (0u8, 3u8, 3u8),
        (0, 3, 9),
        (1, 7, 7),
        (1, 7, 9),
        (0, 0, 0xff),
    ] {
        for methods in method_sets {
            for eax in [0x1234_5678u32, 0xffff_ff00] {
                // each check starts from the same group: the walk changes it
                group(&mut t, flag, mode, owner_mode, methods);
                t.check(
                    t.at(0x1010),
                    Call {
                        eax,
                        ecx: GROUP,
                        edx: 0,
                        ..Call::default()
                    },
                    &[],
                    Eax::Checked,
                    |g, _img, fr| Ret::eax(nodes::visit_nodes(g, fr.ecx, fr.eax)),
                );
            }
        }
    }
}

#[test]
fn visit_thunk_jumps_into_the_walk() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for methods in [&[][..], &[trap(1000)][..], &[trap(1001), trap(1000)][..]] {
        group(&mut t, 0, 5, 9, methods);
        t.check_entry(
            t.at(0x1080),
            Call {
                eax: 0x0bad_f00d,
                ecx: 0x1111,
                edx: 0x2222,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
        group(&mut t, 0, 9, 9, methods);
        t.check_entry(
            t.at(0x1080),
            Call {
                eax: 0x0bad_f00d,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
}

#[test]
fn visit_nodes_vu0_all_paths() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let method_sets: [&[u32]; 3] = [&[], &[trap(1000)], &[trap(1001), trap(1002)]];
    for (flag, mode, owner_mode) in [(0u8, 4u8, 4u8), (0, 4, 1), (1, 2, 2), (1, 2, 8)] {
        for methods in method_sets {
            group(&mut t, flag, mode, owner_mode, methods);
            t.check(
                t.at(0x1090),
                Call {
                    eax: 0x7777_0001,
                    ecx: GROUP,
                    edx: 0,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
                |g, img, fr| Ret::eax(nodes::visit_nodes_vu0(g, img, fr.at, fr.ecx, fr.eax)),
            );
        }
    }
}

#[test]
fn visit_thunk_vu0_jumps_into_the_walk() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for methods in [&[][..], &[trap(1000), trap(1001)][..]] {
        group(&mut t, 1, 6, 1, methods);
        t.check_entry(
            t.at(0x1100),
            Call {
                eax: 0x3131,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
}

#[test]
fn visit_slot268_all_paths() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let method_sets: [&[u32]; 3] = [&[], &[trap(1000)], &[trap(1001), trap(1002)]];
    for (flag, mode, owner_mode) in [(0u8, 4u8, 4u8), (0, 4, 1), (1, 2, 2)] {
        for methods in method_sets {
            group(&mut t, flag, mode, owner_mode, methods);
            t.check(
                t.at(0x40d0),
                Call {
                    eax: 0x2680_0009,
                    ecx: GROUP,
                    edx: 0,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
                |g, img, fr| Ret::eax(nodes::visit_slot268(g, img, fr.at, fr.ecx, fr.eax)),
            );
        }
    }
}

#[test]
fn visit_thunk_slot268_jumps_into_the_walk() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for methods in [&[][..], &[trap(1000), trap(1001)][..]] {
        group(&mut t, 1, 6, 1, methods);
        t.check_entry(
            t.at(0x4140),
            Call {
                eax: 0x4141,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
}

#[test]
fn visit_plain_gated_and_add_float() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    // a float the owner adds into its parent: 1.5 + -0.25, and a sum that
    // rounds (2^24 + 1 is 2^24 in single precision)
    for (a, b) in [
        (0x3fc0_0000u32, 0xbe80_0000u32),
        (0x4b80_0000, 0x3f80_0000),
        (0x7f7f_ffff, 0x7f7f_ffff),
    ] {
        for methods in [&[][..], &[trap(1000)][..], &[trap(1001), trap(1002)][..]] {
            for (flag, mode, owner_mode) in [(0u8, 3u8, 3u8), (0, 3, 9), (1, 7, 7), (1, 7, 9)] {
                for eax in [0x1234_5678u32, 0xffff_ff00] {
                    // each check starts from the same state: the walk changes the group
                    group(&mut t, flag, mode, owner_mode, methods);
                    t.set_u32(OWNER + 0x1c, a);
                    t.set_u32(PARENT + 0x20, b);
                    t.check_entry(
                        t.at(0x34d0),
                        Call {
                            eax,
                            ecx: 0,
                            edx: 0,
                            ..Call::default()
                        },
                        &[GROUP],
                        Eax::Checked,
                    );
                    group(&mut t, flag, mode, owner_mode, methods);
                    t.set_u32(OWNER + 0x1c, a);
                    t.set_u32(PARENT + 0x20, b);
                    t.check_entry(
                        t.at(0x3460),
                        Call {
                            eax,
                            ecx: 0,
                            edx: 0,
                            ..Call::default()
                        },
                        &[GROUP],
                        Eax::Checked,
                    );
                }
            }
        }
    }
    // the gate: the owner's byte +0x1b with the DLL's global at 0x100b0140
    for (idle, gate) in [(0u8, 0u32), (0, 1), (1, 0), (1, 1)] {
        group(&mut t, 0, 3, 3, &[trap(1000)]);
        t.set_u8(OWNER + 0x1b, idle);
        t.set_u32(t.at(0xb0140), gate);
        t.check_entry(
            t.at(0x3400),
            Call {
                eax: 0x3400_0001,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
}

#[test]
fn unlink_list_links_the_node_to_itself() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let (p, x) = (NODES + 0x400, NODES + 0x500);
    let node = NODES + 0x300;
    for (prev, next) in [(p, x), (p, p), (node, node)] {
        // the neighbours' words the function overwrites first, then the
        // node's own links (a node's link never holds junk)
        t.set_u32(prev + 8, 0xa1);
        t.set_u32(next + 4, 0xa2);
        t.set_u32(GROUP, node);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.check_entry(
            t.at(0x3520),
            Call {
                eax: 0x3520_0001,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
}

#[test]
fn flag_calls_pass_the_node_or_the_pointer() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let node = SCRATCH + 0x7000;
    let vt = VTABLES + 0x400;
    t.set_u32(vt + 0x1c, trap(1000));
    t.set_u32(node, vt);
    t.set_u32(node + 0x10, trap(1003));
    t.set_u32(node + 0x14, PARENT);
    for bit in [0u8, 1] {
        t.set_u8(node + 0xc, bit);
        for flag in [0u32, 1] {
            t.set_u32(GROUP, node);
            t.set_u32(GROUP + 4, flag);
            t.check_entry(
                t.at(0x5f70),
                Call {
                    eax: 0x5f70_0001,
                    ecx: 0,
                    edx: 0,
                    ..Call::default()
                },
                &[GROUP],
                Eax::Checked,
            );
            t.set_u32(GROUP, node);
            t.set_u32(GROUP + 4, flag);
            t.check_entry(
                t.at(0x5f20),
                Call {
                    eax: 0x5f20_0001,
                    ecx: 0,
                    edx: 0,
                    ..Call::default()
                },
                &[GROUP],
                Eax::Checked,
            );
        }
    }
}

#[test]
fn free_tail_stores_and_jumps_to_slot_713() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let node = SCRATCH + 0x7800;
    let cell = SCRATCH + 0x7900;
    // the DLL's global holds the cell the store goes through
    t.set_u32(t.at(0xb0428), cell);
    t.set_u32(GROUP, node);
    for flag in [0u8, 4] {
        t.set_u8(node + 0xe8, flag);
        t.set_u32(node + 0x50, 0x5050_0001);
        t.set_u32(cell, 0);
        t.check_entry(
            t.at(0x3c20),
            Call {
                eax: 0x3c20_0001,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
}

#[test]
fn node_insert_links_the_node_into_its_parent() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let node = SCRATCH + 0x7a00;
    let kid_a = NODES + 0x600;
    let kid_b = NODES + 0x700;
    // no parent; an empty parent (it is its own first child); a parent with
    // two children
    for case in 0..3 {
        t.set_u32(node, 0x0bad_0000);
        t.set_u32(GROUP, node);
        t.set_u32(GROUP + 4, if case == 0 { 0 } else { PAR });
        t.set_u32(GROUP + 8, 0x8000 + case);
        t.set_u32(GROUP + 0xc, 0xc000 + case);
        t.set_u32(GROUP + 0x10, 0x1000 + case);
        t.set_u32(GROUP + 0x14, 0x1400 + case);
        match case {
            0 => {}
            1 => {
                t.set_u32(PAR + 0x10, PAR);
                t.set_u16(PAR + 0x18, 0);
            }
            _ => chain(&mut t, &[kid_a, kid_b], 2),
        }
        t.check_entry(
            t.at(0x5df0),
            Call {
                eax: 0,
                ecx: GROUP,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
        );
    }
}

/// `ports` for the unlink: the parent `PAR` with its children `kids` in
/// list order (its `+0x10` first, each `+8` next, the last back to PAR).
fn chain(t: &mut Twin, kids: &[u32], count: u16) {
    t.set_u32(PAR + 0x10, kids.first().copied().unwrap_or(PAR));
    for (i, &k) in kids.iter().enumerate() {
        let prev = if i == 0 { PAR } else { kids[i - 1] };
        let next = kids.get(i + 1).copied().unwrap_or(PAR);
        t.set_u32(k + 4, prev);
        t.set_u32(k + 8, next);
        t.set_u32(k + 0x14, PAR);
        let i = u32::try_from(i).unwrap();
        t.set_u32(k + 0x5c, 0x3f80_0000 + i);
        t.set_u32(k + 0x60, 0x3f00_0000 + i);
        t.set_u32(k + 0x64, 0x3e80_0000 + 0x100 * i);
    }
    t.set_u16(PAR + 0x18, count);
}

#[test]
fn unlink_takes_the_node_out_of_its_list() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let a = NODES;
    let b = NODES + 0x100;
    let c = NODES + 0x200;
    // (children in order, the one unlinked, the count the parent holds)
    let cases: [(&[u32], u32, u16); 7] = [
        (&[a], a, 1),
        (&[a, b], a, 2),
        (&[a, b], b, 2),
        (&[a, b, c], a, 3),
        (&[a, b, c], b, 3),
        (&[a, b, c], c, 3),
        (&[a], a, 0),
    ];
    for (kids, node, count) in cases {
        chain(&mut t, kids, count);
        t.set_u32(GROUP, node);
        t.check_entry(
            t.at(0x1110),
            Call {
                eax: 0x4444,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[GROUP],
            Eax::Checked,
        );
    }
    // a node with no parent: no list to leave, the call still runs
    chain(&mut t, &[a, b], 2);
    t.set_u32(a + 0x14, 0);
    t.set_u32(GROUP, a);
    t.check_entry(
        t.at(0x1110),
        Call {
            eax: 0x4444,
            ecx: 0,
            edx: 0,
            ..Call::default()
        },
        &[GROUP],
        Eax::Checked,
    );
}

#[test]
fn zero_ret_returns_zero() {
    let Some(mut t) = twin() else { return };
    // the lone `xor eax, eax; ret` of 402 DLLs, at its address in this one
    let f = t.at(0x6fd0);
    for eax in [0u32, 0x1234_5678, 0xffff_ffff] {
        t.check_entry(
            f,
            Call {
                eax,
                ecx: 3,
                edx: 4,
                ..Call::default()
            },
            &[],
            Eax::Checked,
        );
    }
}

#[test]
fn visit_children_calls_the_children_then_adds_their_vectors() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    // two children of the owner (+0x2c and +0x30), each with its own vtable
    // and method, and vector words for the slot 248 add
    let (na, nb) = (SCRATCH + 0x7c00, SCRATCH + 0x7d00);
    for (k, node, m) in [(0u32, na, trap(1000)), (1, nb, trap(1001))] {
        let vt = VTABLES + 0x800 + 0x40 * k;
        t.set_u32(vt + 0x1c, m);
        t.set_u32(node, vt);
        for i in 0..4 {
            t.set_u32(node + 0x1c + 4 * i, 0x100 * (k + 1) + i);
        }
    }
    for (flag, mode, owner_mode) in [(0u8, 4u8, 4u8), (0, 4, 1), (1, 2, 2)] {
        for methods in [&[][..], &[trap(1002)][..]] {
            group(&mut t, flag, mode, owner_mode, methods);
            t.set_u32(OWNER + 0x2c, na);
            t.set_u32(OWNER + 0x30, nb);
            t.check(
                t.at(0x5e70),
                Call {
                    eax: 0x5e70_0001,
                    ecx: GROUP,
                    edx: 0,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
                |g, img, fr| Ret::eax(nodes::visit_children(g, img, fr.at, fr.esp, fr.ecx, fr.eax)),
            );
            group(&mut t, flag, mode, owner_mode, methods);
            t.set_u32(OWNER + 0x2c, na);
            t.set_u32(OWNER + 0x30, nb);
            t.check_entry(
                t.at(0x5f10),
                Call {
                    eax: 0x5f10_0001,
                    ecx: 0,
                    edx: 0,
                    ..Call::default()
                },
                &[GROUP],
                Eax::Checked,
            );
        }
    }
}

#[test]
fn free_then_unlink_frees_and_leaves_the_parent() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let cell = SCRATCH + 0x7900;
    t.set_u32(t.at(0xb0428), cell);
    let (a, b, c) = (NODES, NODES + 0x100, NODES + 0x200);
    for (kids, node, count) in [
        (vec![a], a, 1u16),
        (vec![a, b], a, 2),
        (vec![a, b], b, 2),
        (vec![a, b, c], b, 3),
        (vec![a, b, c], c, 3),
    ] {
        chain(&mut t, &kids, count);
        t.set_u32(GROUP, node);
        // the two words the slot's store reads: nonzero, so a store that goes
        // to the wrong place or takes the wrong word shows
        t.set_u32(node + 0x50, 0x5050_0001);
        t.set_u32(node + 0x54, 0x5454_0002);
        t.set_u32(cell, 0);
        t.check_entry(
            t.at(0x3c70),
            Call {
                eax: 0x3c70_0001,
                ecx: GROUP,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
        );
    }
    // no parent: the slot's result stays
    chain(&mut t, &[a, b], 2);
    t.set_u32(a + 0x14, 0);
    t.set_u32(GROUP, a);
    t.set_u32(a + 0x50, 0x5050_0001);
    t.set_u32(a + 0x54, 0x5454_0002);
    t.set_u32(cell, 0);
    t.check_entry(
        t.at(0x3c70),
        Call {
            eax: 0x3c70_0002,
            ecx: GROUP,
            edx: 0,
            ..Call::default()
        },
        &[],
        Eax::Checked,
    );
}
