//! The UI's coordinate remaps between the PS2 screen (512 x 416) and the
//! remaster's (1920 x 1080, 1280 x 720), and the font's screen metrics
//! (`graphicUiRemap*`, `graphicFont*`).
//!
//! The remaps compute on the x87 at 53-bit precision and round to `f32`
//! once (`fstp dword`), then load it back: `f64` arithmetic in the
//! original's order, one rounding.

use ffx_bridge::x87;

/// The PS2 screen's width.
pub const PS2_WIDTH: f64 = 512.0;
/// The PS2 screen's height.
pub const PS2_HEIGHT: f64 = 416.0;
/// The remaster's full-HD screen.
pub const HD_WIDTH: f64 = 1920.0;
/// Its height.
pub const HD_HEIGHT: f64 = 1080.0;
/// The 720p screen.
pub const HD720_WIDTH: f64 = 1280.0;
/// Its height.
pub const HD720_HEIGHT: f64 = 720.0;

/// `x * scale / span`, as `fld; fmul m64; fdiv m64; fstp m32`.
#[must_use]
pub fn remap(x: f32, scale: f64, span: f64) -> f32 {
    let v = x87::div(x87::mul(x87::load(x.to_bits()), scale), span);
    f32::from_bits(x87::store(v))
}

/// `graphicUiRemapX2(x)`: a 1920-wide x on the PS2 screen.
#[must_use]
pub fn remap_x2(x: f32) -> f32 {
    remap(x, PS2_WIDTH, HD_WIDTH)
}

/// `graphicUiRemapY2(y)`: a 1080-high y on the PS2 screen.
#[must_use]
pub fn remap_y2(y: f32) -> f32 {
    remap(y, PS2_HEIGHT, HD_HEIGHT)
}

/// `graphicUiRemapX` (`FUN_006447c0`): a 1280-wide x on the PS2 screen.
#[must_use]
pub fn remap_x(x: f32) -> f32 {
    remap(x, PS2_WIDTH, HD720_WIDTH)
}

/// `graphicUiRemapY` (`FUN_00644800`): a 720-high y on the PS2 screen.
#[must_use]
pub fn remap_y(y: f32) -> f32 {
    remap(y, PS2_HEIGHT, HD720_HEIGHT)
}

/// `graphicGetScreenRatio()`: 16:9 as the image's `f32`.
pub const SCREEN_RATIO: f32 = 16.0 / 9.0;

/// `graphicFontGetEdgeRatio`: the screen width over 1920 (through an
/// `f32` twice), capped at 1.
#[must_use]
pub fn font_edge_ratio(screen_width: i32) -> f32 {
    let w = f32::from_bits(x87::store(f64::from(screen_width)));
    let r = f32::from_bits(x87::store(x87::div(f64::from(w), HD_WIDTH)));
    // `fcom st(1)`: keeps the ratio unless it is above 1
    if f64::from(r) > 1.0 { 1.0 } else { r }
}

/// The originals, over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `g_GraphicUi`: the remaster's UI object (`+0x2be0` the UI type).
    pub const UI: Global<u32> = Global::new(0x00cc_c81c);
    /// `g_GraphicAbmap`: the sphere grid's render object (`+4` its state).
    pub const ABMAP: Global<u32> = Global::new(0x00cc_c838);

    /// `graphicSetNowUIType(v)` and `cgraphicSetNowUIType(v)`: the UI
    /// object's `+0x2be0`. Returns the object (`eax`).
    pub fn set_now_ui_type(m: &mut impl Mem, v: u32) -> u32 {
        let ui = UI.get(m);
        m.write(ui + 0x2be0, v);
        ui
    }

    /// `graphicGetAbmap()`: whether the sphere grid object exists with a
    /// state below 2.
    pub fn get_abmap(m: &impl Mem) -> u32 {
        let a = ABMAP.get(m);
        u32::from(a != 0 && m.read::<i32>(a + 4) < 2)
    }

    /// `graphicFontGetScreenWH(&w, &h)`: 1920 x 1080. Returns `h`'s
    /// pointer (`eax`).
    pub fn font_get_screen_wh(m: &mut impl Mem, w: u32, h: u32) -> u32 {
        m.write(w, 1920u32);
        m.write(h, 1080u32);
        h
    }

    /// `graphicFontGetEdgeRatio()`: of the scene's screen width
    /// (`+0x10dd0`).
    pub fn font_get_edge_ratio(m: &impl Mem) -> f32 {
        let scene = crate::scene::bridge::SCENE.get(m);
        super::font_edge_ratio(m.read(scene + 0x10dd0))
    }
}
