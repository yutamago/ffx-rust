//! `magic_0205.dll` against its Rust ports (`ffx_magic::m0205`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0205;
use magic::{Call, Eax, SCRATCH, StubMem};
use magic_owna::{f, rec, row, twin};

const N: u16 = 205;

/// 0x10001ff0 (internal, `edx` = the matrix): two table indices, each
/// slot's float its own.
#[test]
fn scale_matrix() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1ff0);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 235, 1, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(1.0 + f32::from(u8::try_from(i).unwrap())));
        }
        0x235
    });
    for (s, base) in [(277u32, 0.5f32), (15, 1.25), (16, -2.0), (17, 0.75)] {
        rec(&mut t, s, 1, move |_, a| (0x0100_0000 + s) ^ a[0]);
        let calls = row(s);
        t.stub_st0(s, move |m, a| {
            let k = m.r32(calls);
            f64::from(base + 0.125 * f32::from(u16::try_from(k + (a[0] & 0xf)).unwrap()))
        });
    }
    let table = w(&t, 0x22);
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0a00_0000 + i);
    }
    let bp = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x17), bp);
    let mat = SCRATCH + 0x100;
    for b in [1u8, 3] {
        t.write(bp, &[0x44, b, 0x44, 0x44]);
        t.write(mat, &[0x5c; 0x40]);
        let c = Call {
            edx: mat,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, m0205::scale_matrix);
    }
}
