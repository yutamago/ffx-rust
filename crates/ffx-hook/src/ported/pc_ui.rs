//! `ffx-pcui`: Iggy data values, the Iggy menus' handler slots and the
//! sphere grid screen's helpers, the pointer zones.

use ffx_pcui::abmap::bridge as abmap;
use ffx_pcui::cursor::bridge as cursor;
use ffx_pcui::iggy::bridge as iggy;
use ffx_pcui::item::bridge as item;
use ffx_pcui::link::bridge as link;
use ffx_pcui::panel::bridge as panel;
use ffx_pcui::sea::bridge as sea;
use ffx_pcui::spdctrl::bridge as spdctrl;
use ffx_pcui::touch::bridge as touch;

use super::{mem, ported};

/// `_CIatan2(y, x)` through MSVCR110's import slot (`y` in `ST1`, `x` in
/// `ST0`), the result stored as a float.
fn ci_atan2(y: f64, x: f64) -> f32 {
    #[cfg(target_arch = "x86")]
    {
        let mut out = 0f32;
        let ops = [y, x];
        // SAFETY: the slot holds the CRT's `_CIatan2`, which takes its
        // operands in ST1/ST0 and leaves its result in ST0 (eax, ecx, edx,
        // the x87 and SSE registers clobbered). The x87 stack is empty here.
        // Only edi, which the callee keeps, is read after the call.
        unsafe {
            let f = (crate::rebase(link::CI_ATAN2_SLOT) as *const usize).read();
            std::arch::asm!(
                "fld qword ptr [{ops}]",
                "fld qword ptr [{ops} + 8]",
                "call {f}",
                "fstp dword ptr [edi]",
                ops = in(reg) ops.as_ptr(),
                f = in(reg) f,
                in("edi") &raw mut out,
                clobber_abi("C"),
            );
        }
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        // the CRT's intrinsics only run in the 32-bit game
        let _ = (y, x);
        f32::NAN
    }
}

