//! `magic_0154.dll` against its Rust port: `DllMain` against every bound
//! port, and its own start-up (table `+0x0`). The runtime and the OPU
//! host are checked in `magic_0604.rs`. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

const N: u16 = 154;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn start_up() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x1690);
    magic_l3::start_up(&mut t, f, 0xab, 13, None);
}
