//! `magic_0171.dll` against its Rust port (`ffx_magic::m0171`): the
//! actor block and the aim angles (both internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0171};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, rec, twin};

const N: u16 = 171;

/// 0x10002a00: in and out of a battle.
#[test]
fn actor_block() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2a00);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 215, 1, |_, a| a[0] ^ 0x215);
    rec(&mut t, 413, 1, |_, a| a[0] ^ 0x413);
    t.set_u32(w(&t, 0x13), 0x00e0_0001);
    let block = SCRATCH + 0x100;
    for (b1, b2) in [(0u32, 0u32), (1, 1), (0, 1), (1, 0)] {
        t.write(block, &[0x5a; 0x20]);
        t.set_u32(w(&t, 0x1c), b1);
        t.set_u32(w(&t, 0x57), b2);
        let c = Call {
            edx: block,
            ..Call::default()
        };
        t.check(at, c, &[0x00aa_0000], Eax::Checked, |g, img, fr| {
            let a = fr.arg(g, 0);
            Ret::eax(m0171::actor_block(g, img, fr.at, fr.edx, a))
        });
    }
}

/// 0x100039a0: the angles for points all around, the yaw above, inside
/// and below its range.
#[test]
fn aim() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x39a0);
    let (out, from, to) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for (a, b) in [
        ([0.0f32, 0.0, 0.0], [1.0f32, 2.0, 3.0]),
        ([5.0, -1.0, 2.0], [-3.0, 4.0, -7.5]),
        ([0.0, 0.0, 0.0], [-1.0, 0.5, 0.001]),
        ([2.0, 2.0, 2.0], [2.0, 9.0, -6.0]),
        ([1.0, 0.0, 1.0], [0.5, 0.0, 9.0]),
        ([0.0, 0.0, 0.0], [-4.0, 1.0, -0.25]),
    ] {
        for i in 0..3u32 {
            t.set_u32(from + 4 * i, f(a[i as usize]));
            t.set_u32(to + 4 * i, f(b[i as usize]));
        }
        t.write(out, &[0x77; 0x10]);
        let c = Call {
            ecx: out,
            edx: from,
            ..Call::default()
        };
        t.check(at, c, &[to], Eax::Junk, |g, img, fr| {
            let to = fr.arg(g, 0);
            Ret::eax(m0171::aim(g, img, fr.at, fr.ecx, fr.edx, to))
        });
    }
}
