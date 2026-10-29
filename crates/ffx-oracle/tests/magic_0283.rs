//! `magic_0283.dll` against its Rust port: the OPU start-up variant
//! `opu boot c` (slot 173 after the registration). Needs `FFX_GAME_DIR`.

#[path = "magic/l5.rs"]
mod l5;
mod magic;

#[test]
fn boot_c() {
    let Some(mut t) = l5::twin(283) else { return };
    let f = t.at(0x1690);
    l5::boot_cases(&mut t, f, (0x1b, 0x21, 0x3b, 0x43));
}
