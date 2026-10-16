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
    use ffx_bridge::{Global, Mem, x87};

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

    // ---- graphicUiElementRemap ----
    // The UI object's remap: +0x5ba0 on, +0x5ba4 only inside the rectangle
    // +0x5bb8 (min x), +0x5bbc (max x), +0x5bc0 (min y), +0x5bc4 (max y);
    // the scale +0x5ba8/+0x5bac and offset +0x5bb0/+0x5bb4.

    /// A float of the UI object.
    fn uf(m: &impl Mem, ui: u32, off: u32) -> f32 {
        f32::from_bits(m.read(ui + off))
    }

    /// A float of the element.
    fn ef(m: &impl Mem, e: u32, off: u32) -> f32 {
        f32::from_bits(m.read(e + off))
    }

    fn put(m: &mut impl Mem, e: u32, off: u32, v: f32) {
        m.write(e + off, v.to_bits());
    }

    /// Whether the element's point at `off` lies in the rectangle (a NaN
    /// coordinate counts as inside: every comparison is false).
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN counts as inside, as the x87 compares
    fn inside(m: &impl Mem, ui: u32, e: u32, off: u32) -> bool {
        let (x, y) = (ef(m, e, off), ef(m, e, off + 4));
        !(x < uf(m, ui, 0x5bb8))
            && !(x > uf(m, ui, 0x5bbc))
            && !(y < uf(m, ui, 0x5bc0))
            && !(y > uf(m, ui, 0x5bc4))
    }

    /// `scale * v + offset` (`fld scale; fmul v; fadd offset; fstp`).
    fn axis(m: &impl Mem, ui: u32, axis: u32, v: f32) -> f32 {
        x87::Reg::load(uf(m, ui, 0x5ba8 + axis))
            .mul_m32(v)
            .add_m32(uf(m, ui, 0x5bb0 + axis))
            .store()
    }

    /// `v * scale + offset` (the operand order of the `y` and later points).
    fn axis_rev(m: &impl Mem, ui: u32, axis: u32, v: f32) -> f32 {
        x87::Reg::load(v)
            .mul_m32(uf(m, ui, 0x5ba8 + axis))
            .add_m32(uf(m, ui, 0x5bb0 + axis))
            .store()
    }

    /// `graphicUiElementRemap(element, kind)`: maps a UI element's corner
    /// points (`+0`, `+0x20`, and for kind 1 `+0x40`, kind 4 `+0x60`; `x`
    /// then `y`) from the PS2 layout to the remaster's. While the remap is
    /// limited to its rectangle, an element none of whose points lie in it
    /// stays. Kind 2 moves the box by its first point keeping its size; an
    /// element tagged 500 (`+0x80`) keeps its aspect: the second point
    /// follows the first at the smaller of the two scales.
    #[allow(clippy::many_single_char_names)]
    pub fn element_remap(m: &mut impl Mem, e: u32, kind: u32) {
        let ui = UI.get(m);
        if m.read::<u32>(ui + 0x5ba0) == 0 {
            return;
        }
        if m.read::<u32>(ui + 0x5ba4) != 0 {
            let points: &[u32] = match kind {
                4 => &[0, 0x20, 0x40, 0x60],
                1 => &[0, 0x20, 0x40],
                _ => &[0, 0x20],
            };
            if !points.iter().any(|&p| inside(m, ui, e, p)) {
                return;
            }
        }
        if kind == 2 {
            let dx = x87::Reg::load(ef(m, e, 0x20)).sub_m32(ef(m, e, 0)).store();
            let dy = x87::Reg::load(ef(m, e, 0x24)).sub_m32(ef(m, e, 4)).store();
            let x0 = axis(m, ui, 0, ef(m, e, 0));
            put(m, e, 0, x0);
            let y0 = axis(m, ui, 4, ef(m, e, 4));
            put(m, e, 4, y0);
            let x1 = x87::Reg::load(ef(m, e, 0)).add_m32(dx).store();
            put(m, e, 0x20, x1);
            let y1 = x87::Reg::load(ef(m, e, 4)).add_m32(dy).store();
            put(m, e, 0x24, y1);
            return;
        }
        if m.read::<u32>(e + 0x80) == 500 {
            let dx = x87::Reg::load(ef(m, e, 0x20)).sub_m32(ef(m, e, 0)).store();
            let dy = x87::Reg::load(ef(m, e, 0x24)).sub_m32(ef(m, e, 4)).store();
            let x0 = axis(m, ui, 0, ef(m, e, 0));
            put(m, e, 0, x0);
            let y0 = axis(m, ui, 4, ef(m, e, 4));
            put(m, e, 4, y0);
            let (a, b) = (uf(m, ui, 0x5ba8), uf(m, ui, 0x5bac));
            // |b| < |a| picks b; otherwise (NaN too) a
            let a_abs = x87::round_trip(a).abs();
            let b_abs = x87::round_trip(b).abs();
            let s = x87::round_trip(if b_abs < a_abs { b } else { a });
            let sx = x87::Reg::load(s).mul_m32(dx).store();
            let x1 = x87::Reg::load(sx).add_m32(ef(m, e, 0)).store();
            put(m, e, 0x20, x1);
            let sy = x87::Reg::load(s).mul_m32(dy).store();
            let y1 = x87::Reg::load(sy).add_m32(ef(m, e, 4)).store();
            put(m, e, 0x24, y1);
            return;
        }
        let x0 = axis(m, ui, 0, ef(m, e, 0));
        put(m, e, 0, x0);
        let x1 = axis(m, ui, 0, ef(m, e, 0x20));
        put(m, e, 0x20, x1);
        let y0 = axis(m, ui, 4, ef(m, e, 4));
        put(m, e, 4, y0);
        let y1 = axis_rev(m, ui, 4, ef(m, e, 0x24));
        put(m, e, 0x24, y1);
        let last = match kind {
            1 => 0x40,
            4 => 0x60,
            _ => return,
        };
        let x = axis(m, ui, 0, ef(m, e, last));
        put(m, e, last, x);
        let y = axis_rev(m, ui, 4, ef(m, e, last + 4));
        put(m, e, last + 4, y);
    }
}
