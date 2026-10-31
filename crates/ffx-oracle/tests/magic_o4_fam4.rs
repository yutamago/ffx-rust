//! The families of `ffx_magic::o4_fam4` against every DLL's own copy,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_fam4};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its name, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

const MESH_FLAGS: &[(u16, u32)] = &[(391, 0x3e40), (392, 0x4070)];

/// Every slot answers a value of its first word.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

const CASES: &[Case] = &[
    (
        "three_tasks_keep",
        &[(313, 0x1240), (361, 0x1330)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            t.set_u32(P + t.img.word(f + 0x0c), 0x4470_0000 ^ f);
            t.set_u32(OBJ + 0xac, c + 1);
            fill(t, OBJ + 0x2c, 0x10, f ^ c);
        },
    ),
    (
        "slot_attach_mode_par",
        &[(194, 0x1620), (359, 0x1420)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x14), P);
            fill(t, P + t.img.word(f + 0x1f), 1, c);
            for k in 0..4 {
                fill(t, OBJ + t.img.word(f + 0x53 + 10 * k), 1, c ^ k);
            }
            // the mode call moves the game object: the set-par reads it again
            let g = t.img.word(f + 0x14);
            t.stub(443, move |m, a| {
                m.w32(g, SCRATCH + 0x5000);
                a[0] ^ 0x443
            });
        },
    ),
    (
        "attach_par_seq",
        &[(312, 0x15b0), (378, 0x1250)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P + t.img.word(f + 0x10), 1, c);
            for k in 0..4 {
                fill(t, OBJ + t.img.word(f + 0x34 + 10 * k), 1, c ^ k);
            }
            // the attach moves the game object: the set-par reads it again
            let g = t.img.word(f + 0x04);
            for s in [441u32, 442] {
                t.stub(s, move |m, a| {
                    m.w32(g, SCRATCH + 0x5000);
                    a[0] ^ s
                });
            }
        },
    ),
    (
        "copy_then_rect",
        &[(511, 0x2f70), (589, 0x2b50)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x12), P);
            for op in [0x1f, 0x29, 0x2f, 0x35, 0x3b, 0x41, 0x47, 0x4d, 0x53] {
                fill(t, P + t.img.word(f + op), 1, c ^ op);
            }
            // the slot reads the rectangle it is given
            t.stub(332, |m, a| {
                let mut x = 0x332u32;
                for i in 0..4 {
                    x = x.rotate_left(5) ^ m.r32(a[0] + 4 * i);
                }
                x
            });
        },
    ),
    ("packet", &[(267, 0x2f80), (431, 0x1430)], 1, |t, f, c| {
        t.set_u32(t.img.word(f + 0x02), P);
        fill(t, P + 0x20, 0x10, c);
        fill(t, t.img.word(f + 0x0d), 0x10, c ^ 5);
    }),
    (
        "mixed_record_node",
        &[(473, 0x2010), (673, 0x1850)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x03), P);
            fill(t, P + t.img.word(f + 0x09) + 0x80, 0x08, c);
            let take = t.img.rel32_target(f + 0x65);
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
            for i in 1..=3 {
                t.write(node(i), &[0x5a; 0x1c]);
                t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
                t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
            }
            t.set_u32(free, node(1));
            t.set_u16(count, 3);
            let node8 = SCRATCH + 0x1200;
            t.write(node8, &[0x77; 0x1c]);
            t.set_u32(t.img.word(f + 0x6b), if c == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(f + 0x7f), node8);
        },
    ),
    (
        "effect_record",
        &[(307, 0x24c0), (605, 0x7e40)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x08), P);
            for op in [0x0e, 0x20, 0x29, 0x31, 0x38, 0x4a, 0x54, 0x7e] {
                fill(t, P + t.img.word(f + op) - 4, 3, c ^ op);
            }
        },
    ),
    (
        "deal_then_slots",
        &[(391, 0x2a20), (448, 0x6440)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x03), P);
            fill(t, P + t.img.word(f + 0x09), 0x10, c);
            t.set_u32(P + t.img.word(f + 0x14), if c == 0 { u32::MAX } else { 5 });
            for op in [0x1b, 0x21, 0x2b, 0x35] {
                fill(t, P + t.img.word(f + op), 1, c ^ op);
            }
            fill(t, t.img.word(f + 0x75), 1, c);
            // the deal: 4 draws from 3 kinds, 2 of each
            let d = t.img.rel32_target(f + 0x0e);
            t.set_u32(t.img.word(d + 0x17), 3);
            t.set_u32(t.img.word(d + 0x1e), 4);
            for i in 0..3 {
                t.set_u32(t.img.word(d + 0x46) + 0x1c * i, 2);
            }
            t.stub(324, |m, _| {
                let k = m.r32(SCRATCH + 0x710);
                m.w32(SCRATCH + 0x710, k + 1);
                k * 7 + 1
            });
        },
    ),
    (
        "record_at_node",
        &[(673, 0x1f10), (673, 0x1fa0)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x03), P);
            fill(t, P + t.img.word(f + 0x09), 8, c);
            let take = t.img.rel32_target(f + 0x42);
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
            for i in 1..=3 {
                t.write(node(i), &[0x5a; 0x1c]);
                t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
                t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
            }
            t.set_u32(free, node(1));
            t.set_u16(count, 3);
            let node8 = SCRATCH + 0x1200;
            t.write(node8, &[0x77; 0x1c]);
            t.set_u32(t.img.word(f + 0x48), if c == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(f + 0x60), node8);
        },
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(name, list, cases, set) in CASES {
        let label = format!("o4 fam4 {name}");
        for (n, rva) in copies(&label, list) {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            stubs(&mut t);
            let f = t.at(rva);
            for c in 0..cases {
                set(&mut t, f, c);
                let regs = Call {
                    eax: 0x00ea_0000 + c,
                    ecx: 0x00ec_0000 + rva,
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                };
                t.check_entry(f, regs, &[OBJ], Eax::Checked);
            }
        }
    }
}

