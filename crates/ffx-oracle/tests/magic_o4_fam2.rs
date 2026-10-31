//! The families of `ffx_magic::o4_fam2` against every DLL's own copy,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_fam2};
use magic::map::{self, Var};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its name, copies, how many set-ups, the stack words
/// and the set-up.
type Case = (
    &'static str,
    &'static [(u16, u32)],
    u32,
    &'static [u32],
    SetUp,
);

const REGISTER_LOOP: &[(u16, u32)] = &[(171, 0x2500), (180, 0x1870), (392, 0x4830)];
const FLAG_SLOTS: &[(u16, u32)] = &[(202, 0x6d90), (317, 0x6a40), (597, 0x8e80)];

/// Every slot answers a value of its first word (`edx` too, for the
/// 64-bit handles); slot 288 writes the count it is handed a pointer to.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    t.stub(288, |m, a| {
        let n = m.r32(SCRATCH + 0x700);
        m.w32(a[1], n);
        SCRATCH + 0x4000
    });
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
    }
}

const CASES: &[Case] = &[
    (
        "field_slot_flag8",
        &[(356, 0x1840), (529, 0x1620)],
        1,
        &[OBJ],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            fill(t, P + t.img.word(f + 0x0c), 1, c);
            t.set_u32(OBJ, 0x1230);
        },
    ),
    (
        "scale_actors",
        &[(165, 0x33a0), (606, 0x7810)],
        3,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x05), [0u32, 1, 3][c as usize]);
            t.set_u32(t.img.word(f + 0x1d), P);
            for i in 0..4 {
                t.set_u8(P + i, (2 * i + 1) as u8);
            }
            fill(t, t.img.word(f + 0x28), 8, c);
        },
    ),
    (
        "slot_by_flag",
        &[(252, 0x1340), (303, 0x1420), (318, 0x1540)],
        3,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x02), u32::from(c == 1));
            t.set_u32(t.img.word(f + 0x0a), if c == 2 { 0x20 } else { 0x1f });
            globals(t, f, &[0x42], c);
        },
    ),
    (
        "seq_keep_447_32",
        &[(416, 0x1d20), (510, 0x1430), (525, 0x13b0)],
        2,
        &[OBJ],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x04), P);
            t.set_u32(P + t.img.word(f + 0x0c), 0x4470_0000 ^ f);
            t.set_u32(OBJ + 0xac, c + 1);
            fill(t, OBJ + 0x2c, 0x10, f ^ c);
        },
    ),
    (
        "byte_list_slot",
        &[(397, 0x63f0), (397, 0x6450)],
        3,
        &[],
        |t, f, c| {
            t.set_u32(SCRATCH + 0x700, [0u32, 4, 6][c as usize]);
            fill(t, SCRATCH + 0x4000, 2, c + 3);
            // the skipped byte: one of the list's
            t.set_u8(t.img.word(f + 0x1f), 0x77);
            t.set_u8(SCRATCH + 0x4002, 0x77);
        },
    ),
    (
        "task_list_seq_wait",
        &[(313, 0x1920), (313, 0x1980)],
        2,
        &[OBJ],
        |t, f, c| {
            globals(t, f, &[0x0f, 0x23], c);
            // the task the create answers (its first word ^ 434 << 20)
            let v = t.u32(t.img.word(f + 0x0f));
            let r = v ^ (434 << 20);
            t.set_u32(r + 0xac, c);
        },
    ),
    (
        "ptr_stores8",
        &[(307, 0x2990), (563, 0x2490)],
        1,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x01), P);
            for k in 0..8 {
                fill(t, P + t.img.word(f + 0x07 + 10 * k), 1, c ^ k);
            }
        },
    ),
    (
        "slot_task_seq_wait",
        &[(382, 0x10a0), (522, 0x1090)],
        1,
        &[OBJ],
        |t, f, c| {
            globals(t, f, &[0x1e], c);
        },
    ),
    (
        "store_node",
        &[(353, 0x2390), (672, 0x1cf0)],
        2,
        &[],
        |t, f, c| {
            t.set_u32(t.img.word(f + 0x01), P);
            fill(t, P + t.img.word(f + 0x0c) - 4, 3, c);
            let take = t.img.rel32_target(f + 0x15);
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
            t.set_u32(t.img.word(f + 0x1b), if c == 0 { 0 } else { node8 });
            t.set_u32(t.img.word(f + 0x2f), node8);
        },
    ),
];

#[test]
fn entries_in_every_copy() {
    for &(name, list, cases, args, set) in CASES {
        let label = format!("o4 fam2 {name}");
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
                t.check_entry(f, regs, args, Eax::Checked);
            }
        }
    }
}

/// The copies of `rcef::main_init_create_task` with other immediates.
#[test]
fn main_init_in_every_copy() {
    for (n, rva) in copies("o4 fam2 main init", &[(195, 0x19e0), (507, 0x10d0)]) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        map::stubs(&mut t);
        map::check(&mut t, rva, Var::Plain(0x39));
    }
}

/// The copy at `rva` is the internal function `name` registers.
fn registered(t: &Twin, rva: u32, name: &str) {
    let a = t.at(rva);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(a, sig))
        .map(|(n, _)| *n);
    let ok = found.is_some_and(|n| n.starts_with(name));
    assert!(ok, "{}: {a:#x} is {found:?}", t.name);
}

#[test]
fn internal_in_every_copy() {
    for &(n, rva) in REGISTER_LOOP {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        registered(&t, rva, "o4 fam2 register_loop");
        let f = t.at(rva);
        let lock = t.img.word(f + 0x21);
        t.set_u32(lock, SCRATCH + 0x800);
        rec_lock(&mut t);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        let this = OBJ + 0x40;
        t.check(f, Call::this(this), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(o4_fam2::register_loop(g, img, fr.at, fr.ecx))
        });
    }
    for &(n, rva) in FLAG_SLOTS {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        registered(&t, rva, "o4 fam2 flag_slots");
        let f = t.at(rva);
        let this = OBJ + 0x400;
        for c in 0..2u32 {
            fill(&mut t, this + 0x50, 8, c);
            t.set_u32(this + 0x90, 0x5a5a_0000 | c);
            // the first slot moves the model: its +0x54 is read again
            t.stub(30, move |m, a| {
                m.w32(this + 0x54, a[0] ^ 0x30);
                0x30
            });
            t.check(f, Call::this(this), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o4_fam2::flag_slots(g, img, fr.at, fr.ecx))
            });
        }
    }
}

/// Slot 779 records the game lock it runs under.
fn rec_lock(t: &mut Twin) {
    t.stub(779, |m, a| {
        let v = m.r32(SCRATCH + 0x800);
        let k = m.r32(SCRATCH + 0x900);
        m.w32(SCRATCH + 0x904 + 4 * (k % 0x40), v ^ a[0] ^ a[1]);
        m.w32(SCRATCH + 0x900, k + 1);
        0x779
    });
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(REGISTER_LOOP);
    lists.push(FLAG_SLOTS);
    lists.push(&[(195, 0x19e0), (507, 0x10d0)]);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line): o4_fam2 148 mutants, 144 killed; the generalised immediates
// of `rcef::main_init_create_task`: 3, 2 killed. The survivors are
// equivalent: an extra leading 0 for `slot_task_seq_wait`'s first slot
// (`push 0` in both copies), `store_node` clearing `+0x4` that the pool's
// `take` has just cleared (the original repeats it), the never-taken
// index -1 path of `scale_actors` (`cmp esi, -1` with `esi` counting up
// from 0) and `push 0x80000`, the same in every copy.
