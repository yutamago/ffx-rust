//! Pieces of the item menu the PC port placed among its UI code: the key
//! string of the item a target uses, the item list's row plates, the
//! two line anchors.

use ffx_graphic::ui::{remap_x2, remap_y2};

/// The plate of an item list row at `(x, y)`: 740 x 60 (remapped),
/// drawn as `TODrawMenuPlateXYWHType(x, y, w, h, 5)`.
#[must_use]
pub fn row_plate(x: f32, y: f32) -> [f32; 4] {
    [x, y, remap_x2(740.0), remap_y2(60.0)]
}

/// Where the command cursor goes on a row at `(x, y)`: 160 right and 4
/// down (remapped), each sum rounded to a float.
#[must_use]
pub fn row_cursor(x: f32, y: f32) -> [f32; 2] {
    #[allow(clippy::cast_possible_truncation)] // the original's fstp dword
    let cy = (f64::from(remap_y2(4.0)) + f64::from(y)) as f32;
    #[allow(clippy::cast_possible_truncation)]
    let cx = (f64::from(remap_x2(160.0)) + f64::from(x)) as f32;
    [cx, cy]
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// `TOGetFFXLangJmp()`: the text language, 0 Japanese.
    pub const TO_GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;
    /// `TOMenuGetNameMes(id)`: a menu message.
    pub const TO_MENU_GET_NAME_MES: u32 = 0x008b_edd0;
    /// `Menu_ParseItemKeys(s)`.
    pub const MENU_PARSE_ITEM_KEYS: u32 = 0x008c_5280;
    /// The message ids of the targets' key strings, 4 bytes apart:
    /// Japanese, then the other languages.
    pub const TARGET_KEYS: [u32; 2] = [0x00c5_a0d8, 0x00c5_a120];
    /// `TkMn2SeqGetPos(name, &pos)`.
    pub const TK_MN2_SEQ_GET_POS: u32 = 0x008c_2b50;
    /// `"p_linmline"`, `"p_rinmline"`: the line anchors' layout names.
    pub const LINE_NAMES: [u32; 2] = [0x00b6_59f8, 0x00b6_5a04];
    /// Where `MenuItem_GetLinePos` puts the two anchors.
    pub const LINE_POS: [u32; 2] = [0x0186_a400, 0x0186_a404];
    /// `TODrawMenuPlateXYWHType(x, y, w, h, kind)`.
    pub const TO_DRAW_MENU_PLATE_XYWH_TYPE: u32 = 0x008f_5f70;
    /// `TkMn2DrawCrossCmdCursor(x, y, kind)`.
    pub const TK_MN2_DRAW_CROSS_CMD_CURSOR: u32 = 0x008c_0580;
    /// `MsGetSaveWeapon(id, &out)`.
    pub const MS_GET_SAVE_WEAPON: u32 = 0x007a_bbd0;
    /// `DrawCrossMenuIconWeaponName2(weapon, x, y, colour)`.
    pub const DRAW_CROSS_MENU_ICON_WEAPON_NAME2: u32 = 0x008e_6970;
    /// `g_TkMenuItemList`: `{u16 id, u8 used, u8}` entries.
    pub const ITEM_LIST: u32 = 0x0159_7770;

    /// `MenuItem_ParseTargetKeys(t)` (`0x008c5320`): for `t >= 0`, the key
    /// string of target `t` (`TOMenuGetNameMes` of the `u16` at
    /// `0x00c5a0d8 + 4t` in Japanese, `0x00c5a120 + 4t` otherwise) into
    /// `Menu_ParseItemKeys` (a tail call; neither returns anything).
    pub fn parse_target_keys(mem: &mut impl Host, t: i32) {
        if t < 0 {
            return;
        }
        let lang = mem.call_sym(TO_GET_FFX_LANG_JMP, Conv::Cdecl, &[]);
        let table = TARGET_KEYS[usize::from(lang != 0)];
        let at = mem
            .rebase(table)
            .wrapping_add(t.cast_unsigned().wrapping_mul(4));
        let id: u16 = mem.read(at);
        let s = mem.call_sym(TO_MENU_GET_NAME_MES, Conv::Cdecl, &[u32::from(id)]);
        mem.call_sym(MENU_PARSE_ITEM_KEYS, Conv::Cdecl, &[s]);
    }

    /// `MenuItem_GetLinePos()` (`0x008c9b60`): the left and right line
    /// anchors (`TkMn2SeqGetPos`) into [`LINE_POS`].
    pub fn get_line_pos(mem: &mut impl Host) {
        for (name, out) in LINE_NAMES.into_iter().zip(LINE_POS) {
            let (name, out) = (mem.rebase(name), mem.rebase(out));
            mem.call_sym(TK_MN2_SEQ_GET_POS, Conv::Cdecl, &[name, out]);
        }
    }

    /// `MenuItem_DrawRow(window, x, y, index)` (`0x008c9a70`): row `index`
    /// of the item window (`+0x30` rows, `+0x45` cursor shown, `+0x2c`
    /// cursor row) at `(x, y)`: nothing outside `0..rows`; the plate
    /// ([`super::row_plate`]), the command cursor when shown (`> 0`) on this
    /// row ([`super::row_cursor`]), the weapon name of
    /// `g_TkMenuItemList[index]` when used (`MsGetSaveWeapon`, whose second
    /// argument is a local the original drops).
    pub fn draw_row(mem: &mut impl Host, window: u32, x: f32, y: f32, index: i32) {
        // the arguments go through fld/fstp on their way to the callees
        let (x, y) = (round_trip(x), round_trip(y));
        let rows: u16 = mem.read(window.wrapping_add(0x30));
        if index < 0 || index >= i32::from(rows) {
            return;
        }
        let [px, py, w, h] = super::row_plate(x, y).map(f32::to_bits);
        mem.call_sym(
            TO_DRAW_MENU_PLATE_XYWH_TYPE,
            Conv::Cdecl,
            &[px, py, w, h, 5],
        );
        let shown: i8 = mem.read(window.wrapping_add(0x45));
        let cursor: i16 = mem.read(window.wrapping_add(0x2c));
        if shown > 0 && index == i32::from(cursor) {
            let [cx, cy] = super::row_cursor(x, y);
            mem.call_sym(
                TK_MN2_DRAW_CROSS_CMD_CURSOR,
                Conv::Cdecl,
                &[cx.to_bits(), cy.to_bits(), 1],
            );
        }
        let entry = mem
            .rebase(ITEM_LIST)
            .wrapping_add(index.cast_unsigned().wrapping_mul(4));
        let used: u8 = mem.read(entry.wrapping_add(2));
        if used != 0 {
            let id: u16 = mem.read(entry);
            let local = mem.scratch();
            let weapon = mem.call_sym(MS_GET_SAVE_WEAPON, Conv::Cdecl, &[u32::from(id), local]);
            mem.call_sym(
                DRAW_CROSS_MENU_ICON_WEAPON_NAME2,
                Conv::Cdecl,
                &[weapon, x.to_bits(), y.to_bits(), 0],
            );
        }
    }
}
