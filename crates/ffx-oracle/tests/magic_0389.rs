//! `magic_0389.dll` against its Rust ports (`ffx_magic::m0389`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 389;

/// 0x10001990 (internal, `ecx` = the record): without a parent, under a
/// parent with no child and one with a child.
#[test]
fn strip_record() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1990);
    let r = SCRATCH + 0x100;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x2000;
    let kid = SCRATCH + 0x2800;
    for case in 0..3 {
        t.write(node, &[0x5c; 0x80]);
        t.write(parent, &[0x3c; 0x40]);
        t.set_u16(parent + 0x18, 5);
        t.write(kid, &[0x2c; 0x40]);
        t.set_u32(parent + 0x10, if case == 2 { kid } else { parent });
        t.write(r, &[0x7a; 0x20]);
        t.set_u32(r, node);
        t.set_u32(r + 4, if case == 0 { 0 } else { parent });
        t.set_u32(r + 0x10, 0x00a0_0010);
        t.check(
            at,
            Call::this(r),
            &[],
            Eax::Checked,
            ffx_magic::m0389::strip_record,
        );
    }
}

/// 0x10006e30: the flag bit set and clear, each slot answering a value
/// with a high half.
#[test]
fn mode_switch() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6e30);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    magic_owna::rec(&mut t, 30, 1, |_, a| 0x0030_0000 ^ a[0]);
    magic_owna::rec(&mut t, 36, 2, |_, a| 0xab36_0000 ^ a[1]);
    magic_owna::rec(&mut t, 228, 2, |_, a| 0xcd28_0000 ^ a[1]);
    t.set_u32(w(&t, 0x10), 0x00aa_0010);
    t.set_u32(w(&t, 0x51), 0x00bb_0051);
    for flags in [0x0001u16, 0x0402, 0xfeff, 0x0003] {
        t.set_u16(w(&t, 0x02), flags);
        t.set_u32(w(&t, 0x6a), 0x5a5a_5a5b);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
