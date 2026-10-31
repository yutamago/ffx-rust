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
