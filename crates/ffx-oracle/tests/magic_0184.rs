//! `magic_0184.dll` against its Rust port (`ffx_magic::m0184`): the
//! parts' release. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 184;

/// 0x10001900: four releases.
#[test]
fn release_four() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1900);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let q = SCRATCH + 0x100;
    rec(&mut t, 713, 1, move |m, a| m.r32(q) ^ a[0]);
    for op in [0x01, 0x1e, 0x3b, 0x58] {
        t.set_u32(w(&t, op), q);
    }
    t.set_u32(q, 0);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x10001390: the particle set-up (each slot call answers its own
/// value, so the four globals differ).
#[test]
fn part_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1390);
    for (s, n) in [(718u32, 1usize), (222, 4), (221, 4), (719, 3)] {
        rec(&mut t, s, n, move |m, a| {
            let k = m.r32(magic_owna::row(s));
            (0x0700_0000 + (s << 8) + k) ^ a[0]
        });
    }
    for op in [0x65, 0x80, 0x9e, 0xbc] {
        t.set_u32(t.img.word(at + op), 0x5a5a_5a5b);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
