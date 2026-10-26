//! `ffx-glue-other`, second agent of list `h4-glue-other` (h4b): the
//! addresses at `0x00800000` and up (the PS2 GS layer, the battle and menu
//! drawers, the debug print).
//!
//! Threads: all of these run on the game thread (the battle and menu
//! drawers, the GS layer's frame code). Frames the originals hand callees
//! are `StackFrame`s; nothing is kept between calls.

use ffx_glue_other::{h4b_a as ba, h4b_b as bb};

use super::{StackFrame, mem, ported};

ported! {
    // ---- batch 1 (ffx_glue_other::h4b_a)

    /// `TODrawChangeMacro2(window, code, pen)`; the pen step's float in
    /// the first argument slot, as the original leaves it.
    TODrawChangeMacro2 @ 0x008b_5960 => extern "C" fn change_macro2(win: u32, code: u32, pen: u32; slots: u32) -> u32 {
        ba::change_macro2(&mut mem(), win, code, pen, Some(slots))
    }
    /// `TODrawBtlWindow()`.
    TODrawBtlWindow @ 0x0089_b360 => extern "C" fn draw_btl_window() {
        ba::draw_btl_window(&mut mem());
    }
    /// `Sg_MFIFOSync()`.
    Sg_MFIFOSync @ 0x0084_3360 => extern "C" fn mfifo_sync() -> u32 {
        ba::mfifo_sync(&mut mem())
    }
    /// `Sg_SetDispDrawEnv_008217d0(buffer)`; `buffer & 1` in the slot.
    Sg_SetDispDrawEnv_008217d0 @ 0x0082_17d0 => extern "C" fn set_disp_draw_env(buffer: u32; slots: u32) -> u32 {
        ba::set_disp_draw_env(&mut mem(), buffer, Some(slots))
    }
    /// `TOTransImageHL(dbp, dbw, psm, x, y, w, h)`: the DMA tag in a frame.
    TOTransImageHL @ 0x008b_0b10 => extern "C" fn trans_image_hl(dbp: u32, dbw: u32, psm: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        ba::trans_image_hl(&mut mem(), f.at(), dbp, dbw, psm, x, y, w, h)
    }
    /// `yiPrintGs_0090aaa0(text, x, y, cols, rows, center, &colour)`: the
    /// frame word `Sg_GetDBuffDC` fills; the pen in the slots.
    yiPrintGs_0090aaa0 @ 0x0090_aaa0 => extern "C" fn print_gs(text: u32, x: u32, y: u32, cols: u32, rows: u32, center: u32, colour: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        ba::print_gs(&mut mem(), f.at(), text, x, y, cols, rows, center, colour, Some(slots))
    }

    // ---- batch 2 (ffx_glue_other::h4b_b)

    /// `gdraw_makeTextureBegin_00a3a070(owner, w, h, format, flags, info,
    /// stats)`.
    gdraw_makeTextureBegin_00a3a070 @ 0x00a3_a070 => extern "stdcall" fn make_texture_begin(owner: u32, w: u32, h: u32, format: u32, flags: u32, info: u32, stats: u32) -> u32 {
        bb::make_texture_begin(&mut mem(), owner, w, h, format, flags, info, stats)
    }
    /// `gdraw_makeVertexBufferEnd_00a3a5f0(info, stats)`: the buffer
    /// descriptions in a frame (`ebp - 0x4c`).
    gdraw_makeVertexBufferEnd_00a3a5f0 @ 0x00a3_a5f0 => extern "stdcall" fn make_vertex_buffer_end(info: u32, _stats: u32) -> u32 {
        let mut f = StackFrame::<0x11>::new();
        bb::make_vertex_buffer_end(&mut mem(), f.at(), info)
    }
    /// `gdraw_createRowTexture_00a3a9f0(w, texels)`: the description in a
    /// frame (`ebp - 0x3c`).
    gdraw_createRowTexture_00a3a9f0 @ 0x00a3_a9f0 => extern "stdcall" fn create_row_texture(w: u32, texels: u32) -> u32 {
        let mut f = StackFrame::<0xe>::new();
        bb::create_row_texture(&mut mem(), f.at(), w, texels)
    }
    /// `gdraw_drawRectQuad_00a3bd40(rect, a)`: the mapping in a frame.
    gdraw_drawRectQuad_00a3bd40 @ 0x00a3_bd40 => extern "C" fn draw_rect_quad(rect: u32, a: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        bb::draw_rect_quad(&mut mem(), f.at(), rect, a)
    }
    /// `gdraw_drawTargetQuad_00a3bed0(a, depth, sub)`: the mapping in a
    /// frame; the floats in the third slot.
    gdraw_drawTargetQuad_00a3bed0 @ 0x00a3_bed0 => extern "C" fn draw_target_quad(a: u32, depth: u32, sub: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        bb::draw_target_quad(&mut mem(), f.at(), a, depth, sub, Some(slots))
    }
}
