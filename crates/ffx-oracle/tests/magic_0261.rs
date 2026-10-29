//! `magic_0261.dll` against its Rust port: the OPU start-up variant
//! `opu boot b`. Needs `FFX_GAME_DIR`.

#[path = "magic/l5.rs"]
mod l5;
mod magic;

#[test]
fn boot_b() {
    let Some(mut t) = l5::twin(261) else { return };
    let f = t.at(0x1690);
    l5::boot_cases(&mut t, f, (0x21, 0x1b, 0x3f, 0x4c));
}
