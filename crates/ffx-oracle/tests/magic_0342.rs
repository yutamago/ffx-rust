//! `magic_0342.dll` against its Rust ports (`ffx_magic::m0342`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 342;

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
    t.set_u32(w(&t, 0x18), 0x0018_0018);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    for (phase, flag, kids, done0, half, pass) in [
        (3u32, 0u32, 0u32, 0u32, 0u16, 0x41u8),
        (4, 0, 2, 0, 0, 0x41),
        (5, 0, 0, 0, 0, 0x41),
        (5, 1, 2, 0, 0, 0x41),
        (5, 1, 0, 0, 1, 0x41),
        (6, 0, 1, 1, 0, 0xff),
        (7, 1, 1, 1, 0, 0x41),
    ] {
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
        let bone = w(&t, 0x92);
        t.set_u32(bone + 0x10, bone);
        t.set_u8(bone + 0x1a, pass.wrapping_add(1));
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
