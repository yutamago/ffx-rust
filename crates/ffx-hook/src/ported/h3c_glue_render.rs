//! `ffx_prender::h3c_*`: the render-side glue's largest functions (M4,
//! list `h3-glue-render`, third agent h3c).
//!
//! Threads: the main thread (the menus' and the sphere grid's font
//! drawing); the shims keep no state.

use super::{mem, ported};
use ffx_prender::h3c_font::bridge as font;

ported! {
    // ---- batch 1 (ffx_prender::h3c_font) ----
    /// `graphicFontDraw(rec, name, x, y, slot)`: one glyph quad into the
    /// menu's (or the sphere grid's) vertex buffer; `x`, `y` are `f32`.
    graphicFontDraw @ 0x0064_0660 => extern "C" fn graphic_font_draw(rec: u32, name: u32, x: u32, y: u32, slot: u32) {
        font::graphic_font_draw(&mut mem(), rec, name, x, y, slot);
    }
}
