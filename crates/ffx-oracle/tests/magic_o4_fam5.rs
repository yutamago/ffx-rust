//! The families of `ffx_magic::o4_fam5` against every DLL's own copy,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its name, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

/// Every slot answers a value of its first word; pppStartPart (714)
/// records the game lock it runs under.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(714, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x220, v);
        0x714
    });
}

const CASES: &[Case] = &[
    (
        "part_scale_by_index",
        &[(664, 0x1420), (665, 0x1610)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), SCRATCH + 0x200);
            t.set_u32(t.img.word(f + 0x10), SCRATCH + 0x300);
            t.set_u32(SCRATCH + 0x310, 0x00ab_cd00 ^ f);
            let list = SCRATCH + 0x400;
            fill(t, list, 16, f ^ c);
            t.stub(941, move |_, _| list);
            let wrapper = t.img.rel32_target(f + 0x36);
            t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
            t.set_u32(SCRATCH + 0x204, 9);
            // the object: the block's first (case 0) or its second
            let base = t.img.word(f + 0x3d);
            let obj = base + 0x80 * c;
            fill(t, obj + 0x3c, 0x14, c);
            t.set_u32(SCRATCH + 0x7f0, obj);
        },
    ),
    (
        "attach_mode32_par_wait",
        &[(421, 0x14d0), (535, 0x14b0)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P + t.img.word(f + 0x10), 1, c);
            for k in 0..8 {
                fill(t, OBJ + t.img.word(f + 0x47 + 10 * k), 1, c ^ k);
            }
            // the mode call moves the game object: the set-par reads it again
            let g = t.img.word(f + 0x04);
            t.stub(443, move |m, a| {
                m.w32(g, SCRATCH + 0x5000);
                a[0] ^ 0x443
            });
        },
    ),
    (
        "attach_mode_wait",
        &[(313, 0x1340), (313, 0x1630)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P + t.img.word(f + 0x10), 1, c);
            for insn in [0x42u32, 0x4c, 0x56, 0x60, 0x6a, 0x74, 0x80, 0x86] {
                fill(t, OBJ + t.img.word(f + insn + 2), 1, c ^ insn);
            }
            fill(t, t.img.word(f + 0x39), 1, c ^ 0x39);
        },
    ),
    (
        "part_shape",
        &[(665, 0x1810), (666, 0x17a0)],
        1,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), SCRATCH + 0x200);
            t.set_u32(t.img.word(f + 0x10), SCRATCH + 0x300);
            t.set_u32(SCRATCH + 0x310, 0x00ab_cd00 ^ f);
            let list = SCRATCH + 0x400;
            fill(t, list, 16, f ^ c);
            t.stub(941, move |_, _| list);
            let wrapper = t.img.rel32_target(f + 0x36);
            t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
            t.set_u32(SCRATCH + 0x204, 9);
            fill(t, OBJ + 0x3c, 0x14, c);
            fill(t, t.img.word(f + 0x4f), 1, c);
        },
    ),
    (
        "clear_three",
        &[(162, 0x47e0), (598, 0x7e30)],
        2,
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x07), P);
            let a = P + t.img.word(f + 0x10);
            t.set_u32(a, 2.5f32.to_bits());
            let first = P + t.img.word(f + 0x22);
            for k in 0..3 {
                fill(t, first + 0x130 * k + 0x120, 3, c ^ k);
                t.set_u16(first + 0x130 * k + 0x124, 0x8123);
            }
            t.set_u32(t.img.word(f + 0x51), SCRATCH + 0x200);
            fill(t, P + t.img.word(f + 0x86), 1, c);
            // the list: the second object held, alone (case 1), or none
            let list = t.img.word(f + 0x43);
            let node = SCRATCH + 0x3000;
            t.write(node, &[0; 0x10]);
            t.write(list, &[0; 0xc]);
            if c == 1 {
                t.set_u32(node + 8, first + 0x130);
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 8, node);
            }
        },
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(name, list, cases, set) in CASES {
        let label = format!("o4 fam5 {name}");
        for (n, rva) in copies(&label, list) {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            stubs(&mut t);
            let f = t.at(rva);
            for c in 0..cases {
                let regs = Call {
                    eax: 0x00ea_0000 + c,
                    ecx: 0x00ec_0000 + rva,
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                };
                t.set_u32(SCRATCH + 0x7f0, OBJ);
                set(&mut t, f, c);
                let obj = t.u32(SCRATCH + 0x7f0);
                t.check_entry(f, regs, &[obj], Eax::Checked);
            }
        }
    }
}

const FADE_RECORD: &[(u16, u32)] = &[(269, 0x3210), (325, 0x2250)];

#[test]
fn fade_record_in_every_copy() {
    use ffx_magic::{Ret, Xmm, o4_fam5};
    for &(n, rva) in FADE_RECORD {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let found = ffx_magic::INTERNAL
            .iter()
            .find(|(_, sig)| t.img.matches(f, sig))
            .map(|(n, _)| *n);
        assert!(found.is_some_and(|n| n.starts_with("o4 fam5 fade_record")));
        let rec = SCRATCH + 0x5000;
        for (c, (a, b)) in [(0.5f32, 2.0f32), (-1.0, 0.0), (f32::NAN, 3.5)]
            .into_iter()
            .enumerate()
        {
            let take = t.img.rel32_target(f + 0x7d);
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
            t.set_u32(t.img.word(f + 0x83), if c == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(f + 0x9b), node8);
            fill(&mut t, rec, 0x10, c as u32);
            let regs = Call::this(rec).with_xmm(&[Xmm::f32(0.0), Xmm::f32(a), Xmm::f32(b)]);
            t.check(f, regs, &[0x00a1_0000], Eax::Checked, |g, img, fr| {
                Ret::eax(o4_fam5::fade_record(g, img, fr))
            });
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(FADE_RECORD);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 125
// mutants, 122 killed, 1 that does not compile (`add` not imported). The
// 2 survivors are equivalent: `fade_record` clearing the pool node's
// `+0x4` that `take` has just cleared (the original repeats it).
