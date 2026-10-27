//! `ffx-glue-other`, second agent of list `h4-glue-other` (h4b): the
//! addresses at `0x00800000` and up (the PS2 GS layer, the battle and menu
//! drawers, the debug print).
//!
//! Threads: all of these run on the game thread (the battle and menu
//! drawers, the GS layer's frame code). Frames the originals hand callees
//! are `StackFrame`s; nothing is kept between calls.

use ffx_glue_other::{
    h4b_a as ba, h4b_b as bb, h4b_c as bc, h4b_d as bd, h4b_e as be, h4b_f as bf, h4b_g as bg,
    h4b_h as bh, h4b_i as bi, h4b_j as bj, h4b_k as bk,
};

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
    /// descriptions in a frame (`ebp - 0x4c` .. `ebp - 4`, 18 words).
    gdraw_makeVertexBufferEnd_00a3a5f0 @ 0x00a3_a5f0 => extern "stdcall" fn make_vertex_buffer_end(info: u32, _stats: u32) -> u32 {
        let mut f = StackFrame::<0x12>::new();
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

    // ---- batch 3 (ffx_glue_other::h4b_c)

    /// `TkMn2DrawCrossCursor(x, y, kind)`: the screen size words in a
    /// frame; the temporaries in the slots.
    TkMn2DrawCrossCursor @ 0x008c_0670 => extern "C" fn cross_cursor(x: u32, y: u32, kind: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        bc::cross_cursor(&mut mem(), f.at(), x, y, kind, Some(slots))
    }
    /// `TODrawSelectCursor(window)`: the text and sizes in a frame
    /// (`ebp - 0x38` .. `ebp - 4`).
    TODrawSelectCursor @ 0x008b_5fb0 => extern "C" fn select_cursor(window: u32) -> u32 {
        let mut f = StackFrame::<0xd>::new();
        bc::select_cursor(&mut mem(), f.at(), window)
    }
    /// `TOBtlDrawTidusLimitWindow()`: the caption width in a frame word.
    TOBtlDrawTidusLimitWindow @ 0x0089_7240 => extern "C" fn tidus_limit_window() -> u32 {
        let mut f = StackFrame::<1>::new();
        bc::tidus_limit_window(&mut mem(), f.at())
    }
    /// `TOBtlDrawLuluLimitWindow()`: the caption width and text buffer in
    /// a frame (`ebp - 0x110` .. `ebp - 4`).
    TOBtlDrawLuluLimitWindow @ 0x0089_5660 => extern "C" fn lulu_limit_window() -> u32 {
        let mut f = StackFrame::<0x43>::new();
        bc::lulu_limit_window(&mut mem(), f.at())
    }
    /// `TOBtlDrawCrossLimitGauge(chr, x, y, w, h)`.
    TOBtlDrawCrossLimitGauge @ 0x0089_2910 => extern "C" fn cross_limit_gauge(chr: u32, x: u32, y: u32, w: u32, h: u32; slots: u32) {
        bc::cross_limit_gauge(&mut mem(), chr, x, y, w, h, Some(slots));
    }
    /// `TOBtlDrawStatusLimitGauge(gauge, x, y, w, h)`.
    TOBtlDrawStatusLimitGauge @ 0x0089_60e0 => extern "C" fn status_limit_gauge(gauge: u32, x: u32, y: u32, w: u32, h: u32; slots: u32) {
        bc::status_limit_gauge(&mut mem(), gauge, x, y, w, h, Some(slots));
    }
    /// `TkMn2DrawCrossLimitParam(x, y, w, h, value, max)`.
    TkMn2DrawCrossLimitParam @ 0x008d_3690 => extern "C" fn cross_limit_param(x: u32, y: u32, w: u32, h: u32, value: u32, max: u32; slots: u32) {
        bc::cross_limit_param(&mut mem(), x, y, w, h, value, max, Some(slots));
    }
    /// `TkBtlEndDrawLegendNotice_008bc100()`: the text size in a frame.
    TkBtlEndDrawLegendNotice_008bc100 @ 0x008b_c100 => extern "C" fn legend_notice() {
        let mut f = StackFrame::<2>::new();
        bc::legend_notice(&mut mem(), f.at());
    }
    /// `TkMn2DrawCrossCharFace(face, x, y, w, h)`: the image size and the
    /// record in a frame (`ebp - 0xb8` .. `ebp - 4`).
    TkMn2DrawCrossCharFace @ 0x008c_0250 => extern "C" fn cross_char_face(face: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        let mut f = StackFrame::<0x2d>::new();
        bc::cross_char_face(&mut mem(), f.at(), face, x, y, w, h)
    }
    /// `IggyMouseQueue_update_00a25ab0(this)`.
    IggyMouseQueue_update_00a25ab0 @ 0x00a2_5ab0 => extern "thiscall" fn mouse_queue_update(this: u32) {
        bc::mouse_queue_update(&mut mem(), this);
    }
    /// `IggyMenuManager_dispatchMouse_00a26ce0(this)`: the event and the
    /// result in a frame (`ebp - 0x28` .. `ebp - 4`).
    IggyMenuManager_dispatchMouse_00a26ce0 @ 0x00a2_6ce0 => extern "thiscall" fn dispatch_mouse(this: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        bc::dispatch_mouse(&mut mem(), f.at(), this)
    }

    // ---- batch 4 (ffx_glue_other::h4b_d)

    /// `gdraw_allocRenderTarget_00a3b610(stats)`: the texture description
    /// in a frame (`ebp - 0x30` .. `ebp - 4`, 11 words).
    gdraw_allocRenderTarget_00a3b610 @ 0x00a3_b610 => extern "C" fn alloc_render_target(stats: u32) -> u32 {
        let mut f = StackFrame::<0xb>::new();
        bd::alloc_render_target(&mut mem(), f.at(), stats)
    }
    /// `IggyMenuManager_init_00a262e0(this; device, w, h)`: the `GDraw`
    /// configuration in a frame (`ebp - 0x24` .. `ebp - 1`, 9 words).
    IggyMenuManager_init_00a262e0 @ 0x00a2_62e0 => extern "thiscall" fn menu_manager_init(this: u32, device: u32, w: u32, h: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        bd::menu_manager_init(&mut mem(), f.at(), this, device, w, h)
    }
    /// `TODrawBtlCtbParts(chr, count, x, y)`: the icon x and the count's
    /// temporaries in the slots.
    TODrawBtlCtbParts @ 0x008a_16d0 => extern "C" fn btl_ctb_parts(chr: u32, count: u32, x: u32, y: u32; slots: u32) {
        bd::btl_ctb_parts(&mut mem(), chr, count, x, y, Some(slots));
    }
    /// `TODrawChrFaceCell_008efd90(pos, face, x, y, c0, c1, c2, c3, mode)`:
    /// the image size and the record in a frame (`ebp - 0xb4` .. `ebp - 4`,
    /// 44 words); the column in the face slot.
    TODrawChrFaceCell_008efd90 @ 0x008e_fd90 => extern "C" fn chr_face_cell(pos: u32, face: u32, x: u32, y: u32, c0: u32, c1: u32, c2: u32, c3: u32, mode: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x2c>::new();
        bd::chr_face_cell(&mut mem(), f.at(), pos, face, x, y, [c0, c1, c2, c3], mode, Some(slots))
    }
    /// `gdraw_makeTextureEnd_00a3a200(info, stats)`: the description and
    /// the mip levels' data in a frame (`ebp - 0x150` .. `ebp - 4`, 83
    /// words).
    gdraw_makeTextureEnd_00a3a200 @ 0x00a3_a200 => extern "stdcall" fn make_texture_end(info: u32, stats: u32) -> u32 {
        let mut f = StackFrame::<0x53>::new();
        bd::make_texture_end(&mut mem(), f.at(), info, stats)
    }

    // ---- batch 5 (ffx_glue_other::h4b_e)

    /// `AbmapDrawScreenQuad_00a4b260()`: the screen size in a frame.
    AbmapDrawScreenQuad_00a4b260 @ 0x00a4_b260 => extern "C" fn screen_quad() {
        let mut f = StackFrame::<2>::new();
        be::screen_quad(&mut mem(), f.at());
    }
    /// `MenuConfig_DrawRowFrame_008cb530(x, y)`: the last edges in the
    /// slots.
    MenuConfig_DrawRowFrame_008cb530 @ 0x008c_b530 => extern "C" fn row_frame(x: u32, y: u32; slots: u32) -> u32 {
        be::row_frame(&mut mem(), x, y, Some(slots))
    }

    // ---- round 2, batch 1 (ffx_glue_other::h4b_f)

    /// `AbmapDrawGridTexture_00a4c460(level, size)`: the record and the
    /// name buffer in a frame (`ebp - 0x19c` .. `ebp - 4`, 102 words;
    /// zeroed, so another grid type's never-written name is empty).
    AbmapDrawGridTexture_00a4c460 @ 0x00a4_c460 => extern "C" fn grid_texture(level: u32, size: u32) -> u32 {
        let mut f = StackFrame::<0x66>::new();
        bf::grid_texture(&mut mem(), f.at(), level, size)
    }
    /// `AbmapDrawLangTitle_00a45ab0(_, x)`: the record and the name buffer
    /// in a frame (`ebp - 0x1ac` .. `ebp - 4`, 106 words).
    AbmapDrawLangTitle_00a45ab0 @ 0x00a4_5ab0 => extern "C" fn lang_title(_a: u32, x: u32) -> u32 {
        let mut f = StackFrame::<0x6a>::new();
        bf::lang_title(&mut mem(), f.at(), x)
    }
    /// `AbmapDrawPadArrow_00a4f290(item, top, x)`: the record and the name
    /// buffer in a frame (`ebp - 0x1ac` .. `ebp - 4`, 106 words; zeroed).
    AbmapDrawPadArrow_00a4f290 @ 0x00a4_f290 => extern "C" fn pad_arrow(item: u32, top: u32, x: u32) -> u32 {
        let mut f = StackFrame::<0x6a>::new();
        bf::pad_arrow(&mut mem(), f.at(), item, top, x)
    }
    /// `gdraw_textureDrawBufferBegin_00a3ac90(rect, _, format, keep,
    /// stats)`: the clear quad's rectangle in a frame.
    gdraw_textureDrawBufferBegin_00a3ac90 @ 0x00a3_ac90 => extern "stdcall" fn texture_draw_begin(rect: u32, _b: u32, format: u32, keep: u32, stats: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        bf::texture_draw_begin(&mut mem(), f.at(), rect, format, keep, stats)
    }
    /// `gdraw_filterPass_00a374e0(filter, rect, colour, stats)`: the
    /// rectangle, the null resource and the buffer word in a frame
    /// (`ebp - 0x24` .. `ebp - 4`, 9 words).
    gdraw_filterPass_00a374e0 @ 0x00a3_74e0 => extern "C" fn filter_pass(filter: u32, rect: u32, colour: u32, stats: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        bf::filter_pass(&mut mem(), f.at(), filter, rect, colour, stats)
    }
    /// `TOAdpCrossEasyStrFontSClutRGBA(text, x, y, a, r, g, b, alpha,
    /// scale, space)`: the converted text in a frame (`ebp - 0x118` up; 1
    /// KB, room for a long text's two-byte codes); `y` less the remap in
    /// its slot.
    TOAdpCrossEasyStrFontSClutRGBA @ 0x008e_c2f0 => extern "C" fn cross_str(text: u32, x: u32, y: u32, a: u32, r: u32, g: u32, b: u32, alpha: u32, scale: u32, space: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x100>::new();
        bf::cross_str(&mut mem(), f.at(), text, x, y, a, [r, g, b, alpha], scale, space, Some(slots))
    }

    // ---- round 2, batch 2 (ffx_glue_other::h4b_g)

    /// `TOMakePktShapeCursor(kind, x, y)`: the record and the image size in
    /// a frame (`ebp - 0xb8` .. `ebp - 4`, 45 words; zeroed, so another
    /// kind's never-written layout number is 0).
    TOMakePktShapeCursor @ 0x008f_eb00 => extern "C" fn shape_cursor(kind: u32, x: u32, y: u32) -> u32 {
        let mut f = StackFrame::<0x2d>::new();
        bg::shape_cursor(&mut mem(), f.at(), kind, x, y)
    }
    /// `yiAssert(abort, part, format, ...)`: the message buffer in a frame
    /// (`ebp - 0x101c` .. `ebp - 4`); the varargs after `format` in the
    /// caller's argument words.
    yiAssert @ 0x0090_ae50 => extern "C" fn yi_assert(abort: u32, part: u32, format: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x406>::new();
        bg::yi_assert(&mut mem(), f.at(), abort, part, format, slots.wrapping_add(12))
    }
    /// `gdraw_filterPass3_00a38c40(filter, rect, colour, kind, stats)`: the
    /// rectangle, the buffer word and the null views in a frame (`ebp -
    /// 0x2c` .. `ebp - 4`, 11 words).
    gdraw_filterPass3_00a38c40 @ 0x00a3_8c40 => extern "C" fn filter_pass3(filter: u32, rect: u32, colour: u32, kind: u32, stats: u32) -> u32 {
        let mut f = StackFrame::<0xb>::new();
        bg::filter_pass3(&mut mem(), f.at(), filter, rect, colour, kind, stats)
    }
    /// `gdraw_setRenderState_00a3c6c0(format, r, _, colour)`: the mapped
    /// results, the null view and the scissors in a frame (`ebp - 0x64` ..
    /// `ebp - 4`, 24 words).
    gdraw_setRenderState_00a3c6c0 @ 0x00a3_c6c0 => extern "C" fn set_render_state(format: u32, r: u32, _c: u32, colour: u32) -> u32 {
        let mut f = StackFrame::<0x18>::new();
        bg::set_render_state(&mut mem(), f.at(), format, r, colour)
    }

    // ---- round 2, batch 3 (ffx_glue_other::h4b_h)

    /// `TODrawMessageWaitNamesp(window)`: the temporaries and the Al Bhed
    /// text in a frame (`ebp - 0x14` .. `ebp - 1`, 5 words); the
    /// window's slot holds the original's temporaries.
    TODrawMessageWaitNamesp @ 0x008d_ee00 => extern "C" fn message_wait_namesp(window: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        bh::message_wait_namesp(&mut mem(), f.at(), window, Some(slots))
    }
    /// `AbmapDrawGridCells_00a497b0()`: the record and the name buffer in
    /// a frame (`ebp - 0x1a4` .. `ebp - 4`, 104 words).
    AbmapDrawGridCells_00a497b0 @ 0x00a4_97b0 => extern "C" fn grid_cells() -> u32 {
        let mut f = StackFrame::<0x68>::new();
        bh::grid_cells(&mut mem(), f.at())
    }
    /// `TODrawFrameShape9_008f9280(a, x, y, alpha)`: the contents' x and y
    /// and the last bar's x left in the argument slots.
    TODrawFrameShape9_008f9280 @ 0x008f_9280 => extern "C" fn frame_shape9(a: u32, x: u32, y: u32, alpha: u32; slots: u32) -> u32 {
        bh::frame_shape9(&mut mem(), a, x, y, alpha, Some(slots))
    }

    // ---- round 2, batch 4 (ffx_glue_other::h4b_i)

    /// `TODrawFrameShape_008f9c20(x, y, w, h, kind)`: the last piece's x,
    /// the bottom middle's and the bottom left's y left in the `x`, `w`
    /// and `kind` slots.
    TODrawFrameShape_008f9c20 @ 0x008f_9c20 => extern "C" fn frame_shape(x: u32, y: u32, w: u32, h: u32, kind: u32; slots: u32) -> u32 {
        bi::frame_shape(&mut mem(), x, y, w, h, kind, Some(slots))
    }

    // ---- round 3, batch 1 (ffx_glue_other::h4b_j)

    /// `TODrawWaitBtlWinSummon(w)`: the summon list's wait window.
    TODrawWaitBtlWinSummon @ 0x008a_75f0 => extern "C" fn wait_win_summon(w: u32) -> u32 {
        bj::wait_win_summon(&mut mem(), w)
    }
    /// `TODrawWaitBtlWinRight(w)`: the right command list's wait window;
    /// the opening plate's x left in the window's slot.
    TODrawWaitBtlWinRight @ 0x008a_6a60 => extern "C" fn wait_win_right(w: u32; slots: u32) -> u32 {
        bj::wait_win_right(&mut mem(), w, Some(slots))
    }

    // ---- round 3, batch 2 (ffx_glue_other::h4b_k)

    /// `TODrawWaitBtlWinLeft(w)`: the left command list's wait window;
    /// the opening plate's x left in the window's slot.
    TODrawWaitBtlWinLeft @ 0x008a_3680 => extern "C" fn wait_win_left(w: u32; slots: u32) -> u32 {
        bk::wait_win_left(&mut mem(), w, Some(slots))
    }
}
