//! `magic_0046.dll` against its Rust port: the state reset with `+0x11c`
//! = 0 (`ffx_magic::m0032`). Needs `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0032};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 46;

#[test]
fn reset_state_writes_the_block() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let this = SCRATCH + 0x1000;
    t.write(this + 0xb0, &[0x5a; 0x90]);
    t.check(
        t.at(0x1a10),
        Call {
            eax: 0,
            ecx: this,
            edx: 0,
        },
        &[0x1111, 0x1000_2130],
        Eax::Junk,
        |g, _, fr| {
            let arg = fr.arg(g, 1);
            m0032::reset_state(g, fr.ecx, arg, 0, 0);
            Ret::default()
        },
    );
}
