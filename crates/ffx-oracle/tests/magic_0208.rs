//! `magic_0208.dll` against its Rust port: list 5's OPU start-up
//! (`opu boot a`), shutdown and the part init-data callback with mode 1
//! (the runtime and the rest of the OPU host are `magic_0604.rs`'s).
//! Needs `FFX_GAME_DIR`.

#[path = "magic/l5.rs"]
mod l5;
mod magic;

const N: u16 = 208;

#[test]
fn boot_a() {
    let Some(mut t) = l5::twin(N) else { return };
    let f = t.at(0x1690);
    l5::boot_cases(&mut t, f, (0x1b, 0x21, 0x3b, 0x43));
}

#[test]
fn shutdown() {
    let Some(mut t) = l5::twin(N) else { return };
    let f = t.at(0x10d0);
    l5::shutdown_cases(&mut t, f);
}

#[test]
fn init_data_mode_1() {
    let Some(mut t) = l5::twin(N) else { return };
    let f = t.at(0x1730);
    l5::init_data_cases(&mut t, f);
}
