//! `magic_0160.dll` against its Rust port: `DllMain` against every bound
//! port, and the OPU start-up with the control block in the DLL's data
//! (with the shutdown). The runtime and the OPU host are checked in
//! `magic_0604.rs`. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

const N: u16 = 160;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn start_up_and_shutdown() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let (f, s) = (t.at(0x1660), t.at(0x1820));
    magic_l3::start_up(&mut t, f, 0x95, 11, Some(s));
}
