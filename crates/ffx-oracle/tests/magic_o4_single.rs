//! The single functions of `ffx_magic::o4_single` against their DLLs,
//! and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_single};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its name, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

/// Every slot answers a value of its first word; slot 853 walks a list
/// of three nodes (then 0).
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    t.stub(852, |m, _| {
        m.w32(SCRATCH + 0x700, 0);
        0x852
    });
    t.stub(853, |m, _| {
        let k = m.r32(SCRATCH + 0x700);
        m.w32(SCRATCH + 0x700, k + 1);
        if k < 3 {
            SCRATCH + 0x3000 + 0x10 * k
        } else {
            0
        }
    });
}

/// The globals at the operands `ops` filled.
fn globals(t: &mut Twin, f: u32, ops: &[u32], seed: u32) {
    for &op in ops {
        fill(t, t.img.word(f + op), 1, seed ^ op);
    }
}

const CASES: &[Case] = &[
    ("record_actor_slot", &[(270, 0x1c70)], 2, |t, f, c| {
        globals(t, f, &[0x08], c);
        t.set_u32(t.img.word(f + 0x21), P);
        fill(t, P, 2, c + 3);
        fill(t, OBJ + 0x2c, 3, c);
        t.set_u32(OBJ + 0x2c, if c == 0 { u32::MAX } else { 2 });
    }),
    ("tail_free", &[(203, 0x45f0)], 2, |t, f, c| {
        t.set_u32(OBJ, P);
        fill(t, P + 0x60, 6, c);
        t.set_u8(P + 0xf8, if c == 0 { 0 } else { 4 });
        t.set_u32(t.img.word(f + 0x12), SCRATCH + 0x200);
    }),
    ("free_by_index", &[(351, 0x1bf0)], 3, |t, f, c| {
        // record 1 of N = 2, its entry set (case 0) or not (1); N = 1 (2)
        t.set_u32(t.img.word(f + 0x07), if c == 2 { 1 } else { 2 });
        let table = t.img.word(f + 0x1e);
        fill(t, table, 2, 7);
        t.set_u32(table + 4, u32::from(c != 1));
        t.set_u32(t.img.word(f + 0x26), SCRATCH + 0x200);
        t.set_u32(SCRATCH + 0x7f0, t.img.word(f + 0x0f) + 0x80);
    }),
    ("find_same_id", &[(275, 0x4310)], 3, |t, f, c| {
        let me = SCRATCH + 0x3010;
        t.set_u32(t.img.word(f + 0x22), SCRATCH + 0x3100);
        t.set_u32(SCRATCH + 0x3100, me);
        for k in 0..3 {
            t.set_u16(SCRATCH + 0x3000 + 0x10 * k, 0x50 + k as u16);
        }
        // the id another node shares (case 1), only itself (0), none (2)
        t.set_u16(me, [0x51, 0x51, 0x99][c as usize]);
        if c == 1 {
            t.set_u16(SCRATCH + 0x3020, 0x51);
        }
    }),
    ("flag_pick_slot", &[(601, 0x8070)], 2, |t, f, c| {
        t.set_u8(t.img.word(f + 0x02), 0x10 | c as u8);
        globals(t, f, &[0x18, 0x2c], c);
    }),
    ("model_slots", &[(263, 0x6b90)], 2, |t, f, c| {
        t.set_u32(t.img.word(f + 0x03), P);
        fill(t, P + 0x4c, 0x14, c);
        t.set_u32(P + 0x90, 0x5a5a_5a50 | c);
    }),
    ("display_list", &[(598, 0x77a0)], 1, |t, f, c| {
        globals(t, f, &[0x2a, 0x31, 0x42], c);
    }),
    ("colour_then_run", &[(671, 0x1340)], 1, |t, f, _| {
        let ro = t.img.rel32_target(f + 0x42);
        let list = t.img.word(ro + 0x7c);
        t.set_u32(list + 8, 0);
    }),
    ("set_then_timeline", &[(540, 0x1080)], 2, |t, f, c| {
        t.set_u32(t.img.word(f + 0x02), SCRATCH + 0x200);
        globals(t, f, &[0x07], c);
        // the timeline: idle (case 0), or running with no events
        let this = t.img.word(f + 0x0e);
        fill(t, this, 0x20, c);
        t.set_u32(this + 8, c);
        t.set_u32(this + 4, 0);
    }),
];

#[test]
fn entries_in_every_copy() {
    for &(name, list, cases, set) in CASES {
        let label = format!("o4 single {name}");
        for (n, rva) in copies(&label, list) {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            stubs(&mut t);
            let f = t.at(rva);
            for c in 0..cases {
                t.set_u32(SCRATCH + 0x7f0, OBJ);
                set(&mut t, f, c);
                let obj = t.u32(SCRATCH + 0x7f0);
                let regs = Call {
                    eax: 0x00ea_0000 + c,
                    ecx: 0x00ec_0000 + rva,
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                };
                t.check_entry(f, regs, &[obj], Eax::Checked);
            }
        }
    }
}

/// The copy at `rva` is the internal function `name`.
fn registered(t: &Twin, rva: u32, name: &str) {
    let a = t.at(rva);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(a, sig))
        .map(|(n, _)| *n);
    assert!(
        found.is_some_and(|n| n.starts_with(name)),
        "{a:#x}: {found:?}"
    );
}

#[test]
fn actor_slots() {
    let Some(mut t) = magic_l3::twin(171) else {
        return;
    };
    stubs(&mut t);
    registered(&t, 0x29c0, "o4 single actor_slot");
    let f = t.at(0x29c0);
    let a = t.img.word(f + 0x02);
    fill(&mut t, a, 1, 1);
    t.set_u32(t.img.word(f + 0x10), P);
    fill(&mut t, P, 2, 5);
    for i in [u32::MAX, 2] {
        t.check(f, Call::this(i), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(o4_single::actor_slot(g, img, fr.at, fr.ecx))
        });
    }
    // the first twin holds the harness lock: let it go first
    drop(t);
    let Some(mut t) = magic_l3::twin(298) else {
        return;
    };
    stubs(&mut t);
    registered(&t, 0x2250, "o4 single actor_slot_edx");
    let f = t.at(0x2250);
    let a = t.img.word(f + 0x06);
    fill(&mut t, a, 1, 1);
    t.set_u32(t.img.word(f + 0x1e), P);
    fill(&mut t, P, 2, 5);
    for i in [u32::MAX, 3] {
        let regs = Call {
            ecx: i,
            edx: 0x0ed0,
            ..Call::default()
        };
        t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(o4_single::actor_slot_edx(g, img, fr.at, fr.ecx, fr.edx))
        });
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(&[(171, 0x29c0), (298, 0x2250)]);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line): 99 mutants, all killed.
