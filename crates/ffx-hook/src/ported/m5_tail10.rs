//! M5 stage 1, round 10 (`tail10`): the last game-code and engine tail
//! (`ffx_pother_a::l10_print`, `ffx_system::l10_assets`,
//! `ffx_pcui::l10_gdraw`, ...). Parity
//! tests in the lists' oracle files (`m5p_scripting.rs`,
//! `m5_system_b.rs`, `m5_pc_ui.rs`, ...).
//!
//! Threads: the script-running threads (main, the async init job) for
//! `print`; the scene loads (main, the async init job) for the asset
//! header; the render thread for the `GDraw` target end. Frames are per call
//! (the header's 0x47c bytes on the stack).

use super::{StackFrame, mem, ported};
use ffx_glue_other::l10_vfx::bridge as heap;
use ffx_pcui::l10_filter::bridge as filter;
use ffx_pcui::l10_gdraw::bridge as gdraw;
use ffx_pother_a::l10_print::bridge as print;
use ffx_system::l10_assets::bridge as assets;

ported! {
    /// `(L) -> 0`: Lua's `print`, its 0x200-byte buffer in a frame.
    Phyre_PScripting_Print @ 0x005d_5c00 => extern "C" fn phyre_pscripting_print(l: u32) -> u32 {
        let mut buf = StackFrame::<0x80>::new();
        let at = buf.at();
        print::print(&mut mem(), l, at)
    }
    /// `(reader; path)`: the asset list header into the reader; its strings and read buffer in a frame, the by-value string in another.
    AssetListReader_readAssetHeader @ 0x006f_4900 => extern "thiscall" fn read_asset_header(reader: u32, path: u32) {
        let mut frame = StackFrame::<0x11f>::new();
        let mut arg = StackFrame::<6>::new();
        let (f, a) = (frame.at(), arg.at());
        assets::read_asset_header(&mut mem(), reader, path, f, a);
    }
    /// `(r, x0, y0, x1, y1, stats)`: the end of a `GDraw` render target; the locals the callees see in a frame.
    gdraw_rt_end_00a39200 @ 0x00a3_9200 => extern "stdcall" fn gdraw_rt_end(r: u32, x0: u32, y0: u32, x1: u32, y1: u32, stats: u32) -> u32 {
        let mut frame = StackFrame::<0x38>::new();
        let f = frame.at();
        gdraw::rt_end(&mut mem(), f, [r, x0, y0, x1, y1, stats])
    }
    /// `(axis, api, target, obj, size, scale, rect, out, keep, c)`: the blur's one-axis set-up; its tap records in a frame.
    GDraw_filterAxisSetup_00a36590 @ 0x00a3_6590 => extern "C" fn gdraw_filter_axis_setup(axis: u32, api: u32, target: u32, obj: u32, size: u32, scale: u32, rect: u32, out: u32, keep: u32, c: u32) -> u32 {
        let mut taps = StackFrame::<0x24>::new();
        let at = taps.at();
        filter::axis_setup(&mut mem(), at, [axis, api, target, obj, size, scale, rect, out, keep, c])
    }
    /// `(used, size, mode)`: the effects' debug heap meter.
    opd_heap_display @ 0x00a6_3ff0 => extern "C" fn opd_heap_display(used: u32, size: u32, mode: u32) -> u32 {
        heap::heap_display(&mut mem(), used, size, mode)
    }
}
