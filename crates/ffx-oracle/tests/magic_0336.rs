//! `magic_0336.dll` (map effects) against the Rust ports of its task
//! functions (`m0336.rs` and the shared ones of `rcef.rs`, `m0334.rs`).
//! Needs `FFX_GAME_DIR`.

mod magic;

use magic::Twin;
use magic::map::{self, Var};

const N: u16 = 336;

#[test]
fn map_task_functions() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    map::stubs(&mut t);
    let functions = [
        (0x1000, Var::No),
        (0x1040, Var::No),
        (0x1090, Var::Plain(0xf)),
        (0x10e0, Var::Deref(4)),
        (0x1170, Var::Deref(4)),
        (0x1250, Var::Plain(0x39)),
    ];
    for (rva, var) in functions {
        map::check(&mut t, rva, var);
    }
}
