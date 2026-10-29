//! `magic_0279.dll` against its Rust port: the OPU start-up variant
//! `opu boot d` (set-up, then slot 816). Needs `FFX_GAME_DIR`.

#[path = "magic/l5.rs"]
mod l5;
mod magic;

#[test]
fn boot_d() {
    let Some(mut t) = l5::twin(279) else { return };
    let f = t.at(0x1660);
    l5::boot_cases(&mut t, f, (0x1b, 0x21, 0x3b, 0x43));
}
