//! `magic_0084.dll` against its Rust port (`ffx_magic::m0084`): `DllMain`
//! against every bound port and the object start (internal). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, Xmm, m0084};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 84;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// Appended to an empty list, then to a list with a tail, and with the
/// pool empty; parts 0 and 2.
#[test]
fn start_object() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x26c0);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..4u32 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1110 + i);
    }
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(&mut t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(at + 0x9c);
    let free = t.img.word(pa + 0x04);
    let tail = t.img.word(at + 0xa7);
    let obj = SCRATCH + 0x1000;
    for (n, k, tl, index) in [
        (3u16, 0.5f32, 0u32, 0u32),
        (2, -2.0, SCRATCH + 0x600, 2),
        (0, 1.0, 0, 1),
    ] {
        t.write(obj, &[0x5a; 0x130]);
        t.set_u16(free, n);
        t.set_u32(tail, tl);
        let c = Call {
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(k)]);
        t.check(at, c, &[index], Eax::Checked, |g, img, fr| {
            let index = fr.arg(g, 0);
            Ret::eax(m0084::start_object(
                g,
                img,
                fr.at,
                fr.edx,
                fr.xmm_f32(3),
                index,
            ))
        });
    }
}
