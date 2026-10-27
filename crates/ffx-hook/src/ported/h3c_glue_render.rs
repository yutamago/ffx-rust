//! `ffx_prender::h3c_*`: the render-side glue's largest functions (M4,
//! list `h3-glue-render`, third agent h3c).
//!
//! Threads: the main thread (the menus' and the sphere grid's font
//! drawing, the effects' texture lookups); the shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::h3c_font::bridge as font;
use ffx_prender::h3c_vfxtex::bridge as vfxtex;

ported! {
    // ---- batch 1 (ffx_prender::h3c_font) ----
    /// `graphicFontDraw(rec, name, x, y, slot)`: one glyph quad into the
    /// menu's (or the sphere grid's) vertex buffer; `x`, `y` are `f32`.
    graphicFontDraw @ 0x0064_0660 => extern "C" fn graphic_font_draw(rec: u32, name: u32, x: u32, y: u32, slot: u32) {
        font::graphic_font_draw(&mut mem(), rec, name, x, y, slot);
    }

    // ---- batch 2 (ffx_prender::h3c_vfxtex) ----
    /// `PhyreScene_isVFXTextureLoaded(scene; address, format, p4, p5,
    /// width, height, out)`: the effect texture's file into `out`; 1 (the
    /// cached hit, a special case), 2 (a list hit, "RWTT", the Japanese
    /// texture) or 0.
    PhyreScene_isVFXTextureLoaded @ 0x0066_04d0 => extern "thiscall" fn is_vfx_texture_loaded(this: u32, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32, p7: u32, out: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        vfxtex::is_vfx_texture_loaded(&mut mem(), f.at() + 0x14, this, p2, p3, p4, p5, p6, p7, out)
    }
}
