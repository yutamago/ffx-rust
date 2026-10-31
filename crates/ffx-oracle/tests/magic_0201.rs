//! `magic_0201.dll` against its Rust port (`ffx_magic::m0201`): the
//! matrix node handler. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, TRAPS, Twin};
use magic_owna::{rec, twin};

const N: u16 = 201;

/// 0x100042e0: the handler on a visited node, one without children and
/// one with two.
#[test]
fn matrix_visit() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x42e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, TRAPS + 16 * slot);
    // the children's method logs the node its cursor holds
    rec(&mut t, slot, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x37fc).wrapping_add(1);
        m.w32(SCRATCH + 0x37fc, k);
        m.w32(SCRATCH + 0x3780 + 4 * (k % 16), m.r32(a[0]));
        m.r32(a[0]) ^ 0x1000
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, a[2] ^ i);
        }
        0x231
    });
    rec(&mut t, 697, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x3800 + 4 * i, v);
        }
        a[2] ^ 0x697
    });
    t.set_u32(w(&t, 0x85), SCRATCH + 0x700);
    t.set_u32(SCRATCH + 0x700, 0x0070_0000);
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, TRAPS + 16 * slot);
    let (cur, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    let kid = |i: u32| SCRATCH + 0x1200 + 0x100 * i;
    for (kids, same) in [(0u32, true), (0, false), (2, false)] {
        t.write(node, &[0x22; 0x40]);
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u32(node + 0x1c, SCRATCH + 0x2000);
        t.set_u32(node + 0x20, 0x0020_0000);
        for i in 0..kids {
            t.write(kid(i), &[0x44; 0x20]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { node } else { kid(i + 1) });
        }
        t.set_u32(node + 0x10, if kids == 0 { node } else { kid(0) });
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        t.check_entry(at, Call::this(cur), &[cur], Eax::Checked);
    }
}
