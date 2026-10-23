//! The menu's world map window (`g_TkMenuWorldMapWindow`, opened by
//! `MenuWorldMap_OpenWindow`): its draw callback, which the PC port
//! placed among its UI code.

use ffx_graphic::ui::{remap_x2, remap_y2};

/// The world map picture's quad: `(x, y, w, h)` remapped from the PS2
/// screen (445, 951, 1130, 55) and its UVs (0, 544, 1600, 599).
#[must_use]
pub fn map_quad() -> [f32; 8] {
    [
        remap_x2(445.0),
        remap_y2(951.0),
        remap_x2(1130.0),
        remap_y2(55.0),
        0.0,
        544.0,
        1600.0,
        599.0,
    ]
}

/// The title plate's quad: `(x, y, w, h)` remapped (445, 933, 360, 24)
/// and its UVs (0.529..., 0.466..., 0.881..., 0.489...).
#[must_use]
pub fn title_quad() -> [f32; 8] {
    [
        remap_x2(445.0),
        remap_y2(933.0),
        remap_x2(360.0),
        remap_y2(24.0),
        f32::from_bits(0x3f07_8000),
        f32::from_bits(0x3eee_8000),
        f32::from_bits(0x3f61_8000),
        f32::from_bits(0x3efa_8000),
    ]
}

/// Where the dictionary name goes: (535, 954) remapped.
#[must_use]
pub fn name_pos() -> [f32; 2] {
    [remap_x2(535.0), remap_y2(954.0)]
}

/// The marker's position for `TOSeqSetScenePosByName`: half of each
/// coordinate (C division, toward zero), y 16 higher.
#[must_use]
pub fn marker_pos(x: i32, y: i32) -> (i32, i32) {
    (x / 2, y / 2 - 0x10)
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{map_quad, marker_pos, name_pos, title_quad};

    /// `TOMenuOpenPktBuffTmp()`, `TODrawScissorXYWH(x, y, w, h)`,
    /// `0x00902810(a, b, c0, c1)` (the fade tint), `TOMenuDrawKickTmp()`.
    pub const OPEN_PKT_BUFF_TMP: u32 = 0x008b_ef30;
    pub const DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
    pub const DRAW_TINT: u32 = 0x0090_2810;
    pub const DRAW_KICK_TMP: u32 = 0x008b_ea20;
    /// `AtelGetMapMarkerX()`, `AtelGetMapMarkerY()`.
    pub const MAP_MARKER_X: u32 = 0x0087_3ae0;
    pub const MAP_MARKER_Y: u32 = 0x0087_3af0;
    /// `TOSeqStartScene(i, name)`, `TOSeqSetScenePosByName(i, name, x,
    /// y)` and the scene's name `"scene30"`.
    pub const SEQ_START_SCENE: u32 = 0x008a_f0d0;
    pub const SEQ_SET_SCENE_POS_BY_NAME: u32 = 0x008a_f060;
    pub const SCENE30: u32 = 0x00b6_68f0;
    /// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0, c1)`,
    /// `TOMkpShapeXYWHUV(shape, x, y, w, h, u0, v0, u1, v1)`.
    pub const MKP_SHAPE_XYWHUV_C2: u32 = 0x0090_3ee0;
    pub const MKP_SHAPE_XYWHUV: u32 = 0x0090_3bb0;
    /// `AtelGetSaveDic()`, `MsGetSaveConfigEnglish()`,
    /// `AtelGetSaveDicName(dic, english)`.
    pub const GET_SAVE_DIC: u32 = 0x0086_c410;
    pub const GET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;
    pub const GET_SAVE_DIC_NAME: u32 = 0x0086_c430;
    /// `TOMkpCrossExtMesFontLClut(a, text, x, y, b, scale, alpha)`.
    pub const MKP_CROSS_EXT_MES_FONT_L_CLUT: u32 = 0x0090_16b0;

    /// `MenuWorldMap_Draw_008e2b60(window)` (name ours; the draw callback
    /// `MenuWorldMap_OpenWindow` stores at `+0x10`): the temporary packet
    /// buffer opened, the full-screen scissor (0, 0, 512, 416), the tint
    /// `alpha << 24 | 0x808080` twice (`alpha` the window's `+0x64`).
    /// Fully shown (`alpha` 0x80): when the map marker is not (0, 0),
    /// `"scene30"` started and placed at [`marker_pos`]; the map picture
    /// (shape -1, colours `0x80808080`/`0x808080`), the title plate
    /// (shape 200), and the save's dictionary name (`AtelGetSaveDicName`
    /// of `AtelGetSaveDic()` and `MsGetSaveConfigEnglish()`) at
    /// [`name_pos`] (scale 0.78, alpha 1). Ends with a tail call to
    /// `TOMenuDrawKickTmp`, whose `eax` it returns.
    pub fn draw<M, C>(mem: &mut M, window: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let mut cdecl = |mem: &mut M, sym: u32, args: &[u32]| {
            let f = mem.rebase(sym);
            call(mem, f, args)
        };
        cdecl(mem, OPEN_PKT_BUFF_TMP, &[]);
        cdecl(mem, DRAW_SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
        let alpha: u32 = mem.read(window.wrapping_add(0x64));
        let tint = alpha << 24 | 0x80_8080;
        cdecl(mem, DRAW_TINT, &[0, 0, tint, tint]);
        if mem.read::<u32>(window.wrapping_add(0x64)) == 0x80 {
            let x = cdecl(mem, MAP_MARKER_X, &[]);
            let y = cdecl(mem, MAP_MARKER_Y, &[]);
            if x != 0 || y != 0 {
                let scene = mem.rebase(SCENE30);
                cdecl(mem, SEQ_START_SCENE, &[1, scene]);
                let (px, py) = marker_pos(x.cast_signed(), y.cast_signed());
                cdecl(
                    mem,
                    SEQ_SET_SCENE_POS_BY_NAME,
                    &[1, scene, px.cast_unsigned(), py.cast_unsigned()],
                );
            }
            let q = map_quad().map(f32::to_bits);
            let mut args = vec![0xffff_ffff];
            args.extend(q);
            args.extend([0x8080_8080, 0x80_8080]);
            cdecl(mem, MKP_SHAPE_XYWHUV_C2, &args);
            let q = title_quad().map(f32::to_bits);
            let mut args = vec![200];
            args.extend(q);
            cdecl(mem, MKP_SHAPE_XYWHUV, &args);
            let dic = cdecl(mem, GET_SAVE_DIC, &[]);
            let english = cdecl(mem, GET_SAVE_CONFIG_ENGLISH, &[]);
            let name = cdecl(mem, GET_SAVE_DIC_NAME, &[dic, english]);
            let [nx, ny] = name_pos().map(f32::to_bits);
            cdecl(
                mem,
                MKP_CROSS_EXT_MES_FONT_L_CLUT,
                &[0, name, nx, ny, 0, 0.78f32.to_bits(), 1.0f32.to_bits()],
            );
        }
        cdecl(mem, DRAW_KICK_TMP, &[])
    }
}
