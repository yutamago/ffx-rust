//! Round 9's shared own-row functions (`ffx_magic::r9_misc`) against each
//! copy's original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, Xmm, r9_misc};
use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

/// `magic_0447` 0x10003320 and `magic_0100` 0x100021a0 (internal: `edx`
/// = the object, `xmm3` the scale, the stack `(index)`): appended to an
/// empty list, to a list with a tail, and with the pool empty.
#[test]
fn start_object_state() {
    for (n, rva) in [(447u16, 0x3320u32), (100, 0x21a0)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let pa = t.img.rel32_target(at + 0x9c);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = t.img.word(at + 0xa7);
        let obj = SCRATCH + 0x1000;
        for (k, (count, scale, tl)) in [
            (3u16, 0.5f32, 0u32),
            (2, -2.0, SCRATCH + 0x600),
            (0, 1.0, 0),
            (1, f32::NAN, SCRATCH + 0x700),
        ]
        .into_iter()
        .enumerate()
        {
            t.write(obj, &[0x5a; 0x130]);
            t.set_u16(free, count);
            t.set_u32(tail, tl);
            let c = Call {
                edx: obj,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(scale)]);
            let k = u32::try_from(k).unwrap_or(0);
            t.check(
                at,
                c,
                &[k * 2 + 1, 0x0a00 + k],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(r9_misc::start_object_state(
                        g,
                        img,
                        fr.at,
                        fr.edx,
                        fr.xmm_f32(3),
                        fr.arg(g, 0),
                    ))
                },
            );
        }
    }
}
