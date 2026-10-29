//! The shared-4 list's functions without a family (`ffx_magic::s4_misc`)
//! against the DLLs they were ported from. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

mod magic;

use ffx_magic::{Ret, Xmm, s4_misc};
use magic::{Call, Eax, SCRATCH, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The object start with a part index (internal: `edx` = the object,
/// `xmm3` the scale, the stack `(index, owner, -, owner_b)`): appended to
/// an empty list, to a list with a tail, and with the pool empty.
#[test]
fn start_object_at() {
    for (n, rva) in [(18u16, 0x3a10u32), (446, 0x8090)] {
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
        let pa = t.img.rel32_target(at + 0x9d);
        let free = t.img.word(pa + 0x04);
        let tail = t.img.word(at + 0xa8);
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
            .with_xmm(&[
                Xmm::f32(0.0),
                Xmm::f32(0.0),
                Xmm::f32(0.0),
                Xmm::f32(scale),
            ]);
            let args = [k as u32 * 2 + 1, 0x0a00 + k as u32, 0xdead, 0x0b00 + k as u32];
            t.check(at, c, &args, Eax::Checked, |g, img, fr| {
                Ret::eax(s4_misc::start_object_at(
                    g,
                    img,
                    fr.at,
                    fr.edx,
                    fr.xmm_f32(3),
                    fr.arg(g, 0),
                    fr.arg(g, 1),
                    fr.arg(g, 3),
                ))
            });
        }
    }
}