ported! {
    /// `GuiTouch_update(this)`.
    GuiTouch_update @ 0x0068_a3a0 => extern "thiscall" fn gui_touch_update(this: u32) {
        touch::update(&mut mem(), this);
    }
    /// `GuiTouch_initGroup(group)`.
    GuiTouch_initGroup @ 0x0068_a420 => extern "fastcall" fn gui_touch_init_group(group: u32) {
        touch::init_group(&mut mem(), group);
    }
    /// `AbmapPadActivateHandler(panel, entry, choice, confirmed)`.
    AbmapPadActivateHandler @ 0x00a5_6160 => extern "C" fn abmap_pad_activate_handler(panel: u32, entry: u32, choice: u32, confirmed: u32) -> u32 {
        let _ = (panel, entry, confirmed);
        panel::activate_handler(&mut mem(), choice)
    }
    /// `AbmapCalcLinkPointsStraight(out, a, b, ra, rb, kind)`, floats
    /// taken as bits.
    AbmapCalcLinkPointsStraight @ 0x00a5_81f0 => extern "C" fn abmap_calc_link_points_straight(out: u32, a: u32, b: u32, ra: u32, rb: u32, kind: u32) -> u32 {
        link::calc_link_points_straight(&mut mem(), out, a, b, f32::from_bits(ra), f32::from_bits(rb), kind)
    }
    /// `AbmapCalcLinkPointsCurved(out, a, b, c, ra, rb, kind)`, floats
    /// taken as bits.
    AbmapCalcLinkPointsCurved @ 0x00a5_7710 => extern "C" fn abmap_calc_link_points_curved(out: u32, a: u32, b: u32, c: u32, ra: u32, rb: u32, kind: u32) -> u32 {
        link::calc_link_points_curved(&mut mem(), out, [a, b, c], f32::from_bits(ra), f32::from_bits(rb), kind, |_, y, x| ci_atan2(y, x))
    }
    /// `AbmapReadCursorStick(move)`.
    AbmapReadCursorStick @ 0x00a5_6bd0 => extern "C" fn abmap_read_cursor_stick(mv: u32) -> u32 {
        cursor::read_cursor_stick(&mut mem(), mv)
    }
    /// `AbmapFindNearestNode(dist, pos, filter)`.
    AbmapFindNearestNode @ 0x00a5_6a40 => extern "C" fn abmap_find_nearest_node(dist: u32, pos: u32, filter: u32) -> u32 {
        cursor::find_nearest_node(&mut mem(), dist, pos, filter)
    }
    /// `AbmapFindNodeInDirection(pos, dir, filter)`.
    AbmapFindNodeInDirection @ 0x00a5_6860 => extern "C" fn abmap_find_node_in_direction(pos: u32, dir: u32, filter: u32) -> u32 {
        cursor::find_node_in_direction(&mut mem(), pos, dir, filter)
    }
    /// `AbmapPadCtrl()`.
    AbmapPadCtrl @ 0x00a5_83e0 => extern "C" fn abmap_pad_ctrl() {
        panel::pad_ctrl(&mut mem());
    }
    // 0x008c5320 is hooked in menu.rs as MenuItem_LoadTargetKeys (ported
    // there first); item::parse_target_keys is a second port of it.
    /// `MenuItem_DrawRow(window, x, y, index)`, floats taken as bits.
    MenuItem_DrawRow @ 0x008c_9a70 => extern "C" fn menu_item_draw_row(window: u32, x: u32, y: u32, index: i32) {
        item::draw_row(&mut mem(), window, f32::from_bits(x), f32::from_bits(y), index);
    }
    /// `MenuItem_GetLinePos()`.
    MenuItem_GetLinePos @ 0x008c_9b60 => extern "C" fn menu_item_get_line_pos() {
        item::get_line_pos(&mut mem());
    }
    /// `rcuiSeaAddUvSpd(axis, step)`, the float taken as bits.
    rcuiSeaAddUvSpd @ 0x0093_0c00 => extern "C" fn yn_sea_add_uv_spd(axis: u32, step: u32) {
        sea::add_uv_spd(&mut mem(), axis, f32::from_bits(step));
    }
    /// `SpdCtrl_IsMapExcluded(map)`.
    SpdCtrl_IsMapExcluded @ 0x006f_6fa0 => extern "C" fn spd_ctrl_is_map_excluded(map: u32) -> u32 {
        spdctrl::is_map_excluded(&mut mem(), map)
    }
    /// `SpdCtrl_IsMagicExcluded()`.
    SpdCtrl_IsMagicExcluded @ 0x006f_6ff0 => extern "C" fn spd_ctrl_is_magic_excluded() -> u32 {
        spdctrl::is_magic_excluded(&mut mem())
    }
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
    /// `AbmapInitPlyTag(chr, node, scale, a, b, c)`.
    AbmapInitPlyTag @ 0x00a5_7f80 => extern "C" fn abmap_init_ply_tag(chr: u32, node: u32, scale: u32, a: u32, b: u32, c: u32) -> u32 {
        abmap::init_ply_tag(&mut mem(), [chr, node, scale, a, b, c])
    }
    /// `AbmapPositionPlyTag(chr)`.
    AbmapPositionPlyTag @ 0x00a5_8080 => extern "C" fn abmap_position_ply_tag(chr: u32) -> u32 {
        abmap::position_ply_tag(&mut mem(), chr)
    }
    /// `AbmapPadQuitHandler(panel, entry, choice, confirmed)`.
    AbmapPadQuitHandler @ 0x00a5_6060 => extern "C" fn abmap_pad_quit_handler(panel: u32, entry: u32, choice: u32, confirmed: u32) -> u32 {
        let _ = (panel, entry);
        panel::quit_handler(&mut mem(), choice, confirmed)
    }
    /// `AbmapPadItemHandler(panel, entry, choice, confirmed)`.
    AbmapPadItemHandler @ 0x00a5_60d0 => extern "C" fn abmap_pad_item_handler(panel: u32, entry: u32, choice: u32, confirmed: u32) -> u32 {
        let _ = (panel, choice);
        panel::item_handler(&mut mem(), entry, confirmed)
    }
    /// `AbmapPadCommandHandler(panel, entry, choice, confirmed)`.
    AbmapPadCommandHandler @ 0x00a5_6330 => extern "C" fn abmap_pad_command_handler(panel: u32, entry: u32, choice: u32, confirmed: u32) -> u32 {
        let _ = (panel, entry);
        panel::command_handler(&mut mem(), choice, confirmed)
    }
    /// `AbmapMainMes_00a567d0(id)`: menu text 9's name (as `TkMenuMainMes`).
    AbmapMainMes_00a567d0 @ 0x00a5_67d0 => extern "C" fn abmap_main_mes(id: u32) -> u32 {
        ffx_menu::tk::bridge::menu_text_of(&mem(), 9, false, id)
    }
    /// `AbmapStatusMes_00a56fc0(id)`: menu text 11's name (as `TkMenuStatusMes`).
    AbmapStatusMes_00a56fc0 @ 0x00a5_6fc0 => extern "C" fn abmap_status_mes(id: u32) -> u32 {
        ffx_menu::tk::bridge::menu_text_of(&mem(), 11, false, id)
    }
    /// `MenuItem_SortAllGear_008c9bf0()`.
    MenuItem_SortAllGear_008c9bf0 @ 0x008c_9bf0 => extern "C" fn menu_item_sort_all_gear() -> u32 {
        item::sort_all_gear(&mut mem())
    }
    /// `MenuMes_AddPktFont1612NoEdge_008e52b0(cursor, key, text, x, y, flags, r, g, b, a)`.
    MenuMes_AddPktFont1612NoEdge_008e52b0 @ 0x008e_52b0 => extern "C" fn menu_mes_add_pkt_font1612_no_edge(cursor: u32, key: u32, text: u32, x: u32, y: u32, flags: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        item::font1612_no_edge(&mut mem(), item::MENU_MES_ADD_PKT_FONT1612, [cursor, key, text, x, y, flags, r, g, b, a], &mut |_, f, args| super::call_cdecl(f, args))
    }
    /// `MenuMes_AddPktFont1612IconsNoEdge_008e52e0(cursor, key, text, x, y, flags, r, g, b, a)`.
    MenuMes_AddPktFont1612IconsNoEdge_008e52e0 @ 0x008e_52e0 => extern "C" fn menu_mes_add_pkt_font1612_icons_no_edge(cursor: u32, key: u32, text: u32, x: u32, y: u32, flags: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        item::font1612_no_edge(&mut mem(), item::MENU_MES_ADD_PKT_FONT1612_ICONS, [cursor, key, text, x, y, flags, r, g, b, a], &mut |_, f, args| super::call_cdecl(f, args))
    }
}
