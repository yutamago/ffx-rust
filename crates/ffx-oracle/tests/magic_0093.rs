//! `magic_0093.dll` against its Rust port (`ffx_magic::m0093`): `DllMain`
//! against every bound port and the state reset with the count 0x12
//! (internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0093};
use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 93;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn reset_state_12() {
    let Some(mut t) = twin(N) else { return };
    let obj = SCRATCH + 0x100;
    for arg in [0u32, 0x1000_2c30] {
        t.write(obj, &[0xa5; 0x130]);
        t.check(
            t.at(0x1b80),
            Call::this(obj),
            &[0x77, arg],
            Eax::Checked,
            |g, _, fr| {
                let arg = fr.arg(g, 1);
                Ret::eax(m0093::reset_state_12(g, fr.ecx, arg))
            },
        );
    }
}
