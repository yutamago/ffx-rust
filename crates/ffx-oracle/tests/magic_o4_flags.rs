//! The flag steps and node constructor of `ffx_magic::o4_flags`
//! (internal functions) against their DLLs, and `DllMain` of each of
//! those DLLs. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{Ret, o4_flags};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{dlls, fill};

const M: u32 = SCRATCH + 0x2000;

/// The DLL `n` with every slot answering a value of its first word, and
/// the function at `rva` checked to be the internal function `name`.
fn twin(n: u16, rva: u32, name: &str) -> Option<(Twin, u32)> {
    let mut t = magic_l3::twin(n)?;
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
    let f = t.at(rva);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(f, sig))
        .map(|(n, _)| *n);
    assert!(
        found.is_some_and(|n| n.starts_with(name)),
        "{f:#x}: {found:?}"
    );
    Some((t, f))
}

#[test]
fn flag_steps() {
    let regs = Call {
        ecx: M,
        edx: 0x0ed0,
        ..Call::default()
    };
    if let Some((mut t, f)) = twin(203, 0x73c0, "o4 flags flag_step_arg") {
        for c in 0..2u32 {
            fill(&mut t, M + 0x50, 8, c);
            t.set_u32(M + 0x90, 0x5a5a_5a50 | c);
            t.check(f, regs, &[0x00a1_0000], Eax::Checked, |g, img, fr| {
                o4_flags::flag_step_arg(g, img, fr)
            });
        }
    }
    if let Some((mut t, f)) = twin(262, 0x6af0, "o4 flags flag_step_edx") {
        for c in 0..2u32 {
            fill(&mut t, M + 0x50, 8, c);
            t.set_u32(M + 0x90, 0x5a5a_5a50 | c);
            t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o4_flags::flag_step_edx(g, img, fr.at, fr.ecx, fr.edx))
            });
        }
    }
}

#[test]
fn object_head_node() {
    let Some((mut t, f)) = twin(203, 0x7940, "o4 flags object_head_node") else {
        return;
    };
    let node = SCRATCH + 0x7000;
    fill(&mut t, node, 0x10, 1);
    fill(&mut t, M, 0x20, 2);
    let head = SCRATCH + 0x5000;
    fill(&mut t, head, 4, 3);
    t.set_u32(t.img.word(f + 0x1f), head);
    let flags = t.img.word(f + 0x37);
    fill(&mut t, flags, 1, 4);
    let regs = Call {
        ecx: node,
        edx: M,
        ..Call::default()
    };
    t.check(f, regs, &[], Eax::Checked, |g, img, fr| {
        Ret::eax(o4_flags::object_head_node(g, img, fr.at, fr.ecx, fr.edx))
    });
}

#[test]
fn static_records() {
    let label = "o4 static ten_stores";
    if o4::copies(label, &[(303, 0x1730)]).is_empty()
        || o4::copies("o4 static record_two_node", &[(303, 0x48d0)]).is_empty()
    {
        return;
    }
    let Some(mut t) = magic_l3::twin(303) else {
        return;
    };
    let f = t.at(0x1730);
    for k in 0..10 {
        let a = t.img.word(f + 2 + 10 * k);
        fill(&mut t, a, 1, k);
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    let f = t.at(0x48d0);
    for c in 0..2u32 {
        let take = t.img.rel32_target(f + 0x22);
        let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
        let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
        for i in 1..=3 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
            t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
        }
        t.set_u32(free, node(1));
        t.set_u16(count, 3);
        // the word before the node, which a wrong `+0x4` would hit
        t.set_u32(node(1) - 4, 0x5a5a_5a5a);
        let node8 = SCRATCH + 0x1200;
        t.write(node8, &[0x77; 0x1c]);
        t.set_u32(t.img.word(f + 0x28), if c == 0 { 0 } else { node8 });
        t.set_u32(t.img.word(f + 0x43), node8);
        for op in [0x09, 0x13, 0x1d] {
            let a = t.img.word(f + op);
            fill(&mut t, a, 1, c ^ op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn object_set_ups() {
    let obj = SCRATCH + 0x3000;
    if let Some(mut t) = magic_l3::twin(316) {
        let f = t.at(0x16b0);
        assert_ne!(
            o4::copies("o4 static slot_by_flag_obj", &[(316, 0x16b0)]).len(),
            0
        );
        for s in 0..1024u32 {
            t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
        }
        for c in 0..3u32 {
            t.set_u32(t.img.word(f + 0x05), u32::from(c == 1));
            t.set_u32(t.img.word(f + 0x0d), if c == 2 { 0x20 } else { 0x1f });
            let a = t.img.word(f + 0x45);
            fill(&mut t, a, 1, c);
            fill(&mut t, obj + 0xfc, 0x10, c);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
        drop(t);
    }
    if let Some(mut t) = magic_l3::twin(380) {
        let f = t.at(0x1a20);
        assert_ne!(
            o4::copies("o4 static shape_local", &[(380, 0x1a20)]).len(),
            0
        );
        // slot 237 reads the local it is handed
        t.stub(237, |m, a| {
            let mut x = 0x237u32;
            for i in 0..4 {
                x = x.rotate_left(5) ^ m.r32(a[2] + 4 * i);
            }
            x
        });
        let h = t.img.word(f + 0x28);
        fill(&mut t, h, 1, 7);
        fill(&mut t, obj + 0xd0, 4, 8);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let dll_list = [
        (203, 0x73c0),
        (262, 0x6af0),
        (303, 0x1730),
        (316, 0x16b0),
        (380, 0x1a20),
    ];
    for n in dlls(&[&dll_list]) {
        magic_l3::dll_main(n);
    }
}

// Mutation testing (M5 magic round 4, tools/mutants.py, a few mutants per
// code line, survivors rerun after the tests were strengthened): 85
// mutants, 82 killed. The survivors are equivalent: `object_head_node`'s
// first `+0x8`/`+0x4` stores, which later stores replace (two), and
// `record_two_node`'s `+0x4` = 0, which the pool's take has already done.
