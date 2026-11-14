//! M5 finish, WP5 part 7 (wp5h): the developers' save loader
//! `TkDebugLoadViewerSave` (`ffx_system::wp5h_devsave`; parity test in
//! `crates/ffx-oracle/tests/wp5h.rs`), which the harness's `devsave`
//! command calls on the main thread. Its file reader `debugLoadForViewer`
//! (`0x00649040`) has no line here: the harness puts its archive reader
//! there (`crate::devsave` in the hook, `devsave.rs` in `ffx-host`, which
//! runs the port `debug_load_for_viewer` without the harness), and the
//! game reaches it only from that loader.

use super::{call_cdecl, mem, ported};
use ffx_system::wp5h_devsave::bridge as dv;

fn cdecl<M>(_: &mut M, f: u32, args: &[u32]) -> u32 {
    call_cdecl(f, args)
}

ported! {
    /// `TkDebugLoadViewerSave(n)`: developer save `n` into `g_SaveData`
    /// and the map jump; `AtelSetEventJump`'s `eax`.
    TkDebugLoadViewerSave @ 0x008b_55e0 => extern "C" fn tk_debug_load_viewer_save(n: u32) -> u32 {
        dv::tk_debug_load_viewer_save(&mut mem(), n, &mut cdecl)
    }
}
