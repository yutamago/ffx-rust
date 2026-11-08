//! M5 stage 1, round 11 (`tail11`): the last game-code and engine tail
//! (`ffx_glue_other::l11_shp`, `ffx_system::l11_settings`, `ffx_system::l11_padmap`, `ffx_glue_other::l11_btlpos`, `ffx_menu::l11_refpos`, `ffx_menu::l11_btlproc`). Parity
//! tests in `h4x.rs` and `l11_settings.rs`.
//!
//! Threads: the effects' draw for the shapes' render data, the main thread
//! for the settings file. Frames are per call.

use super::{StackFrame, mem, ported};
use ffx_glue_other::l11_btlpos::bridge as btlpos;
use ffx_glue_other::l11_shp::bridge as shp;
use ffx_menu::l11_btlproc::bridge as btlproc;
use ffx_menu::l11_refpos::bridge as refpos;
use ffx_system::l11_padmap::bridge as padmap;
use ffx_system::l11_settings::bridge as settings;

ported! {
    /// `(lo, hi, shape, obj) -> 1/0`: the `OShapeVu1` shapes' render data; the quad's vertices the transforms see in a frame.
    updateVFXRenderDataShp_OShapeVu1 @ 0x0071_c0b0 => extern "C" fn update_shp_oshape_vu1(lo: u32, hi: u32, shape: u32, obj: u32) -> u32 {
        let mut frame = StackFrame::<{ (shp::SHP_FRAME / 4) as usize }>::new();
        let f = frame.at();
        shp::update_shp_oshape_vu1(&mut mem(), f, lo, hi, shape, obj)
    }
    /// `(this; text) -> 0`: the settings file's words into the settings; a short string's buffer in a frame.
    Settings_Parse @ 0x0040_1e00 => extern "thiscall" fn settings_parse(this: u32, text: u32) -> u32 {
        let mut frame = StackFrame::<{ (settings::PARSE_FRAME / 4) as usize }>::new();
        let f = frame.at();
        settings::parse(&mut mem(), f, this, text)
    }
    /// `(this; buf) -> length`: the settings file's text into `buf` (none for 0).
    Settings_Format @ 0x0040_22f0 => extern "thiscall" fn settings_format(this: u32, buf: u32) -> u32 {
        settings::format(&mut mem(), this, buf)
    }
    /// `(manager) -> the last pair`: the key bindings paired with their pad bits; the empty array it assigns from in a frame.
    Input_BuildPadMap @ 0x0063_15c0 => extern "thiscall" fn input_build_pad_map(manager: u32) -> u32 {
        let mut frame = StackFrame::<{ (padmap::PAD_MAP_FRAME / 4) as usize }>::new();
        let f = frame.at();
        padmap::build_pad_map(&mut mem(), f, manager)
    }
    /// `(name, out) -> eax as left`: the battle position table written out; the records `sceWrite` sees in a frame.
    MsBtlPosWriteBin @ 0x007c_a580 => extern "C" fn ms_btl_pos_write_bin(name: u32, out: u32) -> u32 {
        let mut frame = StackFrame::<{ (btlpos::FRAME / 4) as usize }>::new();
        let f = frame.at();
        btlpos::write_bin(&mut mem(), f, name, out)
    }
    /// `(window, msg, param) -> 0`: the battle debug "Reference" window; the polar form and the text buffer in a frame.
    Ms_Gui_ReferencePos @ 0x007d_9c20 => extern "C" fn ms_gui_reference_pos(window: u32, msg: u32, param: u32) -> u32 {
        let mut fr = StackFrame::<{ (refpos::FRAME / 4) as usize }>::new();
        let ebp = fr.at().wrapping_add(refpos::FRAME);
        refpos::reference_pos(&mut mem(), ebp, window, msg, param)
    }
    /// `(window, msg, param) -> 0`: the battle debug `BattlePos` window; its three text buffers in a frame.
    Ms_Gui_BattleProc @ 0x007d_80a0 => extern "C" fn ms_gui_battle_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut fr = StackFrame::<{ (btlproc::FRAME / 4) as usize }>::new();
        let ebp = fr.at().wrapping_add(btlproc::FRAME);
        btlproc::battle_proc(&mut mem(), ebp, window, msg, param)
    }
}
