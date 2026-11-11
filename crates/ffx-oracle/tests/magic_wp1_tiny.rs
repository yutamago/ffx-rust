//! `ffx_magic::wp1_tiny::set_global` (the callbacks the overlay-bind
//! test found unbound, m5-wp1-bind) against every DLL's own copy. The
//! tail thunks (`wp1_tiny::tail_thunk`) are checked with their targets'
//! cases (`glow_0165` in `magic_r36.rs`, `camera_rigs_steps` in
//! `magic_s8_misc.rs`). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use magic::{Call, Eax};

/// `mov dword [G], imm32; ret`: each copy, with `G` holding junk first,
/// and `eax` passed through.
#[test]
fn set_global_copies() {
    for (n, rva, name) in [
        (249u16, 0x1ed0u32, "wp1 set global (1)"),
        (562, 0x1ee0, "wp1 set global (1)"),
        (443, 0x1200, "wp1 set global (0)"),
        (530, 0x1000, "wp1 set global (0x80)"),
        (531, 0x1000, "wp1 set global (0x80)"),
    ] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let port = ffx_magic::bind(&t.img, f).map(|i| ffx_magic::PORTS[i].0);
        assert_eq!(port, Some(name), "magic_{n:04} {f:#x}");
        let g = t.img.word(f + 2);
        for (k, junk) in [0x5a5a_1234u32, 0xffff_ffff, 0x80].into_iter().enumerate() {
            let k = u32::try_from(k).unwrap();
            t.set_u32(g, junk);
            t.check_entry(
                f,
                Call {
                    eax: 0x00ea_0000 + k,
                    ecx: 0x00ec_0000 + rva,
                    edx: 0x00ed_0000 + rva,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
            );
        }
    }
}
