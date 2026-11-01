//! `magic_0186.dll` against its Rust port (`ffx_magic::m0186`): the
//! cleared node (internal) and the integer motion handler. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0186};
use magic::{Call, Eax, SCRATCH, TRAPS};
use magic_owna::{rec, twin};

const N: u16 = 186;

/// 0x100056f0: under no parent, an empty parent and one with children.
#[test]
fn node_cleared() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x56f0);
    let (r, node, parent, last) = (SCRATCH, SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for (p, l) in [(0u32, 0u32), (parent, parent), (parent, last)] {
        t.write(node, &[0x5a; 0x48]);
        t.write(parent, &[0x3c; 0x20]);
        // the child count (u16 +0x18) differs from its neighbours
        t.set_u16(parent + 0x18, 5);
        t.write(last, &[0x77; 0x10]);
        t.set_u32(parent + 0x10, l);
        t.set_u32(r, node);
        t.set_u32(r + 4, p);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: r,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, _, fr| {
            Ret::eax(m0186::node_cleared(g, fr.ecx, fr.eax))
        });
    }
}

/// 0x10005770: the handler on a visited node, one without children and
/// one with two (each child's method logging its node).
#[test]
fn int_motion() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5770);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, TRAPS + 16 * slot);
    rec(&mut t, slot, 1, |m, a| m.r32(a[0]) ^ 0x1000);
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, TRAPS + 16 * slot);
    let (cur, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    let kid = |i: u32| SCRATCH + 0x1200 + 0x100 * i;
    for (kids, same) in [(0u32, true), (0, false), (2, false)] {
        t.write(node, &[0x22; 0x48]);
        for i in 0..9 {
            t.set_u32(node + 0x1c + 4 * i, 0x100 * i + 7);
        }
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        for i in 0..kids {
            t.write(kid(i), &[0x44; 0x20]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { node } else { kid(i + 1) });
        }
        t.set_u32(node + 0x10, if kids == 0 { node } else { kid(0) });
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        let c = Call {
            eax: 0x1234_5678,
            ecx: cur,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}

/// 0x10003c60: under no parent and under one with children.
#[test]
fn node_abc() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3c60);
    let (r, node, parent, last) = (SCRATCH, SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for (p, l) in [(0u32, 0u32), (parent, parent), (parent, last)] {
        t.write(node, &[0x5a; 0x60]);
        t.write(parent, &[0x3c; 0x20]);
        // the child count (u16 +0x18) differs from its neighbours
        t.set_u16(parent + 0x18, 5);
        t.write(last, &[0x77; 0x10]);
        t.set_u32(parent + 0x10, l);
        for (i, v) in (0u32..).zip([node, p, 0xa, 0xb, 0xc]) {
            t.set_u32(r + 4 * i, v);
        }
        t.check(at, Call::this(r), &[], Eax::Checked, |g, _, fr| {
            Ret::eax(m0186::node_abc(g, fr.ecx))
        });
    }
}

/// 0x10004db0 (internal, `ecx` = the record): a node without a parent,
/// under a parent with no children and under one with a child.
#[test]
fn model_record() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4db0);
    // slot 236 fills the matrix from the model and part words; 233
    // scales the row it gets in place (xor the factor); 325 records
    rec(&mut t, 236, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16 {
            m.w32(a[2] + 4 * i, a[0].rotate_left(i) ^ a[1]);
        }
        0x236
    });
    rec(&mut t, 233, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ a[2];
            m.w32(a[0] + 4 * i, v);
        }
        0x233
    });
    rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[0]);
    let r = SCRATCH + 0x100;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x2000;
    let kid = SCRATCH + 0x2800;
    for case in 0..3 {
        t.write(node, &[0x5c; 0x200]);
        t.write(parent, &[0x3c; 0x40]);
        // the child count, unlike its neighbours
        t.set_u16(parent + 0x18, 5);
        t.write(kid, &[0x2c; 0x40]);
        match case {
            1 => t.set_u32(parent + 0x10, parent),
            2 => t.set_u32(parent + 0x10, kid),
            _ => {}
        }
        let words = [
            node,
            if case == 0 { 0 } else { parent },
            0x00a0_0008,
            0x00b0_000c,
            0x00c0_0010,
            0x00d0_0014,
            0x00e0_0018,
            0x00f0_001c,
        ];
        for (i, v) in (0u32..).zip(words) {
            t.set_u32(r + 4 * i, v);
        }
        t.check(at, Call::this(r), &[], Eax::Checked, m0186::model_record);
    }
}
