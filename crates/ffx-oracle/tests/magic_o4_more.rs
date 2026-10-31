//! The slot loops and node set-ups of `ffx_magic::o4_more` against every
//! copy in its own DLL, and `DllMain` of each of those DLLs. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_more};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x5000;
const OBJ: u32 = SCRATCH + 0x8000;

/// The pool of the take called at `call` (a rel32 field): three free
/// nodes, the word before the first one set.
fn pool(t: &mut Twin, call: u32) {
    let take = t.img.rel32_target(call);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    for i in 1..=3 {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
    }
    t.set_u32(node(1) - 4, 0x5a5a_5a5a);
    t.set_u32(free, node(1));
    t.set_u16(count, 3);
}

/// A set-up before each call: the twin, the function, the case.
type SetUp = fn(&mut Twin, u32, u32);

/// A port's cases: its label, copies, how many set-ups and the set-up.
type Case = (&'static str, &'static [(u16, u32)], u32, SetUp);

const CASES: &[Case] = &[
    ("o4 more slot_pair_loop", &[(165, 0x5490)], 2, |t, f, c| {
        fill(t, P, 4, c);
        t.set_u32(t.img.word(f + 0x21), P + 4 * c);
        // slot 772 answers in eax and edx
        t.stub(772, |_, a| a[1] ^ 0x772);
        t.stub_edx(772, |_, a| a[1] ^ 0x0772_0000);
        // slot 779 records the word it finds at `*B` (set to 1 for it)
        let b = t.img.word(f + 0x21);
        t.stub(779, move |m, a| {
            let seen = m.r32(m.r32(b));
            m.w32(P + 0x40 + 4 * (a[3] & 0xf), seen);
            0x779
        });
    }),
    (
        "o4 more object_list_node",
        &[(165, 0x2ff0)],
        2,
        |t, f, c| {
            pool(t, f + 0x17);
            fill(t, OBJ + 0x5ee0, 0x10, c);
            t.set_u32(t.img.word(f + 0x03), OBJ);
            t.set_u32(t.img.word(f + 0x1d), c * (P + 0x40));
            fill(t, P + 0x40, 4, c);
            t.set_u32(t.img.word(f + 0x34), P + 0x40);
        },
    ),
    (
        "o4 more reset_record_handle",
        &[(297, 0x8020)],
        2,
        |t, f, c| {
            fill(t, P, 0x70, c);
            t.set_u32(t.img.word(f + 0x01), P);
            t.set_u32(t.img.word(f + 0x2f), c * 0x1234);
        },
    ),
    ("o4 more arg_list_node", &[(270, 0x1b80)], 2, |t, f, c| {
        pool(t, f + 0x09);
        fill(t, OBJ, 0x10, c);
        t.set_u32(t.img.word(f + 0x0f), c * (P + 0x40));
        fill(t, P + 0x40, 4, c);
        t.set_u32(t.img.word(f + 0x28), P + 0x40);
    }),
    ("o4 more slot_each_flag", &[(301, 0x22d0)], 3, |t, f, c| {
        fill(t, P + 0x19c, 8, c);
        // no entries, three, a negative count
        t.set_u32(P + 0x19c, [0, 3, 0xffff_fffe][c as usize]);
        t.set_u32(t.img.word(f + 0x04), P);
        fill(t, OBJ, 1, c);
    }),
];

#[test]
fn entries_in_every_copy() {
    for &(label, list, cases, set) in CASES {
        let all = copies(label, list);
        assert!(
            std::env::var_os("FFX_GAME_DIR").is_none() || !all.is_empty(),
            "{label}"
        );
        for (n, rva) in all {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            for s in 0..1024u32 {
                t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
            }
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
fn float_ratio_half() {
    const LIST: &[(u16, u32)] = &[(519, 0x2750)];
    for &(n, rva) in LIST {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let found = ffx_magic::INTERNAL
            .iter()
            .find(|(_, sig)| t.img.matches(f, sig))
            .map(|(n, _)| *n);
        assert!(found.is_some_and(|n| n.starts_with("o4 more float_ratio_half")));
        t.stub(336, |_, a| a[0] ^ 0x336);
        for (c, x) in (0u32..).zip([7.75f32, -3.5, 1.0e12]) {
            t.stub_st0(333, move |_, a| f64::from(x) + f64::from(a[1] & 0xf));
            fill(&mut t, OBJ, 2, c);
            let regs = Call {
                ecx: OBJ,
                edx: 0x0ed0 + c,
                ..Call::default()
            };
            t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o4_more::float_ratio_half(
                    g, img, fr.at, fr.esp, fr.ecx, fr.edx,
                ))
            });
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let mut lists: Vec<&[(u16, u32)]> = CASES.iter().map(|c| c.1).collect();
    lists.push(&[(519, 0x2750)]);
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 90
// mutants, 85 killed. The survivors are equivalent: the pool's take has
// already set a new node's `+0x0` and `+0x4` to 0 and `+0x8` to itself
// (four stores here repeat that), and `float_ratio_half`'s float kept in
// its local below the entry stack, which nothing reads after the return.
