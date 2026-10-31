//! The task steps and small object functions of `ffx_magic::o4_steps`
//! (and the further copies of `ob_summon::step_plain`) against every
//! DLL's own copy, and `DllMain` of each of those DLLs. Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};
use ownb::{rec, rec_all};

const STEP_PLAIN: &[(u16, u32)] = &[
    (162, 0x2190),
    (348, 0x65c0),
    (350, 0x5a10),
    (352, 0x4c90),
    (353, 0x1940),
    (392, 0x1460),
];
const STEP_THEN_REGISTER: &[(u16, u32)] = &[
    (165, 0x1760),
    (307, 0x15a0),
    (605, 0x6ef0),
    (612, 0x7390),
    (688, 0x3a90),
];
const RELEASE_NODE: &[(u16, u32)] = &[
    (162, 0x4120),
    (165, 0x3050),
    (430, 0x3190),
    (431, 0x4360),
    (586, 0x3d30),
    (587, 0x3af0),
    (588, 0x2580),
];
const SUB_CONST: &[(u16, u32)] = &[
    (168, 0x33a0),
    (430, 0x3860),
    (568, 0x2820),
    (568, 0x2dc0),
    (587, 0x2ec0),
];
const POOL_NODE: &[(u16, u32)] = &[
    (350, 0x6280),
    (353, 0x2180),
    (434, 0x7a50),
    (610, 0xb980),
    (672, 0x1860),
];

const STATE: u32 = SCRATCH + 0x8000;

fn node(i: u32) -> u32 {
    SCRATCH + 0x1000 + 0x40 * i
}

fn stubs(t: &mut Twin) {
    rec_all(t, &[(723, 1), (724, 2), (235, 3), (214, 1)]);
    rec(t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
    rec(t, 772, 2, |_, a| 0x7720_0000 ^ a[0] ^ a[1] << 8);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 779, 4, |_, a| a[0]);
}

/// A step's cases: the object list empty or one paused object (it ran),
/// the done word 1, 0 or 2.
fn steps(t: &mut Twin, f: u32, mut each: impl FnMut(&mut Twin)) {
    let ro = t.img.rel32_target(f + 0x08);
    let list = t.img.word(ro + 0x7c);
    let d = t.img.word(f + 0x13);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    t.set_u32(t.img.word(f + 0x03), STATE);
    t.set_u32(t.img.word(f + 0x20), 0x0bfc_0000 + f);
    for (busy, done) in [(false, 1), (false, 0), (false, 2), (true, 1)] {
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(STATE + d, done);
        each(t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn steps_in_every_copy() {
    for (n, rva) in copies("o4 task step (no pause)", STEP_PLAIN) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        steps(&mut t, f, |_| {});
    }
    for (n, rva) in copies("o4 step then register", STEP_THEN_REGISTER) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        // the registration's parts and banks (its own operands)
        let r = t.img.rel32_target(f + 0x31);
        let w = |t: &Twin, off: u32| t.img.word(r + off);
        let (count, table, lock, banks) = (w(&t, 0x0e), w(&t, 0x16), w(&t, 0x43), w(&t, 0x76));
        let recs = SCRATCH + 0x4000;
        t.set_u32(table, recs);
        for i in 0..3 {
            t.set_u32(recs + 0x74 * i + 0x30, 0x0300_0000 + i);
        }
        t.set_u32(lock, SCRATCH + 0x800);
        for (parts, b) in [(0u32, 0u32), (2, 1)] {
            steps(&mut t, f, |t| {
                t.set_u32(count, parts);
                t.set_u32(banks, b);
                t.set_u32(SCRATCH + 0x800, 0x5555);
            });
        }
    }
}

#[test]
fn release_node_in_every_copy() {
    for (n, rva) in copies("o4 release node", RELEASE_NODE) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let base = SCRATCH + 0x6000;
        t.set_u32(t.img.word(f + 0x03), base);
        let field = base + t.img.word(f + 0x09);
        let give = t.img.rel32_target(f + 0x12);
        let w = |t: &Twin, off: u32| t.img.word(give + off);
        let (cur, head, tail, free, count) = (
            w(&t, 0x48),
            w(&t, 0x55),
            w(&t, 0x64),
            w(&t, 0x74),
            w(&t, 0x8e),
        );
        for case in 0..3u32 {
            let me = node(10);
            t.write(node(9), &[0x3c; 0xc0]);
            t.set_u32(me, if case == 2 { node(9) } else { 0 });
            t.set_u32(me + 4, 0);
            t.set_u32(me + 8, me);
            t.set_u32(me + 0xc, 0);
            t.set_u32(node(9) + 4, me);
            t.set_u32(cur, node(11));
            t.set_u32(head, me);
            t.set_u32(tail, node(12));
            t.set_u32(free, if case == 1 { node(11) } else { 0 });
            t.set_u32(node(11), node(11));
            t.set_u32(node(11) + 4, node(11));
            t.set_u16(count, 0x20);
            t.set_u32(field, if case == 0 { 0 } else { me });
            t.check_entry(
                f,
                Call {
                    eax: 0x7777_0000 + case,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
            );
        }
    }
}

#[test]
fn sub_const_in_every_copy() {
    for (n, rva) in copies("o4 sub const", SUB_CONST) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let p = SCRATCH + 0x6000;
        t.set_u32(t.img.word(f + 0x01), p);
        let d = t.img.word(f + 0x09);
        for x in [1.5f32, -3.25, 0.0, f32::INFINITY, 1.0e-39, 7.0e30] {
            t.set_u32(p + d, x.to_bits());
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

#[test]
fn pool_node_in_every_copy() {
    for (n, rva) in copies("o4 pool node", POOL_NODE) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let rec_at = SCRATCH + 0x6000;
        t.set_u32(t.img.word(f + 0x01), rec_at);
        let take = t.img.rel32_target(f + 0x3d);
        let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
        let (head, tail) = (t.img.word(f + 0x43), t.img.word(f + 0x53));
        for case in 0..2u32 {
            // a free ring of three nodes
            for i in 1..=3 {
                t.write(node(i), &[0x5a; 0x1c]);
                t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
                t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
            }
            t.set_u32(free, node(1));
            t.set_u16(count, 3);
            for k in 0..5 {
                let d = t.img.word(f + 0x0a + 10 * k + 2);
                fill(&mut t, rec_at + d - 4, 3, rva ^ case ^ k);
            }
            t.write(node(8), &[0x77; 0x1c]);
            t.set_u32(head, if case == 0 { 0 } else { node(8) });
            t.set_u32(tail, node(8));
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    for n in dlls(&[
        STEP_PLAIN,
        STEP_THEN_REGISTER,
        RELEASE_NODE,
        SUB_CONST,
        POOL_NODE,
    ]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, 23 mutants): 20
// killed; 3 survive, equivalent: `pool_node` clearing the node's `+0x4`
// and `prev` and setting `+0x8` to itself, which the pool's `take` has
// just done (the original repeats them).
