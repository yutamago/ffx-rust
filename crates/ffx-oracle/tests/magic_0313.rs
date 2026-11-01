//! `magic_0313.dll` against its Rust ports (`ffx_magic::m0313`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, rec, twin};

const N: u16 = 313;

fn stubs(t: &mut Twin) {
    for (s, n) in [
        (444u32, 1usize),
        (445, 1),
        (440, 4),
        (450, 2),
        (451, 2),
        (449, 2),
        (442, 3),
        (443, 2),
        (446, 2),
    ] {
        rec(t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    rec(t, 482, 4, |_, a| 0x0482_0000 ^ a[2]);
    t.stub_st0(482, |_, a| {
        f64::from(f32::from_bits(a[0])) * 0.5 + f64::from(f32::from_bits(a[2]))
    });
}

/// 0x10001400, 0x100014a0, 0x10001570, 0x10001a40: the steps below and
/// at the count of 60, two list indices.
#[test]
fn object_steps() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t);
    let obj = SCRATCH + 0x1000;
    let ctx = SCRATCH + 0x2000;
    for k in 0..0x80 {
        t.set_u32(ctx + 0x180 + 4 * k, 0x0c00_0000 + k);
    }
    for rva in [0x1400u32, 0x14a0, 0x1570, 0x1a40] {
        let at = t.at(rva);
        let w = |t: &Twin, off: u32| t.img.word(at + off);
        if rva == 0x1a40 {
            t.set_u32(w(&t, 0x17), ctx);
        } else {
            t.set_u32(w(&t, if rva == 0x1400 { 0x42 } else { 0x40 }), f(0.5));
            if rva != 0x1400 {
                t.set_u32(w(&t, 0x32), f(-1.25));
            }
        }
        for (count, i) in [(0i32, 0u32), (58, 1), (59, 2), (70, 0)] {
            if rva != 0x1400 && rva != 0x1a40 {
                t.set_u32(w(&t, 0x08), count.cast_unsigned());
            }
            t.write(obj, &[0x5c; 0x140]);
            t.set_u32(obj + 0xac, i);
            t.set_u32(obj + 0x2c + 16 * i, 3);
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}
