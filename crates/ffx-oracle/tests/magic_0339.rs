//! `magic_0339.dll` (map effects) against the Rust ports of its task
//! functions (`m0339.rs` and the shared ones of `rcef.rs`, `m0334.rs`,
//! `m0336.rs`). Needs `FFX_GAME_DIR`.

mod magic;

use magic::Twin;
use magic::map::{self, Var};

const N: u16 = 339;

#[test]
fn map_task_functions() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    map::stubs(&mut t);
    let functions = [
        (0x1000, Var::No),
        (0x1040, Var::No),
        (0x1090, Var::Plain(0x26)),
        (0x10f0, Var::Deref(4)),
        (0x1140, Var::No),
        (0x11c0, Var::Deref(4)),
        (0x1270, Var::Plain(0x39)),
    ];
    for (rva, var) in functions {
        map::check(&mut t, rva, var);
    }
}
