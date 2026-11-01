//! `magic_0298.dll` against its Rust ports (`ffx_magic::m0298`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0298;
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 298;

/// 0x10001fe0 (internal: `ecx` = the bytes, `edx` = the count, the
/// list): slot 219 answering 0 or a model by byte and pass.
#[test]
fn models_to_list() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1fe0);
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(&mut t, 276, 2, |_, a| 0x0276_0000 ^ a[0]);
    // byte bit 0 (pass 0) and bit 1 (pass 1) say whether 219 has a model
    rec(&mut t, 219, 2, |_, a| {
        if a[0] & (1 << a[1]) != 0 {
            0x0219_0000 ^ a[0] ^ (a[1] << 8)
        } else {
            0
        }
    });
    let bytes = SCRATCH + 0x700;
    t.write(bytes, &[0, 1, 2, 3, 0x41]);
    for n in [0u32, 1, 5, 0xffff_ffff] {
        let c = Call {
            eax: 0x00ea_00ea,
            ecx: bytes,
            edx: n,
            ..Call::default()
        };
        t.check(at, c, &[0x00b1_0000], Eax::Checked, m0298::models_to_list);
    }
}
