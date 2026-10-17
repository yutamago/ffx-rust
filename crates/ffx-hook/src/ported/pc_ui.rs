//! `ffx-pcui`: Iggy data values, the Iggy menus' handler slots and the
//! sphere grid screen's helpers, the pointer zones.

use ffx_pcui::abmap::bridge as abmap;
use ffx_pcui::iggy::bridge as iggy;
use ffx_pcui::touch::bridge as touch;

use super::{mem, ported};

ported! {
    /// `GuiTouch_resetButtons(this)`.
    GuiTouch_resetButtons @ 0x0068_ab30 => extern "thiscall" fn gui_touch_reset_buttons(this: u32) {
        touch::reset_buttons(&mut mem(), this);
    }
    /// `GuiTouch_handleEvent(this, event)`.
    GuiTouch_handleEvent @ 0x0068_b470 => extern "thiscall" fn gui_touch_handle_event(this: u32, event: u32) {
        touch::handle_event(&mut mem(), this, event);
    }
    /// `GuiTouch_hover(this, x, y)`, floats taken as bits.
    GuiTouch_hover @ 0x0068_b6d0 => extern "thiscall" fn gui_touch_hover(this: u32, x: u32, y: u32) {
        touch::hover(&mut mem(), this, f32::from_bits(x), f32::from_bits(y));
    }
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
    /// `AbmapResetVectors()`.
    AbmapResetVectors @ 0x00a5_7120 => extern "C" fn abmap_reset_vectors() -> u32 {
        abmap::reset_vectors(&mut mem())
    }
    /// `AbmapReadPad(no_stick, pad)`.
    AbmapReadPad @ 0x00a5_7520 => extern "C" fn abmap_read_pad(no_stick: u32, pad: u32) -> u32 {
        abmap::read_pad(&mut mem(), no_stick, pad)
    }
    /// `AbmapFlagConnectedLinks(flag)`.
    AbmapFlagConnectedLinks @ 0x00a5_8df0 => extern "C" fn abmap_flag_connected_links(flag: u32) -> u32 {
        abmap::flag_connected_links(&mut mem(), flag)
    }
    /// `AbmapTryChangePlayer()`.
    AbmapTryChangePlayer @ 0x00a5_8ec0 => extern "C" fn abmap_try_change_player() -> u32 {
        abmap::try_change_player(&mut mem())
    }
}
