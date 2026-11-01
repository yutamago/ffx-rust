//! `magic_0297.dll` against its Rust ports (`ffx_magic::m0297`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0297;
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 297;

/// 0x100031a0 (internal, `ecx` = the record): with `e` set and clear.
#[test]
fn motion_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x31a0);
    for (s, n) in [(363u32, 3usize), (357, 3), (356, 3), (362, 4), (704, 4)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let r = SCRATCH + 0x100;
    let obj = SCRATCH + 0x200;
    let inner = SCRATCH + 0x300;
    for e in [0u32, 0x00e0_0018] {
        t.write(obj, &[0x5c; 0xa0]);
        t.set_u32(obj + 0x88, inner);
        t.set_u32(inner + 0x54, 0x0054_0054);
        for (k, v) in (0u32..).zip([
            obj,
            0x00a0_0004,
            0x1234_5678,
            0xffff_ffc0,
            0x00d0_0010,
            0x14,
            e,
        ]) {
            t.set_u32(r + 4 * k, v);
        }
        t.check(at, Call::this(r), &[], Eax::Checked, m0297::motion_setup);
    }
}

/// 0x100073d0 (internal, `edx` = v): the flag bit set and clear, each
/// slot answering a value with a high half.
#[test]
fn mode_switch() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x73d0);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 30, 1, |_, a| 0x0030_0000 ^ a[0]);
    rec(&mut t, 36, 2, |_, a| 0xab36_0000 ^ a[1]);
    rec(&mut t, 228, 2, |_, a| 0xcd28_0000 ^ a[1]);
    t.set_u32(w(&t, 0x13), 0x00aa_0013);
    t.set_u32(w(&t, 0x4c), 0x00bb_004c);
    for flags in [0x0001u16, 0x0402, 0xfeff, 0x0003] {
        t.set_u16(w(&t, 0x02), flags);
        let c = Call {
            edx: 0x00ed_0000 + u32::from(flags),
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, m0297::mode_switch);
    }
}
