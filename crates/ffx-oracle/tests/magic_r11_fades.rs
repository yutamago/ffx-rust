//! Round 11's start-ups with an object and two fades
//! (`ffx_magic::r11_fades`) against each original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r11;

use ffx_magic::r11_fades;
use magic::{Call, Eax};
use magic_owna::twin;

/// `magic_0105` 0x10003130: the task list empty and not (twice each, the
/// pools' rings moving on).
#[test]
fn object_two_fades_0105() {
    let Some(mut t) = twin(105) else { return };
    let f = t.at(0x3130);
    for listed in [false, true, true, false] {
        magic_r11::object_two_fades_setup(&mut t, f, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            r11_fades::object_two_fades,
        );
    }
}
