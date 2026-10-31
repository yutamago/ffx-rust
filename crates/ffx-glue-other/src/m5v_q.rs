//! M5 stage 1, list `vfx`, round 5 batch 12: Iggy `GDraw` D3D11's tile
//! begin (the `0x01a85830` callback): the tile rectangle with its padding
//! clamped to the screen, the render target grown to it, the state reset
//! and both depth buffers cleared, the viewports and output bound.

use ffx_bridge::Global;

/// `g_GDrawState` (`[0x01a858b4]`).
pub const GDRAW: Global<u32> = Global::new(0x01a8_58b4);

// ---- callees ----

/// `GDraw_freeResources_00a36300(stats)`, `gdraw_resetState_00a3c280()`,
/// `FUN_00a3c350()` (the projection for the target),
/// `gdraw_setViewportForTarget_00a3cd30()`, `gdraw_bindOutput_00a3c570(stats)`
/// (cdecl).
pub const FREE_RESOURCES: u32 = 0x00a3_6300;
pub const RESET_STATE: u32 = 0x00a3_c280;
pub const SET_PROJECTION_FOR_TARGET: u32 = 0x00a3_c350;
pub const SET_VIEWPORT_FOR_TARGET: u32 = 0x00a3_cd30;
pub const BIND_OUTPUT: u32 = 0x00a3_c570;
/// `ID3D11DeviceContext::ClearDepthStencilView` (vtable byte offset).
pub const CLEAR_DEPTH_STENCIL: u32 = 0xd4;

pub mod bridge {
    use super::{
        BIND_OUTPUT, CLEAR_DEPTH_STENCIL, FREE_RESOURCES, GDRAW, RESET_STATE,
        SET_PROJECTION_FOR_TARGET, SET_VIEWPORT_FOR_TARGET,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `ClearDepthStencilView(view, depth and stencil, 1.0, 0)`.
    fn clear_depth(h: &mut impl Host, view: u32) {
        let ctx: u32 = h.read(GDRAW.get(h) + 4);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + CLEAR_DEPTH_STENCIL);
        h.call(f, Conv::Stdcall, &[ctx, view, 3, 1.0_f32.to_bits(), 0]);
    }

    /// `gdraw_tile_begin_00a3a820(x0, y0, x1, y1, pad, stats)` (stdcall)
    /// -> `gdraw_bindOutput_00a3c570`'s `eax`: the tile (`+0x6d8`/`+0x6dc`
    /// its size, `+0x6e8`/`+0x6ec` its origin) and its padded rectangle
    /// clamped to the screen (`+0x6d0`, `+0x6d4`; origin `+0x6f0`/`+0x6f4`,
    /// size `+0x6e0`/`+0x6e4`; no padding for a full-width strip from 0, 0
    /// that ends at the screen's bottom); a padded size past the render
    /// target (`+0x6c0`, `+0x6c4`) grows it and frees the resources; then
    /// the state reset, both depth buffers (`+0x784`, `+0x788`) cleared,
    /// the projection, viewport and output set.
    #[allow(clippy::too_many_arguments)]
    pub fn tile_begin(
        h: &mut impl Host,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        pad: u32,
        stats: u32,
    ) -> u32 {
        let s = GDRAW.get(h);
        let screen_w: u32 = h.read(s + 0x6d0);
        let pad = if x0 == 0 && y0 == 0 && x1 == screen_w && y1 == h.read::<u32>(s + 0x6d4) {
            0
        } else {
            pad
        };
        h.write(s + 0x6d8, x1.wrapping_sub(x0));
        h.write(s + 0x6dc, y1.wrapping_sub(y0));
        h.write(s + 0x6e8, x0);
        let a = x0.wrapping_sub(pad);
        h.write(s + 0x6ec, y0);
        let b = y0.wrapping_sub(pad);
        let ax = if a.cast_signed() <= 0 { 0 } else { a };
        let by = if b.cast_signed() <= 0 { 0 } else { b };
        h.write(s + 0x6f0, ax);
        h.write(s + 0x6f4, by);
        let mut ex = x1.wrapping_add(pad);
        let screen_w: u32 = h.read(s + 0x6d0);
        if ex.cast_signed() >= screen_w.cast_signed() {
            ex = screen_w;
        }
        let ex = ex.wrapping_sub(ax);
        h.write(s + 0x6e0, ex);
        let mut ey = y1.wrapping_add(pad);
        let screen_h: u32 = h.read(s + 0x6d4);
        if ey.cast_signed() >= screen_h.cast_signed() {
            ey = screen_h;
        }
        let ey = ey.wrapping_sub(by);
        h.write(s + 0x6e4, ey);
        let w: u32 = h.read(s + 0x6c0);
        if ex.cast_signed() > w.cast_signed()
            || ey.cast_signed() > h.read::<u32>(s + 0x6c4).cast_signed()
        {
            let nw = if ex.cast_signed() > w.cast_signed() {
                ex
            } else {
                w
            };
            let hh: u32 = h.read(s + 0x6c4);
            h.write(s + 0x6c0, nw);
            let nh = if ey.cast_signed() > hh.cast_signed() {
                ey
            } else {
                hh
            };
            h.write(s + 0x6c4, nh);
            cdecl(h, FREE_RESOURCES, &[stats]);
        }
        cdecl(h, RESET_STATE, &[]);
        let view: u32 = h.read(GDRAW.get(h) + 0x784);
        clear_depth(h, view);
        let view: u32 = h.read(GDRAW.get(h) + 0x788);
        if view != 0 {
            clear_depth(h, view);
        }
        cdecl(h, SET_PROJECTION_FOR_TARGET, &[]);
        cdecl(h, SET_VIEWPORT_FOR_TARGET, &[]);
        cdecl(h, BIND_OUTPUT, &[stats])
    }
}
