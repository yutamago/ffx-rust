//! `magic_0342.dll` against its Rust ports (`ffx_magic::m0342`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 342;

/// A moving bone for one case: its three objects' handler +0x1c (slot
/// 1001), the table entry (slot 32) and slots 233/325, each logging, so
/// `bone_plain`'s body (and its address) shows. Returns the bone node.
fn moving_bone(t: &mut Twin, at: u32) -> u32 {
    let bone = t.img.word(at + 0x92);
    let bvt = SCRATCH + 0x880;
    t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
    t.set_u32(bvt + 0x1c, magic::TRAPS + 16 * 1001);
    for (k, s) in (0u32..).zip([1001u32, 32, 233, 325]) {
        let row = SCRATCH + 0x3400 + 0x40 * k;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x3c).wrapping_add(1);
            m.w32(row + 0x3c, c);
            m.w32(row + 4 * (c % 8), a[0] ^ a.get(1).copied().unwrap_or(0));
            if s == 233 {
                // the vector slot 233 fills (dest, src, rate)
                for i in 0..4 {
                    m.w32(a[0] + 4 * i, a[2] ^ (0x8000_0000 + i));
                }
            }
            0x0400_0000 + s
        });
    }
    let bt = t.img.rel32_target(at + 0x9b);
    let table = t.img.word(bt + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    for (i, off) in (0u32..).zip([0x13cu32, 0x144, 0x140]) {
        let o = SCRATCH + 0x5000 + 0x100 * i;
        t.write(o, &[0x40 + u8::try_from(i).unwrap(); 0x100]);
        t.set_u32(o, bvt);
        t.set_u32(bone + off, o);
    }
    t.set_u32(bone + 0x128, 0);
    t.set_u8(bone + 0x148, 1);
    bone
}

/// 0x10007440: the summon's frame before its phase, with and without
/// children, the flag set and clear, the done bit and the closing slot
/// each way.
#[test]
fn summon_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7440);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let bind = t.img.rel32_target(at + 0x22);
    t.set_u32(t.img.word(bind + 0x7e), SCRATCH + 0x500);
    for (k, s) in (0u32..).zip([723u32, 724, 230, 253, 214]) {
        let row = SCRATCH + 0x3000 + 0x40 * k;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x3c).wrapping_add(1);
            m.w32(row + 0x3c, c);
            if s == 253 {
                return m.r32(SCRATCH + 0x37f0);
            }
            m.w32(row + 4 * (c % 8), a[0]);
            0x0300_0000 + s
        });
    }
    // the children's method +0x20: slot 1000, logging its cursor's node
    // and emptying the root's list on the last child
    let first = w(&t, 0x2d);
    let root = w(&t, 0x43);
    t.stub(1000, move |m, a| {
        let k = m.r32(SCRATCH + 0x37fc).wrapping_add(1);
        m.w32(SCRATCH + 0x37fc, k);
        m.w32(SCRATCH + 0x3780 + 4 * (k % 16), m.r32(a[0]));
        if m.r32(m.r32(a[0]) + 8) == root {
            m.w32(first, root);
        }
        0x1000
    });
    let vt = SCRATCH + 0x800;
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 1000);
    let bone = moving_bone(&mut t, at);
    t.set_u32(w(&t, 0x18), 0x0018_0018);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    for (k, (phase, flag, kids, done0, half, pass)) in (0u32..).zip([
        (3u32, 0u32, 0u32, 0u32, 0u16, 0x41u8),
        (4, 0, 2, 0, 0, 0x41),
        (5, 0, 0, 0, 0, 0x41),
        (5, 1, 2, 0, 0, 0x41),
        (5, 1, 0, 0, 1, 0x41),
        (6, 0, 1, 1, 0, 0xff),
        (7, 1, 1, 1, 0, 0x41),
        (5, 1, 0, 0, 0, 0x52),
    ]) {
        t.set_u32(w(&t, 0x0b), phase);
        t.set_u32(w(&t, 0x35), flag);
        t.set_u32(SCRATCH + 0x37f0, u32::from(flag == 0));
        t.set_u32(w(&t, 0x85), 0x10 | done0);
        t.set_u16(w(&t, 0xeb), half);
        t.set_u32(w(&t, 0xf9), 0x00f9_0000);
        t.set_u8(w(&t, 0x27), pass);
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { root } else { kid(i + 1) });
        }
        t.set_u32(first, if kids == 0 { root } else { kid(0) });
        t.set_u32(bone + 0x10, bone);
        // the last case's bone moves (its pass byte not the cursor's)
        let moving = k == 7;
        t.set_u8(bone + 0x1a, pass.wrapping_add(if moving { 9 } else { 1 }));
        for node in [w(&t, 0xb3), w(&t, 0xd3)] {
            t.set_u32(node + 0x10, node);
            t.set_u8(node + 0x1a, 0);
            t.set_u8(node + 0x1b, 0);
        }
        let c = Call {
            eax: 0x00ea_0000 + phase,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}
