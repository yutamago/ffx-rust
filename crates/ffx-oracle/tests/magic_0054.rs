//! `magic_0054.dll` against its Rust port (`ffx_magic::m0054`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0054};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 54;

#[test]
fn init_ppp_runs_init_data_then_init_env() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let f = t.at(0x19e0);
    // the stubs record every word they see and what the flag was
    let lock = t.u32(t.img.word(f + 0x01));
    t.set_u32(lock, 0x0bad);
    t.stub(711, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 4 * i, *w);
        }
        m.w32(SCRATCH + 0x10, m.r32(lock));
        0x7110
    });
    t.stub(710, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..7]) {
            m.w32(SCRATCH + 0x20 + 4 * i, *w);
        }
        m.w32(SCRATCH + 0x40, m.r32(lock));
        0x7100
    });
    t.set_u32(t.img.word(f + 0x07), 0x00da_7a00);
    t.set_u32(t.img.word(f + 0x11), 0x0000_0123);
    for off in [0x5a, 0x65, 0x6f, 0x79] {
        t.set_u32(t.img.word(f + off), 0xffff_0000 + off);
    }
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0054::init_ppp(g, img, fr.at))
    });
}
