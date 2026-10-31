//! `magic_0076.dll` against its Rust port (`ffx_magic::m0076`): the
//! model start-up, and `magic_0077`'s copy. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, recs, twin};

/// A model start-up into an empty and a filled root list, with the
/// model flag byte clear and set and the ring's tail anywhere.
fn model_start_cases(n: u16, stored: &[u32]) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(0x6800);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    recs(&mut t, &[(268, 3), (719, 3), (413, 1), (215, 1), (14, 2)]);
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    let root = w(&t, 0xba);
    // the words it stores hold junk; junk under the entry too (the
    // records' bytes must be written)
    for &op in stored {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for (flag, filled) in [(0u8, false), (1, true), (0, true), (3, false)] {
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        t.set_u32(w(&t, 0x07), tail);
        t.write(w(&t, 0x42), &[flag, 0x40]);
        t.set_u32(w(&t, 0x7f), if filled { other } else { root });
        t.set_u32(root + 0x10, if filled { other } else { root });
        t.set_u16(w(&t, 0xdb), 4);
        t.set_u32(w(&t, 0xe8), 0x00e8_0000 | u32::from(flag));
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn model_start() {
    let common = [
        0x0c, 0x12, 0x1c, 0x26, 0x30, 0x48, 0x52, 0x71, 0x86, 0x8e, 0x98, 0xa2, 0xac, 0xb5, 0xc2,
        0xee, 0xf8,
    ];
    let mut s76 = common.to_vec();
    s76.extend([0x115, 0x15b, 0x162, 0x1a0, 0x1a7, 0x1b1, 0x1bb]);
    model_start_cases(76, &s76);
    let mut s77 = common.to_vec();
    s77.extend([0x110, 0x155, 0x15c, 0x19e, 0x1a5, 0x1af, 0x1b9]);
    model_start_cases(77, &s77);
}