#[test]
fn mesh_flags_in_every_copy() {
    for &(n, rva) in MESH_FLAGS {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let a = t.at(rva);
        let found = ffx_magic::INTERNAL
            .iter()
            .find(|(_, sig)| t.img.matches(a, sig))
            .map(|(n, _)| *n);
        assert!(found.is_some_and(|n| n.starts_with("o4 fam4 mesh_flags")));
        let kind = t.img.word(f + 0x49);
        let model = SCRATCH + 0x1000;
        let table = SCRATCH + 0x1100;
        let e = SCRATCH + 0x2000;
        // two meshes, the first empty; the second with three parts: the
        // kind and 0x48 (changed), the kind and another byte, another kind
        t.set_u16(model + 0xa, 2);
        t.set_u32(model + 0x18, table);
        t.set_u32(table, 0);
        t.set_u32(table + 4, e);
        t.set_u16(e + 6, 3);
        for (k, (kind_bits, byte)) in [(kind | 0xc000, 0x48u8), (kind, 0x47), (kind ^ 1, 0x48)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            let off = 0x40 + 0x40 * k;
            t.set_u16(e + 0x10 + 8 * k, off as u16);
            let part = e + off;
            t.set_u32(part + 4, 0x400);
            t.set_u32(part + 0x10, kind_bits);
            t.set_u32(part + 0x400, 0x1111_1111);
            t.set_u8(part + 0x404, byte);
        }
        t.check(f, Call::this(model), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(o4_fam4::mesh_flags(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(MESH_FLAGS);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 177
// mutants, 170 killed. The survivors are equivalent: an extra word for
// slot 562 (no arguments) and slot 626 (`push 0`), the pool node's `+0x4`
// cleared again after `take` cleared it (two places, two mutants each),
// and the deal's frame
// placed 16 bytes higher (its locals lie below the entry and are not
// handed to any call).
