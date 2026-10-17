//! `ffx-pcui`: Iggy data values, the Iggy menus' handler slots and the
//! sphere grid screen's helpers.

use ffx_pcui::abmap::bridge as abmap;
use ffx_pcui::iggy::bridge as iggy;

use super::{mem, ported};

ported! {
    /// `iggyDataValueSet(v, int)`.
    iggyDataValueSet @ 0x00a2_8d30 => extern "C" fn iggy_data_value_set(v: u32, value: i32) {
        iggy::set_int(&mut mem(), v, value);
    }
    /// `iggyDataValueSet(v, const char *)`.
    iggyDataValueSet_00a28d70 @ 0x00a2_8d70 => extern "C" fn iggy_data_value_set_utf8(v: u32, text: u32) {
        iggy::set_utf8(&mut mem(), v, text);
    }
    /// `iggyDataValueSet(v, bool)`.
    iggyDataValueSet_00a28db0 @ 0x00a2_8db0 => extern "C" fn iggy_data_value_set_bool(v: u32, value: u32) {
        iggy::set_bool(&mut mem(), v, value);
    }
    /// `IggyMenuHandlers_addCall(this, f, arg)`.
    IggyMenuHandlers_addCall @ 0x00a2_8df0 => extern "thiscall" fn iggy_menu_handlers_add_call(this: u32, f: u32, arg: u32) -> u32 {
        iggy::add_call(&mut mem(), this, f, arg)
    }
    /// `IggyMenuHandlers_add(this, a, b, c, handler)`.
    IggyMenuHandlers_add @ 0x00a2_8e40 => extern "thiscall" fn iggy_menu_handlers_add(this: u32, a: u32, b: u32, c: u32, handler: u32) -> u32 {
        iggy::add(&mut mem(), this, [a, b, c, handler])
    }
    /// `IggyMenu::setEnable(this, on)`.
    IggyMenu_setEnable @ 0x00a2_8ef0 => extern "thiscall" fn iggy_menu_set_enable(this: u32, on: u32) {
        iggy::set_enable(&mut mem(), this, on);
    }
    /// `AbmapZoomScale()`.
    AbmapZoomScale @ 0x00a5_6820 => extern "C" fn abmap_zoom_scale() -> f32 {
        abmap::zoom_scale(&mem())
    }
    /// `AbmapNodeRadius(node)`.
    AbmapNodeRadius @ 0x00a5_6f30 => extern "C" fn abmap_node_radius(node: u32) -> f32 {
        abmap::node_radius(&mem(), node)
    }
    /// `AbmapRangesOverlap(a, b, c, d)`, floats taken as bits.
    AbmapRangesOverlap @ 0x00a5_7000 => extern "C" fn abmap_ranges_overlap(a: u32, b: u32, c: u32, d: u32) -> u32 {
        abmap::ranges_overlap(f32::from_bits(a), f32::from_bits(b), f32::from_bits(c), f32::from_bits(d))
    }
}
