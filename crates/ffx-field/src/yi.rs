//! `yi`: the field map renderer's state and its display objects
//! (`docs/formats/field.md`).

use ffx_bridge::Global;

/// What the field renderer does (`yiRenderEnable`/`yiRenderDisable`); bits
/// 0 and 4 are set once a map is read (`yiMapReadMaster`), bit 0 gates the
/// field particles (`yiCallFieldParticle`).
pub const RENDER_STATUS: Global<u32> = Global::new(0x018d_ed08);
/// 1/32 and 32.0 (`f64`s in `.rdata`): `yiRender_setupFogParam`'s scale.
pub const FOG_EIGHTH: u32 = 0x00b5_01a0;
/// See [`FOG_EIGHTH`].
pub const FOG_SCALE32: u32 = 0x00b4_12f8;
/// 255.0 as an `f64` (`.rdata`).
pub const FOG_255: u32 = 0x00b1_70c0;
/// 255.0 as an `f32` (`.rdata`).
pub const FOG_255_F32: u32 = 0x00b4_80d4;
/// The window handlers by kind (`yiWindowSetHandlers`): 6 words a kind.
pub const WINDOW_HANDLERS: u32 = 0x00c5_d788;
/// `pppSetFpPdt` (`0x0072a7f0`): takes a map file's `ppp` section.
pub const PPP_SET_FP_PDT: u32 = 0x0072_a7f0;
/// `MsBattleGetMapCircleOn` (`0x007816a0`).
pub const MAP_CIRCLE_ON: u32 = 0x0078_16a0;
/// `0x018ded30`: `+0x18` of the last map read taken in.
pub const READ_DONE: Global<u32> = Global::new(0x018d_ed30);
/// `TkRestartSyncH` (`0x0088c7f0`).
pub const TK_RESTART_SYNC_H: u32 = 0x0088_c7f0;
/// `TkGetSyncH` (`0x0088c800`).
pub const TK_GET_SYNC_H: u32 = 0x0088_c800;
/// `0x0088c820(5)`, the third call of `yiResetObj`'s sync steps.
pub const TK_SYNC_H_WAIT: u32 = 0x0088_c820;
/// `TkClearAlarm` (`0x00845000`).
pub const TK_CLEAR_ALARM: u32 = 0x0084_5000;
/// `pppFpFree` (`0x007291c0`).
pub const PPP_FP_FREE: u32 = 0x0072_91c0;
/// `graphicEnableChrClusterDelayRelease` and `Cache_Clear`: run when a 2D
/// map was loaded.
pub const GRAPHIC_CLUSTER_DELAY_RELEASE: u32 = 0x0064_0090;
/// See [`GRAPHIC_CLUSTER_DELAY_RELEASE`].
pub const CACHE_CLEAR: u32 = 0x0084_0030;
/// `graphicFieldMapShutdown`, `graphicFieldMapUnload` and
/// `graphicVFXUnloadLevelData`, `yiResetObj`'s last calls.
pub const GRAPHIC_FIELD_MAP_SHUTDOWN: u32 = 0x0064_03b0;
/// See [`GRAPHIC_FIELD_MAP_SHUTDOWN`].
pub const GRAPHIC_FIELD_MAP_UNLOAD: u32 = 0x0064_03c0;
/// See [`GRAPHIC_FIELD_MAP_SHUTDOWN`].
pub const GRAPHIC_VFX_UNLOAD: u32 = 0x0064_5860;
/// `0x0072aa40`: takes a map file's section 2 (`yiObjReadFinish`).
pub const MAP_SECTION2_HANDLER: u32 = 0x0072_aa40;
/// The map's background section (`0x0191389c`, set by `yiMapReadMaster`).
pub const MAP_SECTION: Global<u32> = Global::new(0x0191_389c);
/// The map read buffer `yiResetObj` frees (`0x019138a4`).
pub const MAP_READ_BUFFER: Global<u32> = Global::new(0x0191_38a4);
/// The 2D maps loaded (`0x01934ec4`, `yiMapReadMaster`).
pub const MAP2D_LOADS: Global<u32> = Global::new(0x0193_4ec4);
/// Two camera ids reset to -1 (`0x01934ed4`).
pub const CAMERA_IDS: Global<[u32; 2]> = Global::new(0x0193_4ed4);
/// The map and file ids of the map read (`0x01934edc`, `yiMapReadMaster`'s
/// first two arguments; -1 when none).
pub const MAP_IDS: Global<[u32; 2]> = Global::new(0x0193_4edc);
/// 0 after a 2D map is loaded, 1 after a 3D one (`0x01934ec0`).
pub const MAP_KIND: Global<u32> = Global::new(0x0193_4ec0);
/// The map data `loadPhyreFieldMap` waits for (0 when none).
pub const PENDING_MAP: Global<u32> = Global::new(0x0193_4ed0);
/// Its name, a string `graphicFieldMapLoad` takes (empty when none).
pub const PENDING_MAP_NAME: u32 = 0x0193_4ee8;
/// `g_BtlMapRead`, cleared by `loadPhyreFieldMap` without a request.
pub const BTL_MAP_READ: Global<u32> = Global::new(0x0112_ca68);
/// `"RAM Inuse : %d u:%d\n"` (`.rdata`).
pub const RAM_INUSE: u32 = 0x00b6_6f0c;
/// `graphicSetHas2dMap(on)` (`0x00643450`).
pub const GRAPHIC_SET_HAS_2D_MAP: u32 = 0x0064_3450;
/// `graphicSetHas3dMap(on)` (`0x00643470`).
pub const GRAPHIC_SET_HAS_3D_MAP: u32 = 0x0064_3470;
/// `graphicFieldMapLoad(name, kind)` (`0x00640210`).
pub const GRAPHIC_FIELD_MAP_LOAD: u32 = 0x0064_0210;
/// `graphicFieldMapInitialize()` (`0x00640200`).
pub const GRAPHIC_FIELD_MAP_INITIALIZE: u32 = 0x0064_0200;
/// `graphicVFXLoadLevelData()` (`0x006454a0`).
pub const GRAPHIC_VFX_LOAD_LEVEL_DATA: u32 = 0x0064_54a0;
/// `GetCD()` (`0x0076d0d0`): the CD file table (`+0x0` read, `+0x8`
/// finish, `+0x10` select a map, `+0x14` a file's size).
pub const GET_CD: u32 = 0x0076_d0d0;
/// `GetCDFileName_PC(file)` (`0x00a429d0`).
pub const GET_CD_FILE_NAME_PC: u32 = 0x00a4_29d0;
/// "RAM Inuse : %d u:%d\n" (`0x00b66f0c`).
pub const RAM_INUSE_TEXT: u32 = 0x00b6_6f0c;
/// `0x009161d0(map, dir, name, extra)`: a map's path parts.
pub const MAP_PATH_PARTS: u32 = 0x0091_61d0;
/// MSVCR110's `sprintf` import slot (`0x00b0c3dc`).
pub const CRT_SPRINTF: u32 = 0x00b0_c3dc;
/// `yiMapReadMaster` (`0x009093b0`).
pub const YI_MAP_READ_MASTER: u32 = 0x0090_93b0;
/// `FFXVu0MulMatrix(out, a, b)` (`0x006ed5c0`).
pub const VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
/// The three battle view matrices (`0x018ded78`, `0x018dedb8`, `0x018dedf8`).
pub const BATTLE_VIEWS: [u32; 3] = [0x018d_ed78, 0x018d_edb8, 0x018d_edf8];
/// `MsCameraGetScrDpt()` (`0x007bd510`): the screen depth in `ST0`.
pub const MS_CAMERA_GET_SCR_DPT: u32 = 0x007b_d510;
/// Where the screen depth is kept (`0x018df160`).
pub const SCREEN_DEPTH: u32 = 0x018d_f160;
/// `Sg_LightGetDir(i, out)` (`0x008361c0`).
pub const SG_LIGHT_GET_DIR: u32 = 0x0083_61c0;
/// `Sg_LightGetColor(i, out)` (`0x008360f0`).
pub const SG_LIGHT_GET_COLOR: u32 = 0x0083_60f0;
/// `FFXVu0NormalLightMatrix(out, a, b, c)` (`0x006ed620`).
pub const VU0_NORMAL_LIGHT_MATRIX: u32 = 0x006e_d620;
/// `FFXVu0LightColorMatrix(out, a, b, c, ambient)` (`0x006ed4f0`).
pub const VU0_LIGHT_COLOR_MATRIX: u32 = 0x006e_d4f0;
/// The renderer's normal light matrix (`0x018deeb8`).
pub const NORMAL_LIGHT: u32 = 0x018d_eeb8;
/// The renderer's light colour matrix (`0x018deef8`).
pub const LIGHT_COLOR: u32 = 0x018d_eef8;
/// The cached 0x14-format image page (`0x025d17a4`, -1 when none).
pub const IMAGE_PAGE_CACHE: Global<u32> = Global::new(0x025d_17a4);
/// The map name table (`0x025d5888`): 16-byte entries, the name, the map
/// number at `+0xc`.
pub const MAP_NAMES: Global<u32> = Global::new(0x025d_5888);
/// Its entry count (`0x01934ec8`).
pub const MAP_NAME_COUNT: Global<u32> = Global::new(0x0193_4ec8);
/// The name without a current map (`0x00b424c8`).
pub const MAP_NAME_NONE: u32 = 0x00b4_24c8;
/// The name of a map not in the table (`0x00b66fd4`).
pub const MAP_NAME_UNKNOWN: u32 = 0x00b6_6fd4;
/// "%s" (`0x00b0e160`).
pub const FORMAT_S: u32 = 0x00b0_e160;
/// "host0:%s%s.vpa" (`0x00b66efc`).
pub const HOST_VPA_FORMAT: u32 = 0x00b6_6efc;
/// `sceOpen(path, mode)` (`0x0062fa30`).
pub const SCE_OPEN: u32 = 0x0062_fa30;
/// `sceLseek(fd, offset, whence)` (`0x0062f8e0`).
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
/// `sceRead(fd, buf, n)` (`0x0062fc00`).
pub const SCE_READ: u32 = 0x0062_fc00;
/// `sceClose(fd)` (`0x0062f610`).
pub const SCE_CLOSE: u32 = 0x0062_f610;
/// `0x018ded34`: `+0x18` of the last finished map read.
pub const READ_TAIL: Global<u32> = Global::new(0x018d_ed34);
/// `Sg_GetKeepFps` (`0x008205a0`).
pub const SG_GET_KEEP_FPS: u32 = 0x0082_05a0;
/// `g_YiAnimUseVCount`: animations time by `yiGetVCount` when set.
pub const ANIM_USE_VCOUNT: Global<u32> = Global::new(0x0193_bb08);
/// `graphicFFXApplicationEnableKeyframeAnim` (`0x006401e0`).
pub const ENABLE_KEYFRAME_ANIM: u32 = 0x0064_01e0;
/// `yiRender_setupparamVU1` (`0x0090da90`).
pub const RENDER_SETUP_VU1: u32 = 0x0090_da90;
/// `yiSwizzleImage` (`0x0090e0b0`): swizzles a format-0x13 image into the
/// scene's VRAM copy (`yiLoadVramImage` calls it).
pub const YI_SWIZZLE_IMAGE: u32 = 0x0090_e0b0;
/// The byte order inside a 16x16 block's column groups (`0x00c5da80`, two
/// halves of 64 for even and odd groups).
pub const SWIZZLE_COLUMNS: u32 = 0x00c5_da80;
/// The block numbers of a 128x64 page in reading order (`0x00c5db40`).
pub const SWIZZLE_ORDER: u32 = 0x00c5_db40;
/// The GS block layout: four rows of eight block numbers (`0x00c5db60`).
pub const SWIZZLE_BLOCKS: u32 = 0x00c5_db60;
/// "Error : width is not 2^n\n" (`0x00b670d0`).
pub const SWIZZLE_ERROR: u32 = 0x00b6_70d0;
/// MSVCR110's `__iob_func` import slot (`0x00b0c350`).
pub const CRT_IOB_FUNC: u32 = 0x00b0_c350;
/// MSVCR110's `fprintf` import slot (`0x00b0c38c`).
pub const CRT_FPRINTF: u32 = 0x00b0_c38c;
/// `Sg_MemSetErrorIgnore`'s flag (`0x01301958`).
pub const MEM_ERROR_IGNORE: Global<u32> = Global::new(0x0130_1958);
/// The `Sg_Mem` heaps, 0x18 bytes each, indexed by bits 27..28 of a
/// block's `+0xc`: `+0` a byte set when a block was freed, `+0x14` the
/// lowest freed block (`Sg_MemSetFree`).
pub const MEM_HEAPS: u32 = 0x0130_1968;
/// The fixed GS packet `yiRender_initpacket` writes at `+0x27624`.
pub const RENDER_PACKET: [u32; 32] = [
    0,
    0,
    0,
    0x5100_0005,
    4,
    0x1003_0000,
    0xe,
    0,
    0,
    0,
    0x50,
    0,
    0,
    0,
    0x51,
    0,
    0x80,
    0,
    0x52,
    0,
    0,
    0,
    0x53,
    0,
    0,
    0,
    0,
    0x5100_4001,
    0xc000,
    0x0800_0000,
    0,
    0,
];
/// The `.rdata` floats of the fog block (`yiRender_initfog`): 0.03,
/// 0.02, 0.005 and 64.0.
pub const FOG_DEFAULTS: [u32; 4] = [0x00b6_7120, 0x00b4_82d8, 0x00b3_eb3c, 0x00b5_83dc];
/// The largest finite float and its negation (`f32`s, `.rdata`):
/// `yiBoundsMinMax`'s starting minimum and maximum.
pub const BOUNDS_INIT: [u32; 2] = [0x00b1_7504, 0x00b1_f2d0];
/// pi, -2pi, 2pi, -pi (`f64`s, `.rdata`): `yiKeyFrameWrapAngles`.
pub const WRAP_CONSTS: [u32; 4] = [0x00b6_7118, 0x00b6_7138, 0x00b6_7128, 0x00b6_7130];
/// The image VRAM area block (`0x025d1798`..`0x025d17d4`, `yiVramSetup`).
pub const VRAM_AREAS: u32 = 0x025d_1798;
/// The end of the last image area (`yiImageTakeLastPage`).
pub const LAST_AREA_END: Global<u32> = Global::new(0x025d_1798);
/// Where the last image area is used up to (its start while unused).
pub const LAST_AREA_USED: Global<u32> = Global::new(0x025d_179c);
/// The start of the last image area.
pub const LAST_AREA_START: Global<u32> = Global::new(0x025d_17a0);
/// Set while a PSMT4 image placed in the last area can share it (`u8`).
pub const LAST_AREA_PSMT4: Global<u8> = Global::new(0x0193_bb24);
/// `Sg_GetPause()` (`0x008205e0`).
pub const SG_GET_PAUSE: u32 = 0x0082_05e0;
/// 3.0 (`f64`, `.rdata`, `0x00b2c5c8`): the Hermite weights' constant.
pub const HERMITE_THREE: u32 = 0x00b2_c5c8;
/// `FFXVu0ApplyMatrix(dst, m, src)` (`0x006ed2e0`).
pub const VU0_APPLY_MATRIX: u32 = 0x006e_d2e0;
/// The display objects' morph values (`f32`, `0xb4` bytes apart;
/// `yiSetMIMeParamDObj`).
pub const MIME_PARAM: u32 = 0x018d_f624;
/// `graphicSetMeshVertexColorMorphValue(obj, v)` (`0x00643700`).
pub const SET_MORPH_VALUE: u32 = 0x0064_3700;
/// 128.0 (`f64`, `.rdata`): `yiGetColorOffsetInt`'s bias.
pub const COLOR_OFFSET_BIAS: u32 = 0x00b4_c170;
/// 1e-5 and -1e-5 (`f32`s, `.rdata`): `yiSegIntersect2`'s parallel test.
pub const SEG_EPSILON: [u32; 2] = [0x00b1_f2ac, 0x00b6_76cc];

/// `FFXVu0RotMatrixX/Y/Z(dst, src, angle)` (`0x006ed780`, `0x006ed7c0`,
/// `0x006ed800`).
pub const VU0_ROT_MATRIX: [u32; 3] = [0x006e_d780, 0x006e_d7c0, 0x006e_d800];
/// `Sg_MFIFOSync` (`0x00843360`).
pub const SG_MFIFO_SYNC: u32 = 0x0084_3360;
/// `sceGsSyncPath` (`0x0062f880`).
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
/// `sceGsSetDefLoadImage` (`0x0062f820`, eight arguments).
pub const GS_SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
/// `sceGsExecLoadImage` (`0x0062f7a0`).
pub const GS_EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
/// `FlushCache` (`0x0062f4c0`).
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
/// `graphicObjectSetTranslation` (`0x006426c0`).
pub const GRAPHIC_OBJECT_SET_TRANSLATION: u32 = 0x0064_26c0;
/// `graphicObjectSetRotation` (`0x006426a0`).
pub const GRAPHIC_OBJECT_SET_ROTATION: u32 = 0x0064_26a0;
/// `FFXVu0RotMatrixX` (`0x006ed780`).
pub const VU0_ROT_MATRIX_X: u32 = 0x006e_d780;
/// `FFXVu0RotMatrixY` (`0x006ed7c0`).
pub const VU0_ROT_MATRIX_Y: u32 = 0x006e_d7c0;
/// 180.0 and pi (`f64`s, `.rdata`): the fog angles in degrees to radians.
pub const DEG_180: u32 = 0x00b4_8378;
/// See [`DEG_180`].
pub const PI_F64: u32 = 0x00b6_7118;
/// `TODrawScissorXYWH` (`0x008f9230`).
pub const TO_DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
/// `0x008f9a40(x, y, w, h)`: the window plate (`yiWindowDrawPlate`).
pub const TO_DRAW_PLATE: u32 = 0x008f_9a40;
/// `TODrawScissor` (`0x008f91f0`).
pub const TO_DRAW_SCISSOR: u32 = 0x008f_91f0;
/// `TOMakePktScissor` (`0x008fdee0`).
pub const TO_MAKE_PKT_SCISSOR: u32 = 0x008f_dee0;
/// `TODrawMenuPlateXYWHType` (`0x008f5f70`, four `f32`s and a type).
pub const TO_DRAW_MENU_PLATE: u32 = 0x008f_5f70;
/// `0x008ac2f0()`: non-zero moves the kind 0 menu plate up a pixel.
pub const WINDOW_MENU_SHIFT: u32 = 0x008a_c2f0;
/// `TOGetWindowStructAdrs` (`0x00906fb0`).
pub const TO_GET_WINDOW: u32 = 0x0090_6fb0;
/// `0x008f5e90`: a sprite of the help bar (seven arguments).
pub const TO_DRAW_SPRITE: u32 = 0x008f_5e90;
/// `TOMkpShapeXY` (`0x00903a10`).
pub const TO_MKP_SHAPE_XY: u32 = 0x0090_3a10;
/// The help bar's layout (`i16` positions and two words,
/// `0x025d1618..=0x025d1631`).
pub const WINDOW_BAR_LAYOUT: u32 = 0x025d_1618;
/// `g_CurrentMapNo`: the pop window's scissor special case is map 0x15b.
pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
/// 65.0 and 290.0 (`f32`s): the pop window position of that case.
pub const POP_WINDOW_X: u32 = 0x00b5_eeb8;
/// See [`POP_WINDOW_X`].
pub const POP_WINDOW_Y: u32 = 0x00b5_f38c;
/// 38.0 (`f32`) and 115.0 (`f64`): the `TO` window it clips to.
pub const POP_TARGET_X: u32 = 0x00b5_9830;
/// See [`POP_TARGET_X`].
pub const POP_TARGET_Y: u32 = 0x00b6_6ec8;
/// 4.0 and 12.0 (`f64`s): the plate's scissor insets.
pub const WINDOW_PLATE_INSET_Y: u32 = 0x00b4_3d80;
/// See [`WINDOW_PLATE_INSET_Y`].
pub const WINDOW_PLATE_INSET_X: u32 = 0x00b4_b468;
/// 8.0, 1.0 and 5.0 (`f64`s): the menu plate's width inset, its shift
/// and the scissor's left inset.
pub const WINDOW_MENU_INSET_W: u32 = 0x00b9_22a8;
/// See [`WINDOW_MENU_INSET_W`].
pub const WINDOW_MENU_SHIFT_Y: u32 = 0x00b9_2248;
/// See [`WINDOW_MENU_INSET_W`].
pub const WINDOW_MENU_INSET_X: u32 = 0x00b4_1d20;
/// The idle window handler (`0x00907e50`) the window finishers set.
pub const WINDOW_IDLE: u32 = 0x0090_7e50;
/// The handler of an open window (`0x00907d70`, returns 0).
pub const WINDOW_OPEN_IDLE: u32 = 0x0090_7d70;
/// `yiWindowFinishAll` (`0x00907040`), a closing window's last handler.
pub const WINDOW_FINISH_ALL: u32 = 0x0090_7040;
/// 416.0 and 512.0 as `f32`s (`.rdata`): the window area's height and
/// width.
pub const WINDOW_HEIGHT_F32: u32 = 0x00b4_1d98;
/// See [`WINDOW_HEIGHT_F32`].
pub const WINDOW_WIDTH_F32: u32 = 0x00b4_1d9c;
/// 416.0, 512.0, 0.0 and 6.0 as `f64`s (`.rdata`): the slide sizes and
/// the six frames they take.
pub const WINDOW_HEIGHT: u32 = 0x00b4_1d30;
/// See [`WINDOW_HEIGHT`].
pub const WINDOW_WIDTH: u32 = 0x00b4_1d38;
/// See [`WINDOW_HEIGHT`].
pub const WINDOW_ZERO: u32 = 0x00b0_db90;
/// See [`WINDOW_HEIGHT`].
pub const WINDOW_FRAMES: u32 = 0x00b3_eb88;
/// 8.0 and 28.0 (`f32`s, `.rdata`): the bar window's top and bottom.
pub const WINDOW_BAR_TOP: u32 = 0x00b9_2314;
/// See [`WINDOW_BAR_TOP`].
pub const WINDOW_BAR_BOTTOM: u32 = 0x00b5_ee74;
/// The draw mask (`yiSetDrawMask`): -9 after a map is read, -1 when the
/// battle map circle is on (`yiMapReadMaster`); script native 0x8000 sets
/// its bits.
pub const DRAW_MASK: Global<u32> = Global::new(0x018d_ed0c);
/// The field scale factor, x, y, z (`yiSetScaleFactor`).
pub const SCALE_FACTOR: Global<[u32; 3]> = Global::new(0x018d_eff8);
/// The display objects in use (`yiGetMatrixDObj` bound).
pub const DOBJ_COUNT: Global<u32> = Global::new(0x018d_f588);
/// The display objects, [`dobj::SIZE`] bytes each.
pub const DOBJS: Global<u8> = Global::new(0x018d_f59c);
/// The map data read (`yiMapReadMaster`); `+4` is its status
/// (`yiGetMapStatus`).
pub const MAP_DATA: Global<u32> = Global::new(0x0191_38a8);
/// The vertical blank counter, +2 a frame (main loop, `0x00821e80`).
pub const SG_VCOUNT: Global<u32> = Global::new(0x012f_b7f0);
/// The main loop's frame counter (`Sg_MainLoop`, which swaps the display
/// buffers with it).
pub const SG_FRAME: Global<u32> = Global::new(0x023c_bbf0);

/// Set with bit 8 of [`RENDER_STATUS`] by `yiSetRenderParam` (script
/// native `0x0091c580`).
pub const RENDER_PARAM: Global<u32> = Global::new(0x018d_ed2c);
/// Three bytes `yiSetColor3` stores (script native `0x00859000`).
pub const COLOR3: Global<[u8; 3]> = Global::new(0x0193_4ea4);

/// A matrix the camera set-up multiplies in before the camera
/// (`0x0090a140`: `cdc_Mul4x4(out, camera, it)`; `yiGetViewAdjust`,
/// `yiSetViewAdjust`).
pub const VIEW_ADJUST: Global<[u32; 16]> = Global::new(0x018d_ee38);
/// The colour offset, four floats `channel - 128` (`yiSetColorOffset`;
/// saved and restored with the render state).
pub const COLOR_OFFSET: Global<[u32; 4]> = Global::new(0x018d_f01c);
/// The image VRAM allocator's next free address (`yiVramAlloc`, bytes,
/// 64-byte steps).
pub const VRAM_NEXT: Global<u32> = Global::new(0x025d_17bc);
/// Its end (`yiVramSetLimit`).
pub const VRAM_LIMIT: Global<u32> = Global::new(0x025d_17ac);

/// `yiVramSetLimit(pages)`: the allocator's end, `(0x38 - pages) * 0x4000`.
#[must_use]
pub fn vram_limit(pages: u32) -> u32 {
    0x38u32.wrapping_sub(pages).wrapping_shl(14)
}

/// `yiVramAlloc(bytes)`: the allocator's next address after `bytes`
/// rounded up to 64 (C division of a negative rounds toward zero).
#[must_use]
pub fn vram_end(next: u32, bytes: i32) -> u32 {
    let n = bytes.wrapping_add(0x3f);
    let rounded = n.wrapping_add((n >> 31) & 0x3f) & !0x3f;
    rounded.cast_unsigned().wrapping_add(next)
}

/// `yiPow2Ceil(n)`: the smallest `1 << k`, `k < 16`, not below `n`; 0 when
/// none is.
#[must_use]
pub fn pow2_ceil(n: i32) -> i32 {
    (0..16).map(|k| 1i32 << k).find(|&p| n <= p).unwrap_or(0)
}

/// `yiHighestBit(bits)`: the index of the highest of the low 16 bits set,
/// -1 for none.
#[must_use]
pub fn highest_bit(bits: u32) -> i32 {
    (0..16).rev().find(|&k| bits & (1 << k) != 0).unwrap_or(-1)
}

/// `yiLog2Ceil16(bits)`: for the low 16 bits, the index of the only bit
/// set, or the highest one's plus 1 when several are; -1 for none.
#[must_use]
pub fn log2_ceil16(bits: u32) -> i32 {
    let mut r = -1;
    for k in 0..16 {
        if bits & (1 << k) != 0 {
            r = if r == -1 { k } else { k + 1 };
        }
    }
    r
}

/// `yiIsPow2(bits)`: whether at most one of the low 16 bits is set.
#[must_use]
pub fn is_pow2(bits: u32) -> bool {
    (bits & 0xffff).count_ones() <= 1
}

/// The quadword of a GS register write in A+D mode: the data, then the
/// register (`yiPkAddGsAD` and its copies).
#[must_use]
pub const fn gs_ad(reg: u32, data: u32) -> [u32; 4] {
    [data, 0, reg, 0]
}

/// Offsets in a display object.
pub mod dobj {
    /// Size (`yiGetMatrixDObj` stride).
    pub const SIZE: u32 = 0xb4;
    /// `+0x0`: its matrix (16 floats).
    pub const MATRIX: u32 = 0x0;
    /// `+0x40`: rotation (`0x00908190` adds to it and rebuilds the matrix).
    pub const ROTATION: u32 = 0x40;
    /// `+0x50`: translation.
    pub const TRANSLATION: u32 = 0x50;
    /// `+0x88`: a float kept with the transform by `yiPushRenderState`.
    pub const WORD_88: u32 = 0x88;
    /// `+0x74`: the group (`u16`, `yiGetMatrixGroup`).
    pub const GROUP: u32 = 0x74;
    /// `+0x76`: the rotation order for `yiRotMatrixSeq` (`u16`).
    pub const ROT_ORDER: u32 = 0x76;
    /// `+0x90`: two RGBA colours, a byte per channel (`yiGetColorDObj`).
    pub const COLORS: u32 = 0x90;
}

/// Offsets in the renderer object (`yiRender_*`).
pub mod render {
    /// Words `yiRender_resettex` clears.
    pub const TEX_CLEARED: [u32; 3] = [0x9c198, 0xbe0ac, 0xbe0b0];
    /// The word it sets to -1.
    pub const TEX_NONE: u32 = 0x67f34;
    /// The DMA packet's write position (`yiRender_addRef`).
    pub const PACKET: u32 = 0xba3f0;
    /// Tags added to it.
    pub const PACKET_COUNT: u32 = 0xba3f8;
    /// The word `yiScreff_init` clears.
    pub const SCREFF: u32 = 0xbe078;
    /// The objects that can take an animation (`yiAnimAttachDefault`'s
    /// bound).
    pub const ANIM_IDS: u32 = 0x68788;
    /// Animations in use.
    pub const ANIM_COUNT: u32 = 0x9cabc;
    /// Its flags (bit 2: colour-scaled background, bit 25: no frame
    /// register in the GS set-up).
    pub const FLAGS: u32 = 0x67f08;
    /// Its colour scale, four floats (`yiRender_setupparamGS`).
    pub const COLOR_SCALE: u32 = 0x6820c;
    /// Its colour offset.
    pub const COLOR_OFFSET: u32 = 0x6821c;
    /// The background colour, RGB bytes.
    pub const BACK_COLOR: u32 = 0x68340;
    /// The frame buffer base (bytes; `>> 11` for the GS).
    pub const FRAME_BASE: u32 = 0x9cab4;
    /// The animation slots: `+0` flags (bit 0 in use; `u16` type at `+0`
    /// for keyframes), `+2` `u16` argument, `+4` `u16` kind, `+6` `u16`
    /// object id, `+8` keyframe index, `+0xc`/`+0x10` arguments, `+0x14`
    /// and `+0x18` the start time, `+0x24` the keyframe data.
    pub const ANIM: u32 = 0x9cac0;
    /// Bytes per slot.
    pub const ANIM_SIZE: u32 = 0x28;
    /// Slots.
    pub const ANIMS: u32 = 0x40;
    /// The keyframe data pointers, one per keyframe index
    /// (`yiKeyFrame_attachChr`).
    pub const KEYFRAMES: u32 = 0x9d4c4;
    /// Texture swaps in use (`yiTexSwap_init` clears it).
    pub const TEX_SWAP_COUNT: u32 = 0x9d9c4;
    /// The texture swap entries.
    pub const TEX_SWAP: u32 = 0x9d9c8;
    /// Bytes per entry.
    pub const TEX_SWAP_SIZE: u32 = 0x720;
    /// Entries.
    pub const TEX_SWAPS: u32 = 0x40;

    /// Offsets in a texture swap entry.
    pub mod tex_swap {
        /// `+0x4`: the frame shown (`yiTexSwapSetFrame`, with bit 0 of
        /// `+0`).
        pub const FRAME: u32 = 0x4;
        /// `+0x1c`: its frame count.
        pub const FRAMES: u32 = 0x1c;
    }
}

/// Fog and background colour (round 19).
pub mod fog {
    /// `graphicGetFogNear` (`0x006411d0`, returns in ST0).
    pub const GET_NEAR: u32 = 0x0064_11d0;
    /// `graphicGetFogFar` (`0x006411b0`, returns in ST0).
    pub const GET_FAR: u32 = 0x0064_11b0;
    /// `graphicGetFogLimit` (`0x006411c0`, returns in ST0).
    pub const GET_LIMIT: u32 = 0x0064_11c0;
    /// `graphicGetFogColor(r, g, b)` (`0x00641160`, float pointers).
    pub const GET_COLOUR: u32 = 0x0064_1160;
    /// `graphicGetGlobalAmbientColor(r, g, b)` (`0x00641230`), what
    /// `yiGetFogColor` reads instead with render status bit 2.
    pub const GET_AMBIENT: u32 = 0x0064_1230;
    /// `graphicSetFogColour(r, g, b)` (`0x00643150`, floats).
    pub const SET_COLOUR: u32 = 0x0064_3150;
    /// `graphicSetFogNear(x)` (`0x006431e0`).
    pub const SET_NEAR: u32 = 0x0064_31e0;
    /// `graphicSetFogFar(x)` (`0x006431a0`).
    pub const SET_FAR: u32 = 0x0064_31a0;
    /// `graphicSetFogLimit(x)` (`0x006431c0`).
    pub const SET_LIMIT: u32 = 0x0064_31c0;
    /// `0x00643180(x)`: another fog manager setter, given 1.0.
    pub const SET_FOG_643180: u32 = 0x0064_3180;
    /// `graphicSetBackgroundColour(r, g, b, a)` (`0x00642e90`).
    pub const SET_BACKGROUND: u32 = 0x0064_2e90;
    /// `graphicSetDistortionEnable(on)` (`0x00642f50`).
    pub const SET_DISTORTION: u32 = 0x0064_2f50;
    /// `MagicFile_GetMagicId()` (`0x009da3b0`).
    pub const MAGIC_GET_ID: u32 = 0x009d_a3b0;
    /// The magic whose background colour stays the game's (`0x2c2`).
    pub const MAGIC_KEEP_BACK: u32 = 0x2c2;
    /// The map whose fog is the PC renderer's (`0xe0`).
    pub const PC_FOG_MAP: u32 = 0xe0;
    /// The fog colour, RGBA bytes (`0x018df140`).
    pub const COLOUR: u32 = 0x018d_f140;
    /// The fog limit, near and far (`f32`s, `0x018df144`, `0x018df148`,
    /// `0x018df14c`), in scale-factor units.
    pub const LIMIT: u32 = 0x018d_f144;
    /// See [`LIMIT`].
    pub const NEAR: u32 = 0x018d_f148;
    /// See [`LIMIT`].
    pub const FAR: u32 = 0x018d_f14c;
    /// The scale factor (`f32`, `0x018deff8`).
    pub const SCALE: u32 = 0x018d_eff8;
    /// Two `f64` factors of the fog distances' units (`.rdata`).
    pub const UNIT: [u32; 2] = [0x00b5_01a0, 0x00b4_12f8];
    /// 255.0 (`f64`): colour bytes to `0..=1`, and the PC limit's scale.
    pub const BYTE_MAX: u32 = 0x00b1_70c0;
    /// The alpha factor (`f64`, `yiSetBackColor`).
    pub const ALPHA_SCALE: u32 = 0x00b1_70b8;
    /// The PC renderer's distance factor (`f64`, `yiSetFogParam`).
    pub const PC_DISTANCE: u32 = 0x00b4_7c68;
    /// The PC renderer's limit factor (`f64`, `yiSetFogParam`).
    pub const PC_LIMIT: u32 = 0x00b4_8318;
    /// The distortion (heat haze) word `yiDisableHamonEff` clears.
    pub const HAMON: u32 = 0x018d_ed28;
}

/// More renderer state (round 4: colours, the render state save area).
pub mod state {
    use ffx_bridge::Global;

    /// The renderer object (`yiRender_*`'s `this`; the texture swaps of
    /// `yiTexSwapSetFrame`).
    pub const RENDER: Global<u32> = Global::new(0x0187_6e00);
    /// The colour scale, four floats (`yiGetColorScale`, `yiSetColorScale`).
    pub const COLOR_SCALE: Global<[f32; 4]> = Global::new(0x018d_f00c);
    /// The battle centre, four floats divided by the scale factor
    /// (`yiSetBattleCenter`).
    pub const BATTLE_CENTER: Global<[u32; 4]> = Global::new(0x018d_f02c);
    /// A word saved with the render state (`0x018df140`).
    pub const RENDER_WORD: Global<u32> = Global::new(0x018d_f140);
    /// Three floats saved with the render state (`0x018df144`).
    pub const RENDER_FLOATS: Global<[u32; 3]> = Global::new(0x018d_f144);
    /// The colour shift, four floats (`yiSetColorShift` copies it out).
    pub const COLOR_SHIFT: Global<[u32; 4]> = Global::new(0x018d_f150);
    /// The background colour, RGBA bytes (`yiGetBackColor`).
    pub const BACK_COLOR: Global<[u8; 4]> = Global::new(0x018d_f178);
    /// 255.0 as an `f64` (`.rdata`, `yiGetBackColor`'s bound).
    pub const BACK_MAX: Global<f64> = Global::new(0x00b1_70c0);
    /// 255.0 as an `f32` (`.rdata`, what it stores past the bound).
    pub const BACK_MAX_F32: Global<f32> = Global::new(0x00b4_80d4);
    /// The render state save area (`yiPushRenderState`).
    pub const SAVED: Global<u32> = Global::new(0x0193_1230);
    /// Four floats a script native sets (`yiSetFieldVec4`).
    pub const FIELD_VEC4: Global<[u32; 4]> = Global::new(0x0193_1208);
    /// Six floats a script native sets (`yiSetFieldVec6`).
    pub const FIELD_VEC6: Global<[u32; 6]> = Global::new(0x0193_1218);
    /// Flags and floats two script natives set (`yiSetFieldPair`,
    /// `yiSetFieldVec3Int`).
    pub const FIELD_FLAGS: Global<u32> = Global::new(0x0193_4e78);
    /// The view matrix (`yiSetViewMatrix`).
    pub const VIEW_MATRIX: Global<[u32; 16]> = Global::new(0x018d_ed38);
    /// Two floats `yiSetViewMatrix` takes from `+0x20` of its matrix.
    pub const VIEW_EXTRA: Global<[u32; 2]> = Global::new(0x018d_f168);
    /// CLUTs in the first VRAM area (`yiClutAlloc_makepacket`: 32, or 16
    /// for texture mode 4).
    pub const CLUT_SPLIT: Global<u32> = Global::new(0x0193_bb14);
    /// Non-zero once a map is read (`yiCallFieldParticle`'s gate).
    pub const MAP_READ: Global<u32> = Global::new(0x0191_38a0);
    /// Non-zero: animations time by the vertical blank counter, else by
    /// fields (`yiAnimAttachDefault`).
    pub const USE_VCOUNT: Global<u32> = Global::new(0x0193_bb08);
    /// The image VRAM regions' limits and next addresses (`0x025d17a8`).
    pub const VRAM_REGIONS: Global<u32> = Global::new(0x025d_17a8);
    /// Per region (`yiVramAllocPsm` order): next and limit offsets from
    /// [`VRAM_REGIONS`]; region 0 is `yiVramAlloc`'s.
    pub const VRAM_REGION: [(u32, u32); 4] = [(0x14, 0x4), (0x10, 0x0), (0x18, 0x8), (0x1c, 0xc)];

    /// Offsets in the save area.
    pub mod saved {
        /// The render status.
        pub const STATUS: u32 = 0x0;
        /// The colour scale.
        pub const SCALE: u32 = 0x4;
        /// The colour offset.
        pub const OFFSET: u32 = 0x14;
        /// The word.
        pub const WORD: u32 = 0x24;
        /// The three floats.
        pub const FLOATS: u32 = 0x28;
        /// The colour shift.
        pub const SHIFT: u32 = 0x34;
        /// The display objects: per object its `+0x88` float, rotation,
        /// translation and matrix.
        pub const DOBJS: u32 = 0x44;
        /// Bytes per object.
        pub const DOBJ_SIZE: u32 = 100;
        /// Objects saved.
        pub const DOBJ_COUNT: u32 = 128;
    }
}

/// The renderer state where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{
        COLOR_OFFSET, COLOR3, DOBJ_COUNT, DOBJS, DRAW_MASK, MAP_DATA, RENDER_PARAM, RENDER_STATUS,
        SCALE_FACTOR, SG_FRAME, SG_VCOUNT, VIEW_ADJUST, VRAM_LIMIT, VRAM_NEXT, dobj, render,
    };
    use crate::x87;

    /// `yiGetRenderStatus()` (`0x00908f10`).
    pub fn yi_get_render_status(mem: &impl Mem) -> u32 {
        RENDER_STATUS.get(mem)
    }

    /// `yiRenderEnable(bits)` (`0x00909e20`).
    pub fn yi_render_enable(mem: &mut impl Mem, bits: u32) {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s | bits);
    }

    /// `yiRenderDisable(bits)` (`0x00909e10`).
    pub fn yi_render_disable(mem: &mut impl Mem, bits: u32) {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s & !bits);
    }

    /// `yiGetDrawMask()` (`0x00908960`).
    pub fn yi_get_draw_mask(mem: &impl Mem) -> u32 {
        DRAW_MASK.get(mem)
    }

    /// `yiSetDrawMask(mask)` (`0x0090a2b0`).
    pub fn yi_set_draw_mask(mem: &mut impl Mem, mask: u32) {
        DRAW_MASK.set(mem, mask);
    }

    /// `yiGetVCount()` (`0x00908f70`).
    pub fn yi_get_vcount(mem: &impl Mem) -> u32 {
        SG_VCOUNT.get(mem)
    }

    /// `yiGetFCount()` (`0x00908970`): the frame counter in fields (two a
    /// frame).
    pub fn yi_get_fcount(mem: &impl Mem) -> u32 {
        SG_FRAME.get(mem).wrapping_mul(2)
    }

    /// `yiGetMapStatus()` (`0x00908dd0`): the map data's status, -1 without
    /// map data.
    pub fn yi_get_map_status(mem: &impl Mem) -> u32 {
        match MAP_DATA.get(mem) {
            0 => u32::MAX,
            m => mem.read(m + 4),
        }
    }

    fn copy_matrix(mem: &mut impl Mem, index: u32, out: u32) {
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::MATRIX;
        let m = mem.read::<[u32; 16]>(at);
        mem.write(out, m);
    }

    /// `yiGetMatrixDObj(index, out)` (`0x00908df0`): copies the object's
    /// matrix; 0, or -1 when there is no such object.
    pub fn yi_get_matrix_dobj(mem: &mut impl Mem, index: u32, out: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        copy_matrix(mem, index, out);
        0
    }

    /// `yiGetMatrixGroup(group, out)` (`0x00908e30`): copies the matrix of
    /// the first object in `group`; 0, or -1 when none is.
    pub fn yi_get_matrix_group(mem: &mut impl Mem, group: u32, out: u32) -> i32 {
        let base = DOBJS.ptr(mem).addr;
        for i in 0..DOBJ_COUNT.get(mem) {
            let g = mem.read::<u16>(base + i * dobj::SIZE + dobj::GROUP);
            if u32::from(g) == group {
                copy_matrix(mem, i, out);
                return 0;
            }
        }
        -1
    }

    /// `yiSetRenderParam(v)` (`0x00908580`): render status bit 8 and the
    /// value.
    pub fn yi_set_render_param(mem: &mut impl Mem, v: u32) {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s | 0x100);
        RENDER_PARAM.set(mem, v);
    }

    /// `yiCheckDObj(index)` (`0x009086b0`): 0 for an object in use, else -1.
    pub fn yi_check_dobj(mem: &impl Mem, index: u32) -> i32 {
        if index < DOBJ_COUNT.get(mem) { 0 } else { -1 }
    }

    /// `yiSetColor3(r, g, b)` (`0x0090a400`): the low bytes; returns 0.
    pub fn yi_set_color3(mem: &mut impl Mem, r: u8, g: u8, b: u8) -> u32 {
        COLOR3.set(mem, [r, g, b]);
        0
    }

    /// `yiSetMatrixDObj(index, m)` (`0x0090a520`): 0, or -1 when there is
    /// no such object.
    pub fn yi_set_matrix_dobj(mem: &mut impl Mem, index: u32, m: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        put_matrix(mem, index, m);
        0
    }

    /// `yiSetMatrixGroup(group, m)` (`0x0090a560`): the first object in
    /// `group`; 0, or -1 when none is.
    pub fn yi_set_matrix_group(mem: &mut impl Mem, group: u32, m: u32) -> i32 {
        match find_group(mem, group) {
            Some(i) => {
                put_matrix(mem, i, m);
                0
            }
            None => -1,
        }
    }

    fn find_group(mem: &impl Mem, group: u32) -> Option<u32> {
        let base = DOBJS.ptr(mem).addr;
        (0..DOBJ_COUNT.get(mem))
            .find(|&i| u32::from(mem.read::<u16>(base + i * dobj::SIZE + dobj::GROUP)) == group)
    }

    fn put_matrix(mem: &mut impl Mem, index: u32, m: u32) {
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::MATRIX;
        let v = mem.read::<[u32; 16]>(m);
        mem.write(at, v);
    }

    /// `yiGetDObjGroup(index)` (`0x00908c50`): its group, -1 when there is
    /// no such object.
    pub fn yi_get_dobj_group(mem: &impl Mem, index: u32) -> u32 {
        if index >= DOBJ_COUNT.get(mem) {
            return u32::MAX;
        }
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::GROUP;
        u32::from(mem.read::<u16>(at))
    }

    /// `yiGetColorDObj(index, a, b)` (`0x00908e90`): its two colours, a
    /// byte per channel, widened to four words each.
    pub fn yi_get_color_dobj(mem: &mut impl Mem, index: u32, a: u32, b: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::COLORS;
        let c = mem.read::<[u8; 8]>(at);
        mem.write(a, [c[0], c[1], c[2], c[3]].map(u32::from));
        mem.write(b, [c[4], c[5], c[6], c[7]].map(u32::from));
        0
    }

    /// `yiSetColorDObj(index, a, b)` (`0x0090a5c0`): the low bytes of the
    /// four words of each.
    pub fn yi_set_color_dobj(mem: &mut impl Mem, index: u32, a: u32, b: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::COLORS;
        for (k, src) in [a, b].into_iter().enumerate() {
            for i in 0..4u32 {
                let v = mem.read::<u8>(src + 4 * i);
                mem.write(at + 4 * u32::try_from(k).unwrap_or(0) + i, v);
            }
        }
        0
    }

    /// `yiGetViewAdjust(out)` (`0x00908f20`).
    pub fn yi_get_view_adjust(mem: &mut impl Mem, out: u32) {
        let m = VIEW_ADJUST.get(mem);
        mem.write(out, m);
    }

    /// `yiSetViewAdjust(m)` (`0x0090a730`).
    pub fn yi_set_view_adjust(mem: &mut impl Mem, m: u32) {
        let v = mem.read::<[u32; 16]>(m);
        VIEW_ADJUST.set(mem, v);
    }

    /// `yiSetColorOffset(rgba)` (`0x00909ff0`): four ints, each less 128,
    /// as floats.
    pub fn yi_set_color_offset(mem: &mut impl Mem, rgba: u32) {
        for i in 0..4 {
            let c = mem.read::<i32>(rgba + 4 * i).wrapping_add(-0x80);
            let at = COLOR_OFFSET.ptr(mem).addr + 4 * i;
            #[allow(clippy::cast_precision_loss)] // fild then fstp m32
            mem.write(at, (c as f32).to_bits());
        }
    }

    /// `yiPkAddGsAD(q, reg, data)` (`0x0090ae20`, with identical copies at
    /// `0x0090bc40` and `0x00910040`).
    pub fn yi_pk_add_gs_ad(mem: &mut impl Mem, q: u32, reg: u32, data: u32) {
        mem.write(q, super::gs_ad(reg, data));
    }

    /// Appends a DMA "ref" tag (ID 3: `qwc` quadwords from `addr`) to the
    /// renderer's packet and counts it.
    fn add_ref(mem: &mut impl Mem, r: u32, addr: u32, qwc: u16) {
        let at = mem.read::<u32>(r.wrapping_add(render::PACKET));
        mem.write(at, [u32::from(qwc) | 0x3000_0000, addr, 0, 0]);
        mem.write(r.wrapping_add(render::PACKET), at.wrapping_add(0x10));
        let n = mem.read::<u32>(r.wrapping_add(render::PACKET_COUNT));
        mem.write(r.wrapping_add(render::PACKET_COUNT), n.wrapping_add(1));
    }

    /// `yiRender_addRef(render, addr, qwc)` (`0x0090c1f0`), `qwc` the low
    /// 16 bits.
    pub fn yi_render_add_ref(mem: &mut impl Mem, r: u32, addr: u32, qwc: u16) {
        add_ref(mem, r, addr, qwc);
    }

    /// `yiRender_addRefData(render, block)` (`0x0090c230`): a block whose
    /// first `u16` is its quadword count and whose data starts at `+0x10`.
    pub fn yi_render_add_ref_data(mem: &mut impl Mem, r: u32, block: u32) {
        let qwc = mem.read::<u16>(block);
        add_ref(mem, r, block.wrapping_add(0x10), qwc);
    }

    /// Copies 16 words one at a time, as `rep movsd`.
    fn copy16(mem: &mut impl Mem, dst: u32, src: u32) {
        for i in 0..16 {
            let v = mem.read::<u32>(src + 4 * i);
            mem.write(dst + 4 * i, v);
        }
    }

    /// `yiPkUnpackMatrix(m, out, addr)` (`0x0090cd10`): a VIF `UNPACK V4-32`
    /// of 4 quadwords to VU memory `addr` (`0x6c04xxxx` in the fourth word of
    /// a quadword of `NOP`s), then the matrix.
    pub fn yi_pk_unpack_matrix(mem: &mut impl Mem, m: u32, out: u32, addr: u32) {
        mem.write(out, [0u32, 0, 0, addr | 0x6c04_0000]);
        copy16(mem, out + 0x10, m);
    }

    /// `yiPkUnpackMatrixDma(m, out, addr)` (`0x0090ccb0`): the same behind a
    /// DMA tag of ID 2 (`next`) and 5 quadwords, and VIF `FLUSH`,
    /// `STCYCL 1,1`.
    pub fn yi_pk_unpack_matrix_dma(mem: &mut impl Mem, m: u32, out: u32, addr: u32) {
        mem.write(
            out,
            [
                0x2000_0005u32,
                0,
                0,
                0,
                0x1100_0000,
                0,
                0x0100_0101,
                addr | 0x6c04_0000,
            ],
        );
        copy16(mem, out + 0x20, m);
    }

    /// `yiVramAlloc(bytes)` (`0x00918130`): the 64-byte block address, or
    /// -1 when it does not fit.
    pub fn yi_vram_alloc(mem: &mut impl Mem, bytes: i32) -> u32 {
        let next = VRAM_NEXT.get(mem);
        let end = super::vram_end(next, bytes);
        if end > VRAM_LIMIT.get(mem) {
            return u32::MAX;
        }
        VRAM_NEXT.set(mem, end);
        next >> 6
    }

    /// `yiVramSetLimit(pages)` (`0x00919630`): returns 0.
    pub fn yi_vram_set_limit(mem: &mut impl Mem, pages: u32) -> u32 {
        VRAM_LIMIT.set(mem, super::vram_limit(pages));
        0
    }

    /// `yiSetScaleFactor(v)` (`0x0090a950`).
    pub fn yi_set_scale_factor(mem: &mut impl Mem, v: u32) {
        let s = mem.read::<[u32; 3]>(v).map(x87::copy);
        SCALE_FACTOR.set(mem, s);
    }

    /// `yiRender_resettex(render)` (`0x0090cea0`).
    pub fn yi_render_resettex(mem: &mut impl Mem, r: u32) {
        for o in render::TEX_CLEARED {
            mem.write(r.wrapping_add(o), 0u32);
        }
        mem.write(r.wrapping_add(render::TEX_NONE), u32::MAX);
    }

    /// `yiScreff_init(render)` (`0x00913f30`).
    pub fn yi_screff_init(mem: &mut impl Mem, r: u32) {
        mem.write(r.wrapping_add(render::SCREFF), 0u32);
    }

    // ---- round 4: colours, render state, textures ----

    use super::state;
    use ffx_bridge::x87::{Reg, ftol};
    use ffx_ps2::vu0::bridge::copy_vector;
    use ffx_system::stub::ptr32;

    /// `yiGetColorScale(out)` (`0x009088d0`): returns `FFXVu0CopyVector`'s
    /// `eax` (the last word).
    pub fn yi_get_color_scale(mem: &mut impl Mem, out: u32) -> u32 {
        let s = state::COLOR_SCALE.ptr(mem).addr;
        copy_vector(mem, out, s)
    }

    /// `yiSetColorScale(v)` (`0x0090a270`).
    pub fn yi_set_color_scale(mem: &mut impl Mem, v: u32) -> u32 {
        let s = state::COLOR_SCALE.ptr(mem).addr;
        copy_vector(mem, s, v)
    }

    /// `yiSetColorShift(out)` (`0x00908c30`, Fahrenheit's name): copies the
    /// colour shift *out*.
    pub fn yi_set_color_shift(mem: &mut impl Mem, out: u32) -> u32 {
        let s = state::COLOR_SHIFT.ptr(mem).addr;
        copy_vector(mem, out, s)
    }

    /// `yiSetBattleCenter(v)` (`0x0090a0e0`): `v` divided by the scale
    /// factor (four words from `g_YiScaleFactor`), each `fld; fdiv m32;
    /// fstp`.
    pub fn yi_set_battle_center(mem: &mut impl Mem, v: u32) -> u32 {
        let c = state::BATTLE_CENTER.ptr(mem).addr;
        let r = copy_vector(mem, c, v);
        let s = SCALE_FACTOR.ptr(mem).addr;
        for i in 0..4 {
            let x = mem.read::<f32>(c + 4 * i);
            let k = mem.read::<f32>(s + 4 * i);
            mem.write(c + 4 * i, Reg::load(x).div_m32(k).store());
        }
        r
    }

    /// `yiGetBattleCenter(out)` (`0x009087f0`): the centre times the scale
    /// factor.
    pub fn yi_get_battle_center(mem: &mut impl Mem, out: u32) -> u32 {
        let c = state::BATTLE_CENTER.ptr(mem).addr;
        let r = copy_vector(mem, out, c);
        let s = SCALE_FACTOR.ptr(mem).addr;
        for i in 0..4 {
            let x = mem.read::<f32>(out + 4 * i);
            let k = mem.read::<f32>(s + 4 * i);
            mem.write(out + 4 * i, Reg::load(x).mul_m32(k).store());
        }
        r
    }

    fn copy_float(mem: &mut impl Mem, dst: u32, src: u32) {
        let v = mem.read::<u32>(src);
        mem.write(dst, x87::copy(v));
    }

    fn copy_word(mem: &mut impl Mem, dst: u32, src: u32) {
        let v = mem.read::<u32>(src);
        mem.write(dst, v);
    }

    /// `yiPushRenderState()` (`0x00909c60`): the render status, colours and
    /// every display object's transform into the save area; returns the
    /// `eax` of the last `FFXVu0CopyVector`.
    pub fn yi_push_render_state(mem: &mut impl Mem) -> u32 {
        render_state(mem, true)
    }

    /// `yiPopRenderState()` (`0x00909b90`): the reverse.
    pub fn yi_pop_render_state(mem: &mut impl Mem) -> u32 {
        render_state(mem, false)
    }

    fn render_state(mem: &mut impl Mem, push: bool) -> u32 {
        let live = [
            state::RENDER_FLOATS.ptr(mem).addr,
            RENDER_STATUS.ptr(mem).addr,
            state::RENDER_WORD.ptr(mem).addr,
            state::COLOR_SHIFT.ptr(mem).addr,
            state::COLOR_SCALE.ptr(mem).addr,
            COLOR_OFFSET.ptr(mem).addr,
            DOBJS.ptr(mem).addr,
        ];
        let save = state::SAVED.ptr(mem).addr;
        let saved = [
            save + state::saved::FLOATS,
            save + state::saved::STATUS,
            save + state::saved::WORD,
            save + state::saved::SHIFT,
            save + state::saved::SCALE,
            save + state::saved::OFFSET,
            save + state::saved::DOBJS,
        ];
        let (src, dst) = if push { (live, saved) } else { (saved, live) };
        for i in 0..3 {
            copy_float(mem, dst[0] + 4 * i, src[0] + 4 * i);
        }
        copy_word(mem, dst[1], src[1]);
        copy_word(mem, dst[2], src[2]);
        let mut r = 0;
        for k in 3..6 {
            r = copy_vector(mem, dst[k], src[k]);
        }
        for i in 0..state::saved::DOBJ_COUNT {
            let o = live[6] + i * dobj::SIZE;
            let s = saved[6] + i * state::saved::DOBJ_SIZE;
            let pairs = [
                (s, o + dobj::WORD_88),
                (s + 4, o + dobj::ROTATION),
                (s + 0x14, o + dobj::TRANSLATION),
                (s + 0x24, o + dobj::MATRIX),
            ];
            let at = |p: (u32, u32), to_save: bool| if to_save { p } else { (p.1, p.0) };
            let (d, f) = at(pairs[0], push);
            copy_float(mem, d, f);
            let (d, f) = at(pairs[1], push);
            copy_vector(mem, d, f);
            let (d, f) = at(pairs[2], push);
            r = copy_vector(mem, d, f);
            let (d, f) = at(pairs[3], push);
            for w in 0..16 {
                copy_word(mem, d + 4 * w, f + 4 * w);
            }
        }
        r
    }

    /// `yiGetBackColor(out)` (`0x009086d0`): the background colour, four
    /// ints. With render status bits 0 and 2 each channel is
    /// `channel * scale + offset` (each step stored as `f32`), below 0 made
    /// 0, above 255.0 (an `f64` in `.rdata`) made 255.0 (an `f32` there),
    /// through `_ftol2_sse`; else the raw bytes. Returns the last word.
    pub fn yi_get_back_color(mem: &mut impl Mem, out: u32) -> u32 {
        let c = state::BACK_COLOR.get(mem);
        let status = RENDER_STATUS.get(mem);
        if status & 1 == 0 || status & 4 == 0 {
            let w = c.map(u32::from);
            mem.write(out, w);
            return w[3];
        }
        let mut v = c.map(f32::from);
        let top = state::BACK_MAX.get(mem);
        let top_f = state::BACK_MAX_F32.get(mem);
        let scale = state::COLOR_SCALE.get(mem);
        let offset = COLOR_OFFSET.get(mem);
        let mut last = 0;
        for i in 0..4 {
            let x = Reg::load(scale[i]).mul_m32(v[i]).store();
            let x = Reg::load(x).add_m32(f32::from_bits(offset[i])).store();
            v[i] = x;
            let lx = ffx_bridge::x87::load(x.to_bits());
            // fcomp with 0.0: below (not unordered) stores +0.0
            if lx < 0.0 {
                v[i] = 0.0;
            }
            let lv = ffx_bridge::x87::load(v[i].to_bits());
            // fcomp with 255.0: above (not equal, not unordered) stores 255.0f
            if lv > top {
                v[i] = top_f;
            }
            last = ftol(mem, ffx_bridge::x87::load(v[i].to_bits()));
            mem.write(out + 4 * u32::try_from(i).unwrap_or(0), last);
        }
        last
    }

    /// `yiImage_init(image)` (`0x009188c0`): returns `set_ptr32(0)`, 0.
    pub fn yi_image_init(mem: &mut impl Mem, im: u32) -> u32 {
        mem.write(im, 0u32);
        mem.write(im + 0x114, 0u32);
        mem.write(im + 0x110, 0u32);
        mem.write(im + 4, 0u8);
        mem.write(im + 0x144, 0u32);
        mem.write(im + 0x124, u32::MAX);
        mem.write(im + 0x160, 0u32);
        mem.write(im + 0x15c, 0u32);
        0
    }

    /// `yiTexSwap_init(render)` (`0x009152d0`): clears the texture swap
    /// count and its 64 entries (each eight words and 64 four-word slots
    /// of `0x1c` bytes); returns the pointer the loop ends on.
    pub fn yi_tex_swap_init(mem: &mut impl Mem, r: u32) -> u32 {
        mem.write(r.wrapping_add(render::TEX_SWAP_COUNT), 0u32);
        let mut end = 0;
        for i in 0..render::TEX_SWAPS {
            let e = r.wrapping_add(render::TEX_SWAP + i * render::TEX_SWAP_SIZE);
            mem.write(e, [0u32; 8]);
            for k in 0..64 {
                mem.write(e.wrapping_add(0x20 + k * 0x1c), [0u32; 4]);
            }
            end = e.wrapping_add(4 + 0x20 + 64 * 0x1c);
        }
        end
    }

    /// `yiTexSwapSetFrame(index, frame)` (`0x00909e30`, script native
    /// `0x008574c0`): for an entry below the count and a frame below its
    /// frame count, sets its bit 0 and the frame; returns 0.
    pub fn yi_tex_swap_set_frame(mem: &mut impl Mem, index: u32, frame: u32) -> u32 {
        let r = state::RENDER.ptr(mem).addr;
        if index < mem.read::<u32>(r + render::TEX_SWAP_COUNT) {
            let e = r.wrapping_add(render::TEX_SWAP + index.wrapping_mul(render::TEX_SWAP_SIZE));
            if frame < mem.read::<u32>(e.wrapping_add(render::tex_swap::FRAMES)) {
                let f = mem.read::<u32>(e);
                mem.write(e, f | 1);
                mem.write(e.wrapping_add(render::tex_swap::FRAME), frame);
            }
        }
        0
    }

    fn clock(mem: &impl Mem) -> u32 {
        if state::USE_VCOUNT.get(mem) == 0 {
            yi_get_fcount(mem)
        } else {
            yi_get_vcount(mem)
        }
    }

    fn free_anim(mem: &impl Mem, r: u32) -> Option<u32> {
        (0..render::ANIMS).find(|&i| {
            let a = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
            mem.read::<u8>(a) & 1 == 0
        })
    }

    /// `yiAnimAttachDefault_009143c0(render, id, a, b)`: for an id below
    /// the object count not attached yet, the first free animation slot
    /// (type 4, flags 5, both times the current clock); always returns 0.
    pub fn yi_anim_attach_default(mem: &mut impl Mem, r: u32, id: u32, a: u32, b: u32) -> u32 {
        if id >= mem.read::<u32>(r.wrapping_add(render::ANIM_IDS)) {
            return 0;
        }
        for i in 0..render::ANIMS {
            let s = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
            if mem.read::<u8>(s) & 1 != 0 && u32::from(mem.read::<u16>(s + 6)) == id {
                return 0;
            }
        }
        let now = clock(mem);
        let Some(i) = free_anim(mem, r) else { return 0 };
        let s = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
        mem.write(s + 4, 4u16);
        mem.write(s + 0xc, a);
        mem.write(s + 0x10, b);
        mem.write(s + 0x14, now);
        mem.write(s + 0x18, now);
        mem.write(s, 5u32);
        #[allow(clippy::cast_possible_truncation)] // the low 16 bits
        mem.write(s + 6, id as u16);
        mem.write(s + 8, 0u32);
        mem.write(s + 0x24, 0u32);
        let n = mem.read::<u32>(r.wrapping_add(render::ANIM_COUNT));
        mem.write(r.wrapping_add(render::ANIM_COUNT), n.wrapping_add(1));
        0
    }

    /// `yiKeyFrame_attachChr(render, id, keyframe, arg)` (`0x00914630`):
    /// with keyframe data `keyframe` and an id below the texture swap
    /// count, the first free animation slot (type 1); returns the `eax` it
    /// ends with: the clock, 64 when no slot is free, else the keyframe
    /// data.
    pub fn yi_key_frame_attach_chr(mem: &mut impl Mem, r: u32, id: u32, kf: u32, arg: u16) -> u32 {
        let now = clock(mem);
        let table = r.wrapping_add(render::KEYFRAMES + kf.wrapping_mul(4));
        let data = mem.read::<u32>(table);
        if data == 0 || id >= mem.read::<u32>(r.wrapping_add(render::TEX_SWAP_COUNT)) {
            return now;
        }
        let Some(i) = free_anim(mem, r) else {
            return render::ANIMS;
        };
        let s = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
        mem.write(s, 1u16);
        mem.write(s + 2, arg);
        let ty = mem.read::<u16>(data.wrapping_add(8));
        mem.write(s + 4, ty);
        #[allow(clippy::cast_possible_truncation)] // the low 16 bits
        {
            mem.write(s + 6, id as u16);
            mem.write(s + 8, kf as u16);
        }
        mem.write(s + 0xa, 0u16);
        mem.write(s + 0x14, now);
        mem.write(s + 0x18, now);
        let data = mem.read::<u32>(table);
        mem.write(s + 0x24, data);
        let n = mem.read::<u32>(r.wrapping_add(render::ANIM_COUNT));
        mem.write(r.wrapping_add(render::ANIM_COUNT), n.wrapping_add(1));
        data
    }

    /// `yiSetFieldVec4i(a, b, c, d)` (`0x0090a1d0`): four ints as floats
    /// (`fild; fstp`).
    pub fn yi_set_field_vec4i(mem: &mut impl Mem, v: [i32; 4]) {
        let at = state::FIELD_VEC4.ptr(mem).addr;
        #[allow(clippy::cast_precision_loss)] // fild rounds to f32 on the store
        mem.write(at, v.map(|x| x87::store(f64::from(x))));
    }

    /// `yiSetFieldVec4(a, b, c, d)` (`0x0090a200`, script native
    /// `0x0091c600`): four floats.
    pub fn yi_set_field_vec4(mem: &mut impl Mem, v: [u32; 4]) {
        let at = state::FIELD_VEC4.ptr(mem).addr;
        mem.write(at, v.map(x87::copy));
    }

    /// `yiSetFieldVec6(a..f)` (`0x0090a230`, script native `0x0091c6b0`):
    /// six floats.
    pub fn yi_set_field_vec6(mem: &mut impl Mem, v: [u32; 6]) {
        let at = state::FIELD_VEC6.ptr(mem).addr;
        mem.write(at, v.map(x87::copy));
    }

    /// `yiSetFieldPair(on, a, b)` (`0x0090a430`, script native
    /// `0x008591d0`): two floats; flags bit 0, and bit 1 when `on`;
    /// returns 0.
    pub fn yi_set_field_pair(mem: &mut impl Mem, on: u32, a: u32, b: u32) -> u32 {
        let base = state::FIELD_FLAGS.ptr(mem).addr;
        let f = mem.read::<u32>(base);
        mem.write(base + 4, x87::copy(a));
        mem.write(base + 0x24, x87::copy(b));
        mem.write(base, f | if on == 0 { 1 } else { 3 });
        0
    }

    /// `yiSetFieldVec3Int(a, b, c, n)` (`0x0090a470`, script native
    /// `0x00858c90`): three floats and an int; returns 0.
    pub fn yi_set_field_vec3_int(mem: &mut impl Mem, a: u32, b: u32, c: u32, n: u32) -> u32 {
        let base = state::FIELD_FLAGS.ptr(mem).addr;
        mem.write(base + 0x18, x87::copy(a));
        mem.write(base + 0x28, n);
        mem.write(base + 0x1c, x87::copy(b));
        mem.write(base + 0x20, x87::copy(c));
        0
    }

    /// `yiMapGetSection(map, index)` (`0x00907f00`): for data starting
    /// with `MAP1`, its section `index` (an offset at `+0x10 + 4 * index`)
    /// as a pointer; 0 for another tag or an empty offset.
    pub fn yi_map_get_section(mem: &impl Mem, map: u32, index: u32) -> u32 {
        if mem.read::<[u8; 4]>(map) != *b"MAP1" {
            return 0;
        }
        match mem.read::<u32>(map.wrapping_add(0x10).wrapping_add(index.wrapping_mul(4))) {
            0 => 0,
            o => o.wrapping_add(map),
        }
    }

    /// `yiVramAllocPsm(bytes, psm)` (`0x00918090`): like `yiVramAlloc`
    /// from the main region, then the second; for PSMT8 (`0x13`) and PSMT4
    /// (`0x14`) images four or eight times the size from the third or the
    /// fourth (in their units); -1 when none fits.
    pub fn yi_vram_alloc_psm(mem: &mut impl Mem, bytes: i32, psm: u32) -> u32 {
        let size = super::vram_end(0, bytes);
        if let Some(a) = vram_take(mem, 0, size).or_else(|| vram_take(mem, 1, size)) {
            return a;
        }
        let n = match psm {
            0x13 => size.wrapping_shl(2),
            0x14 => size.wrapping_shl(3),
            _ => return u32::MAX,
        };
        vram_take(mem, 2, n)
            .or_else(|| vram_take(mem, 3, n))
            .unwrap_or(u32::MAX)
    }

    /// Takes `n` bytes from VRAM region `i`: its old next address `>> 6`.
    fn vram_take(mem: &mut impl Mem, i: u32, n: u32) -> Option<u32> {
        let regions = state::VRAM_REGIONS.ptr(mem).addr;
        let (next, limit) = state::VRAM_REGION[i as usize];
        let (next, limit) = (regions + next, regions + limit);
        let at = mem.read::<u32>(next);
        let end = at.wrapping_add(n);
        if end > mem.read::<u32>(limit) {
            return None;
        }
        mem.write(next, end);
        Some(at >> 6)
    }

    /// `yiAttachChrDObj(id, a, b)` (`0x00908350`):
    /// [`yi_anim_attach_default`] on the renderer; returns 0.
    pub fn yi_attach_chr_dobj(mem: &mut impl Mem, id: u32, a: u32, b: u32) -> u32 {
        let r = state::RENDER.ptr(mem).addr;
        yi_anim_attach_default(mem, r, id, a, b)
    }

    /// `yiCallFieldParticle()` (`0x009083e0`): with a map read
    /// (`0x019138a0`) and render status bit 0, the field particles run
    /// (`pppFpLoop`, `fp_loop`, a tail jump).
    pub fn yi_call_field_particle<M: Mem>(mem: &mut M, fp_loop: impl FnOnce(&mut M)) {
        if state::MAP_READ.get(mem) != 0 && RENDER_STATUS.get(mem) & 1 != 0 {
            fp_loop(mem);
        }
    }

    /// `yiClutAlloc_makepacket(packet, cluts, n)` (`0x00917050`): the CLUT
    /// upload packet: a VIF `DIRECT` of `n * 0x46` quadwords, then 72
    /// transfers of `0x460` bytes from `+0x10`, each a GIF A+D tag of four
    /// registers (`BITBLTBUF`, `TRXPOS`, `TRXREG` 16 x 16, `TRXDIR`; the
    /// PC build leaves their values 0), an `IMAGE` tag of 64 quadwords and
    /// the next 256-colour CLUT from `cluts` (left alone when null). The
    /// texture mode (`+0x67f20`: 2, 3 → 32, 4 → 16) sets how many CLUTs
    /// go to the first VRAM area (`0x0193bb14`, which the PC build does
    /// not otherwise use). The packet (`0x13b10` bytes, its end count at
    /// `+0x13b10`) is copied to the second buffer at `+0x13b14` and the
    /// count to `+0x676a4`. Returns 0.
    pub fn yi_clut_alloc_makepacket(mem: &mut impl Mem, pk: u32, cluts: u32, n: u32) -> u32 {
        match mem.read::<u32>(pk.wrapping_add(0x67f20)) {
            2 | 3 => state::CLUT_SPLIT.set(mem, 0x20),
            4 => state::CLUT_SPLIT.set(mem, 0x10),
            _ => {}
        }
        let qwc = n.wrapping_mul(0x46);
        mem.write(pk.wrapping_add(0xc), qwc | 0x5100_0000);
        mem.write(pk, [0u32; 3]);
        let mut src = cluts;
        for i in 0..72u32 {
            let rec = pk.wrapping_add(0x10 + i * 0x460);
            mem.write(
                rec,
                [
                    4u32,
                    0x1003_0000,
                    0xe,
                    0,
                    0,
                    0,
                    0x50,
                    0,
                    0,
                    0,
                    0x51,
                    0,
                    0x10,
                    0,
                    0x52,
                    0,
                    0,
                    0,
                    0x53,
                    0,
                    0x8040,
                    0x0800_0000,
                    0,
                    0,
                ],
            );
            if src != 0 {
                let c = mem.read::<[u32; 256]>(src);
                mem.write(rec.wrapping_add(0x60), c);
                src = src.wrapping_add(0x400);
            }
        }
        let end = qwc.wrapping_add(1);
        mem.write(pk.wrapping_add(0x13b10), end);
        let mut buf = vec![0u8; 0x13b10];
        mem.read_bytes(pk, &mut buf);
        mem.write_bytes(pk.wrapping_add(0x13b14), &buf);
        let end = mem.read::<u32>(pk.wrapping_add(0x13b10));
        mem.write(pk.wrapping_add(0x676a4), end);
        0
    }

    /// The renderer's background colour as the GS `BGCOLOR`-style word
    /// `b << 16 | g << 8 | r`: the raw bytes (`+0x68340`), or with flag
    /// bit 2 (`+0x67f08`) each scaled and offset by the renderer's own
    /// colour scale and offset (`+0x6820c`, `+0x6821c`), clamped to
    /// 0..=255 (as `yiGetBackColor`) and converted with `_ftol2`.
    fn render_back_color(mem: &impl Mem, r: u32) -> u32 {
        let c = mem.read::<[u8; 3]>(r.wrapping_add(render::BACK_COLOR));
        if mem.read::<u8>(r.wrapping_add(render::FLAGS)) & 4 == 0 {
            return u32::from(c[2]) << 16 | u32::from(c[1]) << 8 | u32::from(c[0]);
        }
        let top = state::BACK_MAX.get(mem);
        let top_f = state::BACK_MAX_F32.get(mem);
        let ch = |i: u32| -> u32 {
            let s = mem.read::<f32>(r.wrapping_add(render::COLOR_SCALE + 4 * i));
            let o = mem.read::<f32>(r.wrapping_add(render::COLOR_OFFSET + 4 * i));
            let x = Reg::load(s).mul_m32(f32::from(c[i as usize])).store();
            let mut v = Reg::load(o).add_m32(x).store();
            if ffx_bridge::x87::load(v.to_bits()) < 0.0 {
                v = 0.0;
            }
            if ffx_bridge::x87::load(v.to_bits()) > top {
                v = top_f;
            }
            ffx_bridge::x87::ftol2(ffx_bridge::x87::load(v.to_bits()))
        };
        let (red, green, blue) = (ch(0), ch(1), ch(2));
        blue.wrapping_shl(16) | green.wrapping_shl(8) | red
    }

    /// `yiRender_setupparamGS(render)` (`0x0090d260`): the two GS
    /// set-up packets (`+0x677a8`, `+0x678a8`): a DMA tag of 12
    /// quadwords, VIF `FLUSH` and `DIRECT` 11, a GIF A+D tag of 10
    /// registers (`PRMODECONT`, `TEST`, `COLCLAMP`, `DTHE`, ..., the
    /// background colour, `FOGCOL`-style `0x3d`, `0x3b`, `0x49`, and the
    /// frame register `0x4f` from `+0x9cab4 >> 11` unless flag bit 25 is
    /// set). The two differ only in `0x7000d` / `0x5000d`.
    pub fn yi_render_setupparam_gs(mem: &mut impl Mem, r: u32) {
        let color = render_back_color(mem, r);
        let flags = mem.read::<u32>(r.wrapping_add(render::FLAGS));
        let (frame, frame_reg) = if flags & 0x0200_0000 == 0 {
            let fb = mem.read::<u32>(r.wrapping_add(render::FRAME_BASE));
            ((fb >> 11) | 0x0100_0000, 0x4f)
        } else {
            (0, 0)
        };
        for (at, mode) in [(0x677a8, 0x0007_000du32), (0x678a8, 0x0005_000d)] {
            let words: [u32; 52] = [
                0x2000_000c,
                0,
                0,
                0,
                0x1100_0000,
                0,
                0,
                0x5100_000b,
                0x800a,
                0x1000_0000,
                0xe,
                0,
                0,
                0,
                0x3f,
                0,
                mode,
                0,
                0x48,
                0,
                0x60,
                0,
                0x15,
                0,
                0x44,
                0,
                0x43,
                0,
                0,
                0,
                0x4b,
                0,
                0,
                0,
                10,
                0,
                color,
                0,
                0x3d,
                0,
                0x80,
                0,
                0x3b,
                0,
                0,
                0,
                0x49,
                0,
                frame,
                0,
                frame_reg,
                0,
            ];
            mem.write(r.wrapping_add(at), words);
        }
    }

    /// `yiArcPrintSize()` (`0x0090e6f0`): prints the archive section sizes
    /// through `dbgPrintf`, empty on PC; returns 0.
    #[must_use]
    pub const fn yi_arc_print_size() -> u32 {
        0
    }

    /// `yiSetViewMatrix(m)` (`0x0090a980`): unless render status bit 11 is
    /// set, the view matrix (`0x018ded38`, `FFXVu0CopyMatrix`) and the
    /// floats at `m +0x20`, `+0x24` into `0x018df168`.
    pub fn yi_set_view_matrix(mem: &mut impl Mem, m: u32) {
        if RENDER_STATUS.get(mem) & 0x800 != 0 || m == 0 {
            return;
        }
        let at = state::VIEW_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, at, m);
        let a = mem.read::<u32>(m + 0x20);
        let b = mem.read::<u32>(m + 0x24);
        let at = state::VIEW_EXTRA.ptr(mem).addr;
        mem.write(at, [x87::copy(a), x87::copy(b)]);
    }

    /// `yiPolyBufClear(buf)` (`0x00911970`): zeroes the first `0x4000`
    /// bytes, the counters at `+0x4000`/`+0x4008` and the words
    /// `+0x4048..=+0x4080`, and the `u16` at `+0x4084`. Returns `buf`.
    pub fn yi_poly_buf_clear(mem: &mut impl Mem, buf: u32) -> u32 {
        for row in 0..0x400u32 {
            let at = buf.wrapping_add(8 + 16 * row);
            for d in [4u32, 0, 0u32.wrapping_sub(4), 0u32.wrapping_sub(8)] {
                mem.write(at.wrapping_add(d), 0u32);
            }
        }
        for off in [
            0x4000u32, 0x4008, 0x4054, 0x4050, 0x404c, 0x4048, 0x4064, 0x4060, 0x405c, 0x4058,
            0x4074, 0x4078, 0x406c, 0x4070, 0x407c, 0x4080,
        ] {
            mem.write(buf.wrapping_add(off), 0u32);
        }
        mem.write(buf.wrapping_add(0x4084), 0u16);
        buf
    }

    /// The colour bytes of three GIF `RGBAQ` quadwords from `gif + 16 *
    /// first` (`r g b a` each) to `out`, the alphas first set to `alpha`
    /// (none when `first + 3` wraps); `+0x12` cleared. Returns the last.
    fn tri_colors(mem: &mut impl Mem, out: u32, gif: u32, first: u32, alpha: u32) -> u32 {
        mem.write(out.wrapping_add(0x12), 0u16);
        let end = first.wrapping_add(3);
        if first < end {
            let mut at = first.wrapping_mul(16).wrapping_add(0xc).wrapping_add(gif);
            for _ in first..end {
                mem.write(at, alpha);
                at = at.wrapping_add(16);
            }
        }
        let q = first.wrapping_mul(16).wrapping_add(gif);
        let mut last = 0;
        for i in 0..12u32 {
            last = mem.read::<u8>(q.wrapping_add(4 * i));
            mem.write(out.wrapping_add(i), last);
        }
        u32::from(last)
    }

    /// `yiTriColorsFromGif(out, gif, first)` (`0x00910340`): a triangle's
    /// three colours (`r g b a`, 12 bytes) from the GIF packet's `RGBAQ`
    /// quadwords at `first`, their alphas first set to 0x80; `+0x12`
    /// cleared. Returns the last byte copied.
    pub fn yi_tri_colors_from_gif(mem: &mut impl Mem, out: u32, gif: u32, first: u32) -> u32 {
        tri_colors(mem, out, gif, first, 0x80)
    }

    /// `yiTriColorsUvFromGif(out, gif, first, uv)` (`0x009103e0`): as
    /// [`yi_tri_colors_from_gif`] with alpha 0x7f, then the three `UV`
    /// quadwords at `uv` (`u16` u and v each) to `+0x14`. Returns the last
    /// `u16` copied.
    pub fn yi_tri_colors_uv_from_gif(
        mem: &mut impl Mem,
        out: u32,
        gif: u32,
        first: u32,
        uv: u32,
    ) -> u32 {
        tri_colors(mem, out, gif, first, 0x7f);
        let q = uv.wrapping_mul(16).wrapping_add(gif);
        let mut last = 0;
        for (i, off) in [0u32, 4, 0x10, 0x14, 0x20, 0x24].into_iter().enumerate() {
            last = mem.read::<u16>(q.wrapping_add(off));
            mem.write(
                out.wrapping_add(0x14 + 2 * u32::try_from(i).unwrap_or(0)),
                last,
            );
        }
        u32::from(last)
    }

    /// `yiLoadMatrixChunk(scene, chunk)` (`0x0090ed10`): `+4` of the chunk
    /// to `+0x6823c`, then 64 floats from `+0x40` to `+0x68240` (`fld` /
    /// `fstp`, so [`x87::copy`]).
    pub fn yi_load_matrix_chunk(mem: &mut impl Mem, scene: u32, chunk: u32) {
        let w = mem.read::<u32>(chunk.wrapping_add(4));
        mem.write(scene.wrapping_add(0x6823c), w);
        for i in 0..64u32 {
            let v = mem.read::<u32>(chunk.wrapping_add(0x40 + 4 * i));
            mem.write(scene.wrapping_add(0x68240 + 4 * i), x87::copy(v));
        }
    }

    /// `yiRender_setupFogParam(scene)` (`0x0090d1b0`): from the fog scale
    /// (`+0x681f8`, times 1/32 and 32) and the near/far (`+0x68348`,
    /// `+0x6834c`): `+0x68350` = -255 / (far - near) (0 when equal),
    /// `+0x68354` = 255 - `+0x68344` (255 with flag 8 at `+0x67f08`),
    /// `+0x68358` = -near, `+0x6835c` = 0.
    pub fn yi_render_setup_fog_param(mem: &mut impl Mem, scene: u32) {
        let ld = |mem: &_, off: u32| {
            ffx_bridge::x87::load(Mem::read::<u32>(mem, scene.wrapping_add(off)))
        };
        let k = |mem: &_, at: u32| {
            let a = Mem::rebase(mem, at);
            Mem::read::<f64>(mem, a)
        };
        let s = ld(&*mem, 0x681f8) * k(&*mem, super::FOG_EIGHTH) * k(&*mem, super::FOG_SCALE32);
        let far = f64::from(f32::from_bits(x87::store(ld(&*mem, 0x6834c) * s)));
        let near = f64::from(f32::from_bits(x87::store(s * ld(&*mem, 0x68348))));
        let c255 = k(&*mem, super::FOG_255);
        #[allow(clippy::float_cmp)] // fucomp: exact
        let r = if near == far {
            0
        } else {
            x87::store(-(c255 / (far - near)))
        };
        mem.write(scene.wrapping_add(0x68350), r);
        let p = ld(&*mem, 0x68344);
        mem.write(scene.wrapping_add(0x68354), x87::store(c255 - p));
        mem.write(scene.wrapping_add(0x68358), x87::store(-near));
        mem.write(scene.wrapping_add(0x6835c), 0u32);
        if mem.read::<u8>(scene.wrapping_add(0x67f08)) & 8 != 0 {
            let a = mem.rebase(super::FOG_255_F32);
            let v = mem.read::<u32>(a);
            mem.write(scene.wrapping_add(0x68354), x87::copy(v));
        }
    }

    /// The fields `yiObjSave`/`yiObjLoad` copy: (object offset, record
    /// offset, size: 2 a `u16`, 4 a word, 0 a float through `fld`/`fstp`).
    const OBJ_RECORD: [(u32, u32, u8); 18] = [
        (0x80, 0, 2),
        (0x82, 2, 2),
        (0x84, 4, 2),
        (0x88, 8, 4),
        (0x86, 0x30, 2),
        (0x50, 0x10, 0),
        (0x54, 0x14, 0),
        (0x58, 0x18, 0),
        (0x5c, 0x1c, 0),
        (0x60, 0x20, 0),
        (0x64, 0x24, 0),
        (0x68, 0x28, 0),
        (0x6c, 0x2c, 0),
        (0x8c, 0x34, 4),
        (0x90, 0x38, 2),
        (0x92, 0x3a, 2),
        (0x94, 0x3c, 2),
        (0x96, 0x3e, 2),
    ];

    /// `yiObjSave(record, obj)` (`0x0090ee60`): the object's state to a
    /// record ([`OBJ_RECORD`]; `+0x84` widened to a word). Returns the
    /// last `u16`.
    pub fn yi_obj_save(mem: &mut impl Mem, rec: u32, obj: u32) -> u32 {
        let mut last = 0;
        for (o, r, size) in OBJ_RECORD {
            let (from, to) = (obj.wrapping_add(o), rec.wrapping_add(r));
            match (size, o) {
                (2, 0x84) => mem.write(to, u32::from(mem.read::<u16>(from))),
                (2, _) => {
                    let v = mem.read::<u16>(from);
                    last = u32::from(v);
                    mem.write(to, v);
                }
                (4, _) => mem.write(to, mem.read::<u32>(from)),
                _ => mem.write(to, x87::copy(mem.read::<u32>(from))),
            }
        }
        last
    }

    /// `yiObjLoad(obj, record)` (`0x0090edb0`): the reverse of
    /// [`yi_obj_save`] (`+0x84` from the record's low half). Returns the
    /// last `u16`.
    pub fn yi_obj_load(mem: &mut impl Mem, obj: u32, rec: u32) -> u32 {
        let mut last = 0;
        for (o, r, size) in OBJ_RECORD {
            let (from, to) = (rec.wrapping_add(r), obj.wrapping_add(o));
            match size {
                2 => {
                    let v = mem.read::<u16>(from);
                    last = u32::from(v);
                    mem.write(to, v);
                }
                4 => mem.write(to, mem.read::<u32>(from)),
                _ => mem.write(to, x87::copy(mem.read::<u32>(from))),
            }
        }
        last
    }

    /// `yiWindowSetHandlers(window)` (`0x00907d80`): the six handlers of
    /// the window's kind (`+4`, `i16`; 6 words a kind from `0x00c5d788`) to
    /// `+0x60`..`+0x70` and `+0x78`. Returns the last.
    pub fn yi_window_set_handlers(mem: &mut impl Mem, window: u32) -> u32 {
        let mut last = 0;
        for (i, off) in [0x60u32, 0x64, 0x68, 0x6c, 0x70, 0x78]
            .into_iter()
            .enumerate()
        {
            let kind = i32::from(mem.read::<i16>(window.wrapping_add(4))).cast_unsigned();
            let table = mem.rebase(super::WINDOW_HANDLERS);
            let at = kind
                .wrapping_mul(0x18)
                .wrapping_add(table)
                .wrapping_add(4 * u32::try_from(i).unwrap_or(0));
            last = mem.read::<u32>(at);
            mem.write(window.wrapping_add(off), last);
        }
        last
    }

    /// `yiCameraLoad(camera, record)` (`0x0090ef10`): a camera record from
    /// the map file: `u16`s to `+0x30`, `+0x32`, `+0x36`, `+0x34`, `+0x38`
    /// a float, `+0x48` a word, and twelve floats (`+0x10..`) to `+0`.
    /// Returns the last `u16` (`+6`).
    pub fn yi_camera_load(mem: &mut impl Mem, cam: u32, rec: u32) -> u32 {
        let h = |mem: &mut _, d: u32, s: u32| {
            let v = Mem::read::<u16>(mem, rec.wrapping_add(s));
            Mem::write(mem, cam.wrapping_add(d), v);
            u32::from(v)
        };
        h(mem, 0x30, 0);
        h(mem, 0x32, 2);
        h(mem, 0x36, 4);
        let f = x87::copy(mem.read::<u32>(rec.wrapping_add(8)));
        mem.write(cam.wrapping_add(0x38), f);
        let w = mem.read::<u32>(rec.wrapping_add(0xc));
        mem.write(cam.wrapping_add(0x48), w);
        let last = h(mem, 0x34, 6);
        for i in 0..12u32 {
            let f = x87::copy(mem.read::<u32>(rec.wrapping_add(0x10 + 4 * i)));
            mem.write(cam.wrapping_add(4 * i), f);
        }
        last
    }

    /// `yiFogLoad(scene, record)` (`0x0090f090`): the fog record: `+0xc`
    /// to `+0x68340`, the floats `+0x10..=+0x18` to `+0x68344..`, `+0x1c`
    /// and `+0x20` to `+0xba400`/`+0xba404`, the colour words `+0..=+8` to
    /// `+0x68378..`, the floats `+0x30..=+0x3c` to `+0x6822c..`. Returns
    /// the colour's last word.
    pub fn yi_fog_load(mem: &mut impl Mem, scene: u32, rec: u32) -> u32 {
        let w = mem.read::<u32>(rec.wrapping_add(0xc));
        mem.write(scene.wrapping_add(0x68340), w);
        for (d, s) in [
            (0x68344u32, 0x10u32),
            (0x68348, 0x14),
            (0x6834c, 0x18),
            (0xba400, 0x1c),
            (0xba404, 0x20),
        ] {
            let f = x87::copy(mem.read::<u32>(rec.wrapping_add(s)));
            mem.write(scene.wrapping_add(d), f);
        }
        let mut last = 0;
        for i in 0..3u32 {
            last = mem.read::<u32>(rec.wrapping_add(4 * i));
            mem.write(scene.wrapping_add(0x68378 + 4 * i), last);
        }
        for i in 0..4u32 {
            let f = x87::copy(mem.read::<u32>(rec.wrapping_add(0x30 + 4 * i)));
            mem.write(scene.wrapping_add(0x6822c + 4 * i), f);
        }
        last
    }

    /// `yiWindow*` handlers (window kinds of [`super::WINDOW_HANDLERS`]):
    /// the window's `+0` (`u16`, active), `+6` (`i16`, opening progress
    /// in 1/4096), `+8` (`u16`, state: 2 open, 4 closed), `+0xc`/`+0xe`
    /// (`i16`, frames left and the reload), `+0x10` (`i16`, height),
    /// `+0x18..=+0x24` (the rectangle animated from), `+0x28..=+0x34` (the
    /// rectangle: x0, y0, x1, y1), `+0x38`/`+0x3c` (the per-frame steps),
    /// `+0x48..=+0x54` (the rectangle drawn), `+0x58` (colour), the
    /// handlers `+0x60..` and the current one `+0x74`. Each returns 1 when
    /// it moved to the next state.
    fn win_copy(mem: &mut impl Mem, w: u32, to: u32, from: u32) {
        let v = x87::copy(mem.read::<u32>(w.wrapping_add(from)));
        mem.write(w.wrapping_add(to), v);
    }

    fn win_const(mem: &impl Mem, c: u32) -> u32 {
        let at = mem.rebase(c);
        x87::copy(mem.read::<u32>(at))
    }

    fn win_f64(mem: &impl Mem, c: u32) -> f64 {
        let at = mem.rebase(c);
        mem.read::<f64>(at)
    }

    fn win_ld(mem: &impl Mem, w: u32, off: u32) -> f64 {
        ffx_bridge::x87::load(mem.read::<u32>(w.wrapping_add(off)))
    }

    /// The state change of a finished step: state `state`, the handler
    /// from slot `slot`. Returns 1.
    fn win_next(mem: &mut impl Mem, w: u32, state: u16, slot: u32) -> u32 {
        mem.write(w.wrapping_add(8), state);
        let h = mem.read::<u32>(w.wrapping_add(slot));
        mem.write(w.wrapping_add(0x74), h);
        1
    }

    /// A slide step: `+field` plus (or minus) the step at `+step`, one
    /// frame (`+0xc`) less; when none is left, the frames reloaded from
    /// `+0xe` (`reload`) and [`win_next`].
    #[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)] // the variants, as flags
    fn win_step(
        mem: &mut impl Mem,
        w: u32,
        field: u32,
        step: u32,
        add: bool,
        reload: bool,
        state: u16,
        slot: u32,
    ) -> u32 {
        let v = if add {
            win_ld(mem, w, step) + win_ld(mem, w, field)
        } else {
            win_ld(mem, w, field) - win_ld(mem, w, step)
        };
        let frames = mem.read::<i16>(w.wrapping_add(0xc)).wrapping_sub(1);
        mem.write(w.wrapping_add(0xc), frames);
        mem.write(w.wrapping_add(field), x87::store(v));
        if frames > 0 {
            return 0;
        }
        if reload {
            let r = mem.read::<u16>(w.wrapping_add(0xe));
            mem.write(w.wrapping_add(0xc), r);
        }
        win_next(mem, w, state, slot)
    }

    /// The last handler of a close: inactive (`+0`, and `+8` when
    /// `clear_state`), the idle handler (`0x00907e50`). Returns 1.
    fn win_finish(mem: &mut impl Mem, w: u32, clear_state: bool) -> u32 {
        mem.write(w, 0u16);
        if clear_state {
            mem.write(w.wrapping_add(8), 0u16);
        }
        let idle = mem.rebase(super::WINDOW_IDLE);
        mem.write(w.wrapping_add(0x74), idle);
        1
    }

    /// The drawn rectangle from the rectangle (x0, y0, `a`, `b`).
    fn win_drawn(mem: &mut impl Mem, w: u32, a: u32, b: u32) {
        win_copy(mem, w, 0x48, 0x28);
        win_copy(mem, w, 0x4c, 0x2c);
        win_copy(mem, w, 0x50, a);
        win_copy(mem, w, 0x54, b);
    }

    /// `yiWindowGrowInit(w)` (`0x00907cc0`, kind 0): progress 0, drawn
    /// x1 = x0 (a zero-width rectangle).
    pub fn yi_window_grow_init(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(6), 0u16);
        win_drawn(mem, w, 0x28, 0x34);
        0
    }

    /// The drawn right edge of a growing window: `x0 + height * progress
    /// / 4096` (`f32`).
    fn win_grow_edge(mem: &mut impl Mem, w: u32) -> f64 {
        let p = i32::from(mem.read::<i16>(w.wrapping_add(0x10)))
            * i32::from(mem.read::<i16>(w.wrapping_add(6)));
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32 rounds
        let f = f64::from((p / 4096) as f32);
        let t = x87::store(f + win_ld(mem, w, 0x28));
        mem.write(w.wrapping_add(0x50), t);
        f64::from(f32::from_bits(t))
    }

    /// `yiWindowGrowOpen(w)` (`0x00907cf0`): progress + 0x2aa (at most
    /// 0x1000); the drawn x1 from it; open (state 2, the handler
    /// `0x00907d70`) once it reaches x1.
    pub fn yi_window_grow_open(mem: &mut impl Mem, w: u32) -> u32 {
        let p = mem.read::<i16>(w.wrapping_add(6)).wrapping_add(0x2aa);
        mem.write(w.wrapping_add(6), if p >= 0x1000 { 0x1000 } else { p });
        let t = win_grow_edge(mem, w);
        if win_ld(mem, w, 0x30) <= t {
            mem.write(w.wrapping_add(8), 2u16);
            let h = mem.rebase(super::WINDOW_OPEN_IDLE);
            mem.write(w.wrapping_add(0x74), h);
            return 1;
        }
        0
    }

    /// `yiWindowGrowClose(w)` (`0x00906fc0`): progress - 0x2aa (at least
    /// 0); closed (state 4, the handler `yiWindowFinishAll`) once the
    /// drawn x1 is back at x0.
    pub fn yi_window_grow_close(mem: &mut impl Mem, w: u32) -> u32 {
        let p = mem.read::<i16>(w.wrapping_add(6)).wrapping_add(-0x2aa);
        mem.write(w.wrapping_add(6), if p > 0 { p } else { 0 });
        let t = win_grow_edge(mem, w);
        if t <= win_ld(mem, w, 0x28) {
            mem.write(w.wrapping_add(8), 4u16);
            let h = mem.rebase(super::WINDOW_FINISH_ALL);
            mem.write(w.wrapping_add(0x74), h);
            return 1;
        }
        0
    }

    /// `yiWindowFinishAll(w)` (`0x00907040`): [`win_finish`], state 0.
    pub fn yi_window_finish_all(mem: &mut impl Mem, w: u32) -> u32 {
        win_finish(mem, w, true)
    }

    /// `yiWindowFinish(w)` (`0x009071f0`, `0x00907370`, `0x009075b0`,
    /// `0x009077e0`, `0x00907a20`: the same code): [`win_finish`].
    pub fn yi_window_finish(mem: &mut impl Mem, w: u32) -> u32 {
        win_finish(mem, w, false)
    }

    /// `yiWindowPopInit(w)` (`0x00907160`, kinds 1 and 2): no frames, the
    /// drawn rectangle the rectangle.
    pub fn yi_window_pop_init(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0u32);
        win_drawn(mem, w, 0x30, 0x34);
        0
    }

    /// `yiWindowPopOpen(w)` (`0x00907190`): open at once (state 2, slot
    /// `+0x68`).
    pub fn yi_window_pop_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_next(mem, w, 2, 0x68)
    }

    /// `yiWindowPopClose(w)` (`0x009071c0`): one frame (`+0xc`) less;
    /// closed (state 4, slot `+0x70`) when none is left.
    pub fn yi_window_pop_close(mem: &mut impl Mem, w: u32) -> u32 {
        let f = mem.read::<i16>(w.wrapping_add(0xc)).wrapping_sub(1);
        mem.write(w.wrapping_add(0xc), f);
        if f > 0 {
            return 0;
        }
        win_next(mem, w, 4, 0x70)
    }

    /// `yiWindowSlideDownInit(w)` (`0x00907450`, kind 3): the window drops
    /// from y = 416 (the screen's bottom) to its rectangle: from `(x0, 0,
    /// x1, 0)`, drawn `(x0, 0, x1, 0)`, 6 frames of `(416 - 0) / 6`, y1
    /// 416.
    pub fn yi_window_slide_down_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_HEIGHT_F32);
        mem.write(w.wrapping_add(0x34), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        for off in [0x2cu32, 0x24, 0x1c] {
            mem.write(w.wrapping_add(off), 0u32);
        }
        win_copy(mem, w, 0x18, 0x28);
        win_copy(mem, w, 0x20, 0x30);
        win_copy(mem, w, 0x50, 0x20);
        win_copy(mem, w, 0x48, 0x18);
        mem.write(w.wrapping_add(0x4c), 0u32);
        mem.write(w.wrapping_add(0x54), 0u32);
        let s = (win_f64(mem, super::WINDOW_HEIGHT) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x3c), x87::store(s));
        0
    }

    /// `yiWindowSlideDownOpen(w)` (`0x009072e0`): the drawn y1 (`+0x54`)
    /// grows by the step; then open.
    pub fn yi_window_slide_down_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x54, 0x3c, true, true, 2, 0x68)
    }

    /// `yiWindowSlideDownClose(w)` (`0x00907330`): the drawn y1 shrinks;
    /// then closed.
    pub fn yi_window_slide_down_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x54, 0x3c, false, false, 4, 0x70)
    }

    /// `yiWindowSlideUpInitHere(w)` (`0x009074b0`, kind 4): from the
    /// bottom edge (416) up to the rectangle moved so its bottom is at
    /// 416: y0 = y0 - (416 - y1), 6 frames.
    pub fn yi_window_slide_up_init_here(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        let h = win_f64(mem, super::WINDOW_HEIGHT);
        let t = x87::store(win_ld(mem, w, 0x2c) - (h - win_ld(mem, w, 0x34)));
        mem.write(w.wrapping_add(0x2c), t);
        let k = win_const(mem, super::WINDOW_HEIGHT_F32);
        mem.write(w.wrapping_add(0x24), k);
        mem.write(w.wrapping_add(0x1c), k);
        slide_up_rest(mem, w, k, f64::from(f32::from_bits(t)));
        0
    }

    /// The common tail of the kind 4/5 inits: from `(x0, 416, x1, 416)`,
    /// drawn the same, the step `(y1 - y0) / 6`.
    fn slide_up_rest(mem: &mut impl Mem, w: u32, k: u32, y0: f64) {
        win_copy(mem, w, 0x18, 0x28);
        win_copy(mem, w, 0x20, 0x30);
        win_copy(mem, w, 0x50, 0x20);
        win_copy(mem, w, 0x48, 0x18);
        mem.write(w.wrapping_add(0x4c), k);
        mem.write(w.wrapping_add(0x54), k);
        let s = (win_ld(mem, w, 0x34) - y0) / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x3c), x87::store(s));
    }

    /// `yiWindowSlideUpInit(w)` (`0x00907690`, kind 5): the rectangle
    /// `(x0, 0, x1, 416)` slides up from the bottom edge.
    pub fn yi_window_slide_up_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_HEIGHT_F32);
        mem.write(w.wrapping_add(0x34), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        mem.write(w.wrapping_add(0x2c), 0u32);
        mem.write(w.wrapping_add(0x24), k);
        mem.write(w.wrapping_add(0x1c), k);
        win_copy(mem, w, 0x18, 0x28);
        win_copy(mem, w, 0x20, 0x30);
        win_copy(mem, w, 0x50, 0x20);
        win_copy(mem, w, 0x48, 0x18);
        mem.write(w.wrapping_add(0x4c), k);
        mem.write(w.wrapping_add(0x54), k);
        let s = (win_f64(mem, super::WINDOW_HEIGHT) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x3c), x87::store(s));
        0
    }

    /// `yiWindowSlideUpOpen(w)` (`0x00907520`): the drawn y0 (`+0x4c`)
    /// rises by the step; then open.
    pub fn yi_window_slide_up_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x4c, 0x3c, false, true, 2, 0x68)
    }

    /// `yiWindowSlideUpClose(w)` (`0x00907570`): the drawn y0 falls; then
    /// closed.
    pub fn yi_window_slide_up_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x4c, 0x3c, true, false, 4, 0x70)
    }

    /// `yiWindowSlideRightInitHere(w)` (`0x009076f0`, kind 6): x1 = x1 +
    /// x0, from the left edge (0), 6 frames of `(x1 - x0) / 6`.
    pub fn yi_window_slide_right_init_here(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        let t = x87::store(win_ld(mem, w, 0x30) + win_ld(mem, w, 0x28));
        mem.write(w.wrapping_add(0x30), t);
        mem.write(w.wrapping_add(0x20), 0u32);
        mem.write(w.wrapping_add(0x18), 0u32);
        side_rest(mem, w, 0);
        let s = (f64::from(f32::from_bits(t)) - win_ld(mem, w, 0x28))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// The common middle of the kind 6..=9 inits: from y0/y1 of the
    /// rectangle, drawn `(edge, y0, edge, y1)`.
    fn side_rest(mem: &mut impl Mem, w: u32, edge: u32) {
        win_copy(mem, w, 0x1c, 0x2c);
        win_copy(mem, w, 0x24, 0x34);
        win_copy(mem, w, 0x54, 0x24);
        mem.write(w.wrapping_add(0x48), edge);
        mem.write(w.wrapping_add(0x50), edge);
        win_copy(mem, w, 0x4c, 0x1c);
    }

    /// `yiWindowSlideRightInit(w)` (`0x009078c0`, kind 7): the rectangle
    /// `(0, y0, 512, y1)` slides in from the left edge.
    pub fn yi_window_slide_right_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_WIDTH_F32);
        mem.write(w.wrapping_add(0x30), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        for off in [0x28u32, 0x20, 0x18] {
            mem.write(w.wrapping_add(off), 0u32);
        }
        side_rest(mem, w, 0);
        let s = (win_f64(mem, super::WINDOW_WIDTH) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// `yiWindowSlideRightOpen(w)` (`0x00907750`): the drawn x1 (`+0x50`)
    /// grows by the step; then open.
    pub fn yi_window_slide_right_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x50, 0x38, true, true, 2, 0x68)
    }

    /// `yiWindowSlideRightClose(w)` (`0x009077a0`): the drawn x1 shrinks;
    /// then closed.
    pub fn yi_window_slide_right_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x50, 0x38, false, false, 4, 0x70)
    }

    /// `yiWindowSlideLeftInitHere(w)` (`0x00907920`, kind 8): x0 = x0 -
    /// (512 - x1), from the right edge (512), 6 frames of `(x1 - x0) / 6`.
    pub fn yi_window_slide_left_init_here(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        let width = win_f64(mem, super::WINDOW_WIDTH);
        let t = x87::store(win_ld(mem, w, 0x28) - (width - win_ld(mem, w, 0x30)));
        mem.write(w.wrapping_add(0x28), t);
        let k = win_const(mem, super::WINDOW_WIDTH_F32);
        mem.write(w.wrapping_add(0x20), k);
        mem.write(w.wrapping_add(0x18), k);
        side_rest(mem, w, k);
        let s = (win_ld(mem, w, 0x30) - f64::from(f32::from_bits(t)))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// `yiWindowSlideLeftInit(w)` (`0x00907b00`, kind 9): the rectangle
    /// `(0, y0, 512, y1)` slides in from the right edge.
    pub fn yi_window_slide_left_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_WIDTH_F32);
        mem.write(w.wrapping_add(0x30), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        mem.write(w.wrapping_add(0x28), 0u32);
        mem.write(w.wrapping_add(0x20), k);
        mem.write(w.wrapping_add(0x18), k);
        side_rest(mem, w, k);
        let s = (win_f64(mem, super::WINDOW_WIDTH) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// `yiWindowSlideLeftOpen(w)` (`0x00907990`): the drawn x0 (`+0x48`)
    /// moves left by the step; then open.
    pub fn yi_window_slide_left_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x48, 0x38, false, true, 2, 0x68)
    }

    /// `yiWindowSlideLeftClose(w)` (`0x009079e0`): the drawn x0 moves
    /// back; then closed.
    pub fn yi_window_slide_left_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x48, 0x38, true, false, 4, 0x70)
    }

    /// `yiWindowBarInit(w)` (`0x00907b60`, kind 10): the drawn rectangle
    /// `(0, 8, 512, 28)` and colour `0x80808080`.
    pub fn yi_window_bar_init(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0x48), 0u32);
        mem.write(w.wrapping_add(0x58), 0x8080_8080u32);
        for (off, c) in [
            (0x4cu32, super::WINDOW_BAR_TOP),
            (0x50, super::WINDOW_WIDTH_F32),
            (0x54, super::WINDOW_BAR_BOTTOM),
        ] {
            let v = win_const(mem, c);
            mem.write(w.wrapping_add(off), v);
        }
        0
    }

    /// `yiWindowBarOpen(w)` (`0x00907ba0`): open at once (state 2, the
    /// handler `0x00907d70`).
    pub fn yi_window_bar_open(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(8), 2u16);
        let h = mem.rebase(super::WINDOW_OPEN_IDLE);
        mem.write(w.wrapping_add(0x74), h);
        1
    }

    /// `yiWindowBarClose(w)` (`0x00907bd0`): closed at once (state 4, the
    /// handler `yiWindowFinishAll`).
    pub fn yi_window_bar_close(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(8), 4u16);
        let h = mem.rebase(super::WINDOW_FINISH_ALL);
        mem.write(w.wrapping_add(0x74), h);
        1
    }

    /// `yiObjInit(obj)` (`0x0090ff40`): a map object (`0xb4` bytes)
    /// cleared: the scale (`+0x5c`, `+0x6c`, `+0x7c`) 1.0, the rest of
    /// `+0x50..+0x7c` 0.0, `+0x80..+0x98` and `+0xac` 0, `+0xc` the GS tag
    /// word `0x6c040014`, `+0x10` the identity (`FFXVu0UnitMatrix` into a
    /// local, then copied).
    pub fn yi_obj_init(mem: &mut impl Mem, obj: u32) {
        mem.write(obj.wrapping_add(0x82), 0u32);
        mem.write(obj.wrapping_add(0x80), 0u16);
        for off in [0x70u32, 0x60, 0x50, 0x74, 0x64, 0x54, 0x78, 0x68, 0x58] {
            mem.write(obj.wrapping_add(off), 0u32);
        }
        for off in [0x7cu32, 0x6c, 0x5c] {
            mem.write(obj.wrapping_add(off), 1.0f32.to_bits());
        }
        mem.write(obj.wrapping_add(0xac), 0u32);
        mem.write(obj.wrapping_add(0x86), 0u16);
        mem.write(obj.wrapping_add(0x8c), 0u32);
        mem.write(obj.wrapping_add(0x98), 0u32);
        mem.write(obj.wrapping_add(0x90), 0u32);
        mem.write(obj.wrapping_add(0x94), 0u32);
        for off in [0u32, 4, 8] {
            mem.write(obj.wrapping_add(off), 0u32);
        }
        mem.write(obj.wrapping_add(0xc), 0x6c04_0014u32);
        ffx_ps2::vu0::bridge::matrix_identity(mem, obj.wrapping_add(0x10));
    }

    /// `yiLoadVramImage(scene, entry, data)` (`0x0090e5d0`): an image
    /// meant for GS memory at or above `0x3c00` (entry `+4`) kept in the
    /// scene's copy from `+0x276a4` (64 bytes a page step): format 0
    /// (`+0xc`) copies `w * h / 8` quadwords (`+0x10`, `+0x14`), 0x14
    /// `w * h / 32`, 0x13 goes through `0x0090e0b0` (a swizzle, through
    /// `call`). Returns that call's `eax`, else 0.
    pub fn yi_load_vram_image<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        entry: u32,
        data: u32,
        call: &mut C,
    ) -> u32 {
        let page = mem
            .read::<u32>(entry.wrapping_add(4))
            .wrapping_sub(0x3c00)
            .wrapping_mul(0x40)
            .cast_signed()
            / 4;
        let dst = scene
            .wrapping_add(0x276a4)
            .wrapping_add(page.cast_unsigned().wrapping_mul(0x10));
        let w = mem.read::<i32>(entry.wrapping_add(0x10));
        let h = mem.read::<i32>(entry.wrapping_add(0x14));
        let n = match mem.read::<u32>(entry.wrapping_add(0xc)) {
            0 => w.wrapping_mul(h) / 8,
            0x13 => {
                let f = mem.rebase(super::YI_SWIZZLE_IMAGE);
                return call(mem, f, &[w.cast_unsigned(), h.cast_unsigned(), data, dst]);
            }
            0x14 => w.wrapping_mul(h) / 32,
            _ => 0,
        };
        let mut i = 0;
        while i < n {
            let k = i.cast_unsigned().wrapping_mul(16);
            for j in 0..4u32 {
                let v = mem.read::<u32>(data.wrapping_add(k).wrapping_add(4 * j));
                mem.write(dst.wrapping_add(k).wrapping_add(4 * j), v);
            }
            i += 1;
        }
        0
    }

    /// `yiMapFileRead(scene, file, image)` (`0x0090f900`): reads a map
    /// render file (magic `0x214365`, version `0x110`; `+0xc` the record
    /// count, decremented as read; records of a `0x40`-byte header, `+0`
    /// kind, `+4` length, then data): 0 an object ([`yi_obj_init`],
    /// [`yi_obj_load`], `graphicObjectSetTranslation`/`Rotation`), 1 a
    /// camera of the last object ([`yi_camera_load`], linked to the
    /// previous; stops when its key count `+0x48` disagrees with the
    /// length), 2 a texture (into the scene's table from `+0x9c19c`, `0x24`
    /// bytes each; uploaded with `sceGsSetDefLoadImage` into `image` and
    /// `sceGsExecLoadImage`, or kept by [`yi_load_vram_image`] at or above
    /// `0x3c00`), 3 the CLUTs ([`yi_clut_alloc_makepacket`], kinds 2..=4
    /// to `+0x67f20`), 4 the fog ([`yi_fog_load`], the fog matrix
    /// `+0x68078` from its angles), 5 and 8 packet lists, 6 animation
    /// tracks (`0x720` bytes a set from `+0x9d9d0`), 7 the matrix chunk
    /// ([`yi_load_matrix_chunk`]); other kinds are skipped without their
    /// data. `image` stands for the original's 0x60-byte stack buffer;
    /// the original leaves the current object and camera uninitialised,
    /// so files start with an object. The engine calls go through
    /// `call`. Returns 0.
    #[allow(clippy::too_many_lines)] // one case after another, as the original
    pub fn yi_map_file_read<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        file: u32,
        image: u32,
        call: &mut C,
    ) -> u32 {
        let c = |mem: &M, f: u32| mem.rebase(f);
        let sync = |mem: &mut M, call: &mut C| {
            let f = c(mem, super::SG_MFIFO_SYNC);
            call(mem, f, &[]);
            let f = c(mem, super::GS_SYNC_PATH);
            call(mem, f, &[0, 0]);
        };
        sync(mem, call);
        if mem.read::<u32>(file) != 0x0021_4365
            || mem.read::<u32>(file.wrapping_add(4)) != 0x110
            || mem.read::<u32>(file.wrapping_add(0xc)) == 0
        {
            return 0;
        }
        let (mut obj, mut cam, mut index) = (0u32, 0u32, 0u32);
        let mut rec = file.wrapping_add(0x40);
        loop {
            let mut data = rec.wrapping_add(0x40);
            match mem.read::<u32>(rec) {
                0 => {
                    index = mem.read::<u32>(scene.wrapping_add(0x68788));
                    obj = index
                        .wrapping_mul(0xb4)
                        .wrapping_add(0x6878c)
                        .wrapping_add(scene);
                    mem.write(scene.wrapping_add(0x68788), index.wrapping_add(1));
                    yi_obj_init(mem, obj);
                    yi_obj_load(mem, obj, data);
                    let n = mem.read::<u32>(scene.wrapping_add(0x68788)).wrapping_sub(1);
                    let f = c(mem, super::GRAPHIC_OBJECT_SET_TRANSLATION);
                    call(mem, f, &[n, obj.wrapping_add(0x60)]);
                    let angle = u32::from(mem.read::<u16>(obj.wrapping_add(0x86)));
                    let n = mem.read::<u32>(scene.wrapping_add(0x68788)).wrapping_sub(1);
                    let f = c(mem, super::GRAPHIC_OBJECT_SET_ROTATION);
                    call(mem, f, &[n, obj.wrapping_add(0x50), angle]);
                    data = data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)));
                    mem.write(obj.wrapping_add(0xac), 0u32);
                    cam = 0;
                }
                1 => {
                    let n = mem.read::<u32>(scene.wrapping_add(0x6e18c));
                    mem.write(scene.wrapping_add(0x6e18c), n.wrapping_add(1));
                    let k = n
                        .wrapping_mul(0x5c)
                        .wrapping_add(0x6e190)
                        .wrapping_add(scene);
                    mem.write(k.wrapping_add(0x30), 0u32);
                    mem.write(k.wrapping_add(0x36), 0u16);
                    for off in [0x48u32, 0x44, 0x54, 0x58, 0x4c, 0x50] {
                        mem.write(k.wrapping_add(off), 0u32);
                    }
                    let keys = mem.read::<u32>(obj.wrapping_add(0xac));
                    mem.write(obj.wrapping_add(0xac), keys.wrapping_add(1));
                    yi_camera_load(mem, k, data);
                    data = data.wrapping_add(0x40);
                    mem.write(k.wrapping_add(0x44), data);
                    let prev = cam;
                    cam = k;
                    if prev == 0 {
                        mem.write(obj.wrapping_add(0xa8), k);
                        mem.write(k.wrapping_add(0x50), 0u32);
                    } else {
                        mem.write(prev.wrapping_add(0x4c), k);
                        mem.write(k.wrapping_add(0x50), prev);
                    }
                    #[allow(clippy::cast_possible_truncation)] // the u16 index
                    mem.write(k.wrapping_add(0x34), index as u16);
                    mem.write(k.wrapping_add(0x54), 0u32);
                    let count = mem.read::<u32>(rec.wrapping_add(4)).wrapping_sub(0x40) >> 4;
                    data = data.wrapping_add(count << 4);
                    if mem.read::<u32>(k.wrapping_add(0x48)) != count {
                        return 0;
                    }
                }
                2 => {
                    data = data.wrapping_add(0x3c);
                    let n = mem.read::<u32>(scene.wrapping_add(0x9c198));
                    let e =
                        scene.wrapping_add(n.wrapping_mul(9).wrapping_add(0x27067).wrapping_mul(4));
                    for i in 0..6u32 {
                        let v = mem.read::<u32>(data.wrapping_sub(0x3c).wrapping_add(4 * i));
                        mem.write(e.wrapping_add(4 * i), v);
                    }
                    let v = mem.read::<u32>(data.wrapping_sub(4));
                    mem.write(e.wrapping_add(0x20), v);
                    sync(mem, call);
                    let rd = |mem: &M, off: u32| mem.read::<u32>(e.wrapping_add(off));
                    let args = if rd(mem, 0xc) == 0x14 {
                        [
                            image,
                            rd(mem, 4) & 0xffff,
                            rd(mem, 8) >> 1,
                            0,
                            0,
                            0,
                            rd(mem, 0x10) >> 1,
                            rd(mem, 0x14) >> 2,
                        ]
                    } else {
                        [
                            image,
                            rd(mem, 4) & 0xffff,
                            rd(mem, 8) & 0xffff,
                            rd(mem, 0xc) & 0xffff,
                            0,
                            0,
                            rd(mem, 0x10) & 0xffff,
                            rd(mem, 0x14) & 0xffff,
                        ]
                    };
                    let f = c(mem, super::GS_SET_DEF_LOAD_IMAGE);
                    call(mem, f, &args);
                    if rd(mem, 4) >= 0x3c00 {
                        yi_load_vram_image(mem, scene, e, data, call);
                    } else {
                        let f = c(mem, super::GS_SYNC_PATH);
                        call(mem, f, &[0, 0]);
                        let f = c(mem, super::FLUSH_CACHE);
                        call(mem, f, &[0]);
                        let f = c(mem, super::GS_EXEC_LOAD_IMAGE);
                        call(mem, f, &[image, data]);
                    }
                    data =
                        data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)).wrapping_sub(0x3c));
                    let n = mem.read::<u32>(scene.wrapping_add(0x9c198));
                    mem.write(scene.wrapping_add(0x9c198), n.wrapping_add(1));
                }
                3 => {
                    let kind = mem.read::<u32>(rec.wrapping_add(0x24));
                    if (2..=4).contains(&kind) {
                        mem.write(scene.wrapping_add(0x67f20), kind);
                        yi_clut_alloc_makepacket(mem, scene, data, 0x48);
                    }
                    data = data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)));
                }
                4 => {
                    yi_fog_load(mem, scene, data);
                    let m = scene.wrapping_add(0x68078);
                    ffx_ps2::vu0::bridge::matrix_identity(mem, m);
                    for (off, f) in [
                        (0xba400u32, super::VU0_ROT_MATRIX_X),
                        (0xba404, super::VU0_ROT_MATRIX_Y),
                    ] {
                        let deg = ffx_bridge::x87::load(mem.read::<u32>(scene.wrapping_add(off)));
                        let at = c(mem, super::DEG_180);
                        let d = mem.read::<f64>(at);
                        let at = c(mem, super::PI_F64);
                        let pi = mem.read::<f64>(at);
                        let a = x87::store(deg / d * pi);
                        let f = c(mem, f);
                        call(mem, f, &[m, m, a]);
                    }
                    data = data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)));
                }
                kind @ (5 | 8) => {
                    let list = if kind == 5 { 0x9d4c4u32 } else { 0x9d5c4 };
                    let shift = if kind == 5 { 7 } else { 5 };
                    let count = scene.wrapping_add(0x9d4c0);
                    if mem.read::<u32>(rec.wrapping_add(0x20)) == 0 {
                        let n = mem.read::<u32>(count);
                        mem.write(
                            scene.wrapping_add(list).wrapping_add(n.wrapping_mul(4)),
                            data,
                        );
                        let v = mem.read::<u32>(rec.wrapping_add(4)) >> shift;
                        let n = mem.read::<u32>(count);
                        mem.write(
                            scene.wrapping_add(0x9d6c4).wrapping_add(n.wrapping_mul(4)),
                            v,
                        );
                    } else {
                        let i = mem.read::<u32>(rec.wrapping_add(0x24));
                        mem.write(
                            scene.wrapping_add(list).wrapping_add(i.wrapping_mul(4)),
                            data,
                        );
                        let i = mem.read::<u32>(rec.wrapping_add(0x24));
                        let v = mem.read::<u32>(rec.wrapping_add(0x28));
                        mem.write(
                            scene.wrapping_add(0x9d6c4).wrapping_add(i.wrapping_mul(4)),
                            v,
                        );
                    }
                    let n = mem.read::<u32>(count);
                    mem.write(count, n.wrapping_add(1));
                    data = data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)));
                }
                6 => {
                    track_record(mem, scene, data);
                    data = data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)));
                }
                7 => {
                    yi_load_matrix_chunk(mem, scene, data);
                    data = data.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)));
                }
                _ => {}
            }
            let left = mem.read::<u32>(file.wrapping_add(0xc)).wrapping_sub(1);
            mem.write(file.wrapping_add(0xc), left);
            if left == 0 {
                return 0;
            }
            rec = data;
        }
    }

    /// The track set `data[0]` names (`0x720` bytes a set from the
    /// scene's `+0x9d9d0`), read again for every store as the original
    /// does.
    fn track_set(mem: &impl Mem, scene: u32, data: u32) -> u32 {
        scene.wrapping_add(mem.read::<u32>(data).wrapping_mul(0x720))
    }

    /// Record kind 6 of [`yi_map_file_read`]: a track (`0x1c` bytes from
    /// `+0x9d9e8` of the set, its count at `+0x9d9e4`) added to set
    /// `data[0]` (the highest set + 1 at `+0x9d9c4`), with its key table
    /// at `+(n + 0x5a12) * 0x1c`; a non-zero `data[1]` sets the set's
    /// five words at `+0x9d9d0`.
    fn track_record<M0: Mem>(mem: &mut M0, scene: u32, data: u32) {
        let k = mem.read::<u32>(data);
        if mem.read::<u32>(scene.wrapping_add(0x9d9c4)) <= k {
            mem.write(scene.wrapping_add(0x9d9c4), k.wrapping_add(1));
        }
        let n = mem.read::<u32>(track_set(mem, scene, data).wrapping_add(0x9d9e4));
        let e = n.wrapping_mul(0x1c);
        let at = |mem: &M0, off: u32| {
            track_set(mem, scene, data)
                .wrapping_add(off)
                .wrapping_add(e)
        };
        let a = at(mem, 0x9d9e8);
        mem.write(a, 0u32);
        let a = at(mem, 0x9d9ec);
        mem.write(a, 0u32);
        let v = mem.read::<u32>(data.wrapping_add(0x18));
        let a = at(mem, 0x9d9f4);
        mem.write(a, v);
        let a = at(mem, 0x9d9f0);
        mem.write(a, data.wrapping_add(0x40));
        let keys = |mem: &M0| {
            track_set(mem, scene, data).wrapping_add(n.wrapping_add(0x5a12).wrapping_mul(0x1c))
        };
        let on = mem.read::<u32>(data.wrapping_add(0x24)) != 0;
        let (step, table) = if on {
            (
                mem.read::<u16>(data.wrapping_add(0x24)).wrapping_shl(2),
                mem.read::<u32>(data.wrapping_add(0x18))
                    .wrapping_add(4)
                    .wrapping_shl(4)
                    .wrapping_add(data),
            )
        } else {
            (0, 0)
        };
        let a = at(mem, 0x9da00);
        mem.write(a, step);
        let a = keys(mem);
        mem.write(a, table);
        for (off, src) in [(0x9d9fcu32, 0x1cu32), (0x9d9fe, 0x20), (0x9da02, 0x28)] {
            let v = if on {
                mem.read::<u16>(data.wrapping_add(src))
            } else {
                0
            };
            let a = at(mem, off);
            mem.write(a, v);
        }
        let b = track_set(mem, scene, data);
        let c = mem.read::<u32>(b.wrapping_add(0x9d9e4));
        mem.write(b.wrapping_add(0x9d9e4), c.wrapping_add(1));
        if mem.read::<u32>(data.wrapping_add(4)) != 0 {
            for i in 0..5u32 {
                let v = mem.read::<u32>(data.wrapping_add(4 + 4 * i));
                let b = track_set(mem, scene, data);
                mem.write(b.wrapping_add(0x9d9d0 + 4 * i), v);
            }
        }
    }

    /// The full-screen scissor every window draw starts with:
    /// `TODrawScissorXYWH(0, 0, 512, 416)`.
    fn win_full_scissor<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(mem: &mut M, call: &mut C) {
        let f = mem.rebase(super::TO_DRAW_SCISSOR_XYWH);
        call(mem, f, &[0, 0, 0x200, 0x1a0]);
    }

    /// `_ftol2_sse` of an x87 value.
    fn win_int(mem: &impl Mem, x: f64) -> u32 {
        ffx_bridge::x87::ftol(mem, x)
    }

    /// `yiWindowDrawPlate(w)` (`0x00907390`, `0x009075d0`, `0x00907800`,
    /// `0x00907a40`: the same code, kinds 3..=9): the drawn rectangle as a
    /// plate (`0x008f9a40(x0, y0, x1 - x0, y1 - y0)`, truncated), then the
    /// scissor `TODrawScissor(x0 + 12, y0 + 4, x1 - 12, y1 - 4)`. Returns 0.
    pub fn yi_window_draw_plate<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        w: u32,
        call: &mut C,
    ) -> u32 {
        let x0 = win_ld(mem, w, 0x48);
        let y0 = win_ld(mem, w, 0x4c);
        let x1 = win_ld(mem, w, 0x50);
        let y1 = win_ld(mem, w, 0x54);
        win_full_scissor(mem, call);
        let h = win_int(mem, y1 - y0);
        let wd = win_int(mem, x1 - x0);
        let y = win_int(mem, y0);
        let x = win_int(mem, x0);
        let f = mem.rebase(super::TO_DRAW_PLATE);
        call(mem, f, &[x, y, wd, h]);
        let four = win_f64(mem, super::WINDOW_PLATE_INSET_Y);
        let twelve = win_f64(mem, super::WINDOW_PLATE_INSET_X);
        let a = win_int(mem, y1 - four);
        let b = win_int(mem, x1 - twelve);
        let c = win_int(mem, y0 + four);
        let d = win_int(mem, x0 + twelve);
        let f = mem.rebase(super::TO_DRAW_SCISSOR);
        call(mem, f, &[d, c, b, a]);
        0
    }

    /// `yiWindowDrawMenu(w)` (`0x00907060`, kind 0): a menu plate
    /// (`TODrawMenuPlateXYWHType(x0 + 4, y0 [- 1], x1 - x0 - 8, y1 - y0,
    /// 10)`, `f32`s; `y0 - 1` when `0x008ac2f0` says so), then the scissor
    /// `TODrawScissor(x0 - 5, y0, x1 - 6, y1)`. Returns 0.
    pub fn yi_window_draw_menu<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        w: u32,
        call: &mut C,
    ) -> u32 {
        win_full_scissor(mem, call);
        let f = mem.rebase(super::WINDOW_MENU_SHIFT);
        let shift = call(mem, f, &[]) != 0;
        let h = x87::store(win_ld(mem, w, 0x54) - win_ld(mem, w, 0x4c));
        let wd = x87::store(
            win_ld(mem, w, 0x50) - win_ld(mem, w, 0x48) - win_f64(mem, super::WINDOW_MENU_INSET_W),
        );
        let y = if shift {
            x87::store(win_ld(mem, w, 0x4c) - win_f64(mem, super::WINDOW_MENU_SHIFT_Y))
        } else {
            x87::copy(mem.read::<u32>(w.wrapping_add(0x4c)))
        };
        let x = x87::store(win_ld(mem, w, 0x48) + win_f64(mem, super::WINDOW_PLATE_INSET_Y));
        let f = mem.rebase(super::TO_DRAW_MENU_PLATE);
        call(mem, f, &[x, y, wd, h, 10]);
        let a = win_int(mem, win_ld(mem, w, 0x54));
        let b = win_int(
            mem,
            win_ld(mem, w, 0x50) - win_f64(mem, super::WINDOW_FRAMES),
        );
        let c = win_int(mem, win_ld(mem, w, 0x4c));
        let d = win_int(
            mem,
            win_ld(mem, w, 0x48) - win_f64(mem, super::WINDOW_MENU_INSET_X),
        );
        let f = mem.rebase(super::TO_DRAW_SCISSOR);
        call(mem, f, &[d, c, b, a]);
        0
    }

    /// `yiWindowDrawPop(w)` (`0x00907210`, kinds 1 and 2): on map 0x15b
    /// with the window at (65, 290), the scissor of the first of the eight
    /// `TO` windows (`TOGetWindowStructAdrs`) at (38, 115):
    /// `TODrawScissor(x0 - 5, y0, x1 - 0, y1)`; otherwise the full scissor
    /// packet
    /// (`TOMakePktScissor(0, 0, 512, 416)`). Returns 0.
    pub fn yi_window_draw_pop<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        w: u32,
        call: &mut C,
    ) -> u32 {
        #[allow(clippy::float_cmp)] // fucompp: exact
        let found = if super::CURRENT_MAP.get(mem) == 0x15b
            && win_ld(mem, w, 0x28)
                == f64::from(f32::from_bits(win_const(mem, super::POP_WINDOW_X)))
            && win_ld(mem, w, 0x2c)
                == f64::from(f32::from_bits(win_const(mem, super::POP_WINDOW_Y)))
        {
            let mut hit = None;
            for i in 0..8u32 {
                let f = mem.rebase(super::TO_GET_WINDOW);
                let p = call(mem, f, &[i]);
                if win_ld(mem, p, 0x28)
                    == f64::from(f32::from_bits(win_const(mem, super::POP_TARGET_X)))
                    && win_ld(mem, p, 0x2c) == win_f64(mem, super::POP_TARGET_Y)
                {
                    hit = Some(p);
                    break;
                }
            }
            hit
        } else {
            None
        };
        if let Some(p) = found {
            let a = win_int(mem, win_ld(mem, p, 0x54));
            let b = win_int(mem, win_ld(mem, p, 0x50) - win_f64(mem, super::WINDOW_ZERO));
            let c = win_int(mem, win_ld(mem, p, 0x4c));
            let d = win_int(
                mem,
                win_ld(mem, p, 0x48) - win_f64(mem, super::WINDOW_MENU_INSET_X),
            );
            let f = mem.rebase(super::TO_DRAW_SCISSOR);
            call(mem, f, &[d, c, b, a]);
        } else {
            let f = mem.rebase(super::TO_MAKE_PKT_SCISSOR);
            call(mem, f, &[0, 0, 0x200, 0x1a0]);
        }
        0
    }

    /// `yiWindowDrawBar(w)` (`0x00907c10`, kind 10): the help bar: two
    /// sprites (`0x008f5e90`) from the bar layout at `0x025d1618`..
    /// (`i16` positions, two words), its text (`TOMkpShapeXY(0x106, x +
    /// 0x50, y)`) and the scissor packet `(0, 8, 512, 28)`. The window is
    /// not read. Returns 0.
    pub fn yi_window_draw_bar<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        _w: u32,
        call: &mut C,
    ) -> u32 {
        win_full_scissor(mem, call);
        let l = mem.rebase(super::WINDOW_BAR_LAYOUT);
        let s = |mem: &M, off: u32| i32::from(mem.read::<i16>(l + off)).cast_unsigned();
        let wd = |mem: &M, off: u32| mem.read::<u32>(l + off);
        let f = mem.rebase(super::TO_DRAW_SPRITE);
        let args = [
            0x10d,
            0x10e,
            s(mem, 0x18),
            s(mem, 0x16),
            s(mem, 0x14),
            wd(mem, 0x10),
            wd(mem, 0xc),
        ];
        call(mem, f, &args);
        let args = [
            0xce,
            0xcf,
            s(mem, 4),
            s(mem, 2),
            s(mem, 0),
            0x4000_0000,
            0x4000_0000,
        ];
        call(mem, f, &args);
        let f = mem.rebase(super::TO_MKP_SHAPE_XY);
        let args = [0x106, s(mem, 0x18).wrapping_add(0x50), s(mem, 0x16)];
        call(mem, f, &args);
        let f = mem.rebase(super::TO_MAKE_PKT_SCISSOR);
        call(mem, f, &[0, 8, 0x200, 0x1c]);
        0
    }

    /// `yiArc_readmem(scene, file)` (`0x0090f8e0`): [`yi_map_file_read`]
    /// when the file is version `0x110`. Returns 0.
    pub fn yi_arc_readmem<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        file: u32,
        image: u32,
        call: &mut C,
    ) -> u32 {
        if mem.read::<u32>(file.wrapping_add(4)) == 0x110 {
            yi_map_file_read(mem, scene, file, image, call);
        }
        0
    }

    /// `yiAnimInfo_init(info)` (`0x00914310`): `g_YiAnimUseVCount` from
    /// `Sg_GetKeepFps` (through `call`), the 64 animation slots (`0x28`
    /// bytes from `+8`) and the five 64-word tables from `+0xa0c`
    /// cleared, `+4` and `+0xa08` 0.
    pub fn yi_anim_info_init<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        info: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(super::SG_GET_KEEP_FPS);
        let v = call(mem, f, &[]);
        super::ANIM_USE_VCOUNT.set(mem, v);
        mem.write(info.wrapping_add(4), 0u32);
        for i in 0..0x40u32 {
            let e = info.wrapping_add(0xc + 0x28 * i);
            for d in [-4i32, 0, 0x10, 0x14, 4, 0x20] {
                mem.write(e.wrapping_add_signed(d), 0u32);
            }
        }
        mem.write(info.wrapping_add(0xa08), 0u32);
        for i in 0..0x40u32 {
            let e = info.wrapping_add(0xb0c + 4 * i);
            for d in [-0x100i32, 0, 0x100, 0x200, 0x300] {
                mem.write(e.wrapping_add_signed(d), 0u32);
            }
        }
    }

    /// `yiAnimDetach(scene, obj, kind)` (`0x00914920`): every active
    /// animation slot (`0x28` bytes from `+0x9cac0`, 64) of `obj` (`+6`)
    /// and `kind` (`+4`) cleared and the count (`+0x9cabc`) lowered; kind
    /// 2 turns the keyframe animation off first
    /// (`graphicFFXApplicationEnableKeyframeAnim(0)`). Stops after as many
    /// active slots as the count. Returns 0.
    pub fn yi_anim_detach<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        obj: u32,
        kind: u32,
        call: &mut C,
    ) -> u32 {
        let mut seen = 0u32;
        for i in 0..0x40u32 {
            let e = scene.wrapping_add(0x9cac0 + 0x28 * i);
            if mem.read::<u8>(e) & 1 == 0 {
                continue;
            }
            if u32::from(mem.read::<u16>(e.wrapping_add(6))) == obj
                && u32::from(mem.read::<u16>(e.wrapping_add(4))) == kind
            {
                if mem.read::<u16>(e.wrapping_add(4)) == 2 {
                    let f = mem.rebase(super::ENABLE_KEYFRAME_ANIM);
                    call(mem, f, &[0]);
                }
                mem.write(e, 0u16);
                mem.write(e.wrapping_add(0x14), 0u32);
                mem.write(e.wrapping_add(0x18), 0u32);
                mem.write(e.wrapping_add(8), 0u32);
                mem.write(e.wrapping_add(6), 0u16);
                let n = mem.read::<u32>(scene.wrapping_add(0x9cabc));
                mem.write(scene.wrapping_add(0x9cabc), n.wrapping_sub(1));
            }
            seen += 1;
            if seen >= mem.read::<u32>(scene.wrapping_add(0x9cabc)) {
                return 0;
            }
        }
        0
    }

    /// `yiAnimDetachAll(scene, obj)` (`0x009149a0`, name ours;
    /// `yiDisposeAnimDObj` with a negative kind): [`yi_anim_detach`]
    /// without the kind check, every active slot of `obj` cleared. Returns
    /// 0.
    pub fn yi_anim_detach_all<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        obj: u32,
        call: &mut C,
    ) -> u32 {
        let mut seen = 0u32;
        for i in 0..0x40u32 {
            let e = scene.wrapping_add(0x9cac0 + 0x28 * i);
            if mem.read::<u8>(e) & 1 == 0 {
                continue;
            }
            if u32::from(mem.read::<u16>(e.wrapping_add(6))) == obj {
                if mem.read::<u16>(e.wrapping_add(4)) == 2 {
                    let f = mem.rebase(super::ENABLE_KEYFRAME_ANIM);
                    call(mem, f, &[0]);
                }
                mem.write(e, 0u16);
                mem.write(e.wrapping_add(0x14), 0u32);
                mem.write(e.wrapping_add(0x18), 0u32);
                mem.write(e.wrapping_add(8), 0u32);
                mem.write(e.wrapping_add(6), 0u16);
                let n = mem.read::<u32>(scene.wrapping_add(0x9cabc));
                mem.write(scene.wrapping_add(0x9cabc), n.wrapping_sub(1));
            }
            seen += 1;
            if seen >= mem.read::<u32>(scene.wrapping_add(0x9cabc)) {
                return 0;
            }
        }
        0
    }

    /// `yiDisposeAnimDObj(obj, kind)` (`0x009084b0`): on the renderer
    /// object, [`yi_anim_detach_all`] for a negative kind, else
    /// [`yi_anim_detach`]. Returns 0.
    pub fn yi_dispose_anim_dobj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        kind: u32,
        call: &mut C,
    ) -> u32 {
        let scene = mem.rebase(0x0187_6e00);
        if kind.cast_signed() < 0 {
            yi_anim_detach_all(mem, scene, obj, call)
        } else {
            yi_anim_detach(mem, scene, obj, kind, call)
        }
    }

    /// `yiSetMIMeParamDObj(obj, p)` (`0x0090a4d0`): for a display object
    /// below the count, the float at `p` (through the x87 stack) into the
    /// object's `+0x18df624` slot (`0xb4` bytes each) and to
    /// `graphicSetMeshVertexColorMorphValue(obj, v)`. Returns 0.
    pub fn yi_set_mime_param_dobj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        p: u32,
        call: &mut C,
    ) -> u32 {
        if obj >= DOBJ_COUNT.get(mem) {
            return 0;
        }
        let v = ffx_bridge::x87::round_trip(mem.read::<f32>(p)).to_bits();
        let at = mem
            .rebase(super::MIME_PARAM)
            .wrapping_add(obj.wrapping_mul(0xb4));
        mem.write(at, v);
        let f = mem.rebase(super::SET_MORPH_VALUE);
        call(mem, f, &[obj, v]);
        0
    }

    /// `yiGetColorOffsetInt(out)` (`0x00908660`, an effect import; name
    /// ours): the four colour offsets plus 128.0, truncated
    /// (`_ftol2_sse`) into four ints. Returns the last.
    pub fn yi_get_color_offset_int(mem: &mut impl Mem, out: u32) -> u32 {
        let c: f64 = mem.read(mem.rebase(super::COLOR_OFFSET_BIAS));
        let off = COLOR_OFFSET.get(mem);
        let mut last = 0;
        for i in 0..4u32 {
            last = ffx_bridge::x87::ftol(
                mem,
                ffx_bridge::x87::add(ffx_bridge::x87::load(off[i as usize]), c),
            );
            mem.write(out.wrapping_add(4 * i), last);
        }
        last
    }

    /// `loadPhyreFieldMap()` (`0x00908000`): with a map read pending
    /// ([`PENDING_MAP`] and the name at [`PENDING_MAP_NAME`]), both map
    /// kinds off, `"RAM Inuse : %d u:%d\n"` (0, 0) to `stdout`, then each
    /// section the map has (`yiMapGetSection` 0: 2D, 1: 3D) switched on,
    /// counted ([`MAP2D_LOADS`]) and loaded (`graphicFieldMapLoad(name,
    /// kind)`, [`MAP_KIND`]); the 2D one also sets render status bit 14
    /// and [`MAP_SECTION`]. The request is cleared,
    /// `graphicFieldMapInitialize` and `graphicVFXLoadLevelData` follow;
    /// returns the last one's `eax`. Without a request it clears
    /// [`BTL_MAP_READ`] and leaves `eax` alone (`None`).
    pub fn load_phyre_field_map<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> Option<u32> {
        let name = mem.rebase(super::PENDING_MAP_NAME);
        if super::PENDING_MAP.get(mem) == 0 || mem.read::<u8>(name) == 0 {
            super::BTL_MAP_READ.set(mem, 0);
            return None;
        }
        let has2d = mem.rebase(super::GRAPHIC_SET_HAS_2D_MAP);
        let has3d = mem.rebase(super::GRAPHIC_SET_HAS_3D_MAP);
        call(mem, has2d, &[0]);
        call(mem, has3d, &[0]);
        let iob = mem.read::<u32>(mem.rebase(super::CRT_IOB_FUNC));
        let file = call(mem, iob, &[]);
        let fprintf = mem.read::<u32>(mem.rebase(super::CRT_FPRINTF));
        let text = mem.rebase(super::RAM_INUSE);
        call(mem, fprintf, &[file.wrapping_add(0x20), text, 0, 0]);
        let load = mem.rebase(super::GRAPHIC_FIELD_MAP_LOAD);
        let map = super::PENDING_MAP.get(mem);
        let s2 = yi_map_get_section(mem, map, 0);
        if s2 != 0 {
            call(mem, has2d, &[1]);
            super::MAP2D_LOADS.set(mem, super::MAP2D_LOADS.get(mem).wrapping_add(1));
            call(mem, load, &[name, 0]);
            RENDER_STATUS.set(mem, RENDER_STATUS.get(mem) | 0x4000);
            super::MAP_KIND.set(mem, 0);
            super::MAP_SECTION.set(mem, s2);
        }
        let map = super::PENDING_MAP.get(mem);
        if yi_map_get_section(mem, map, 1) != 0 {
            call(mem, has3d, &[1]);
            super::MAP2D_LOADS.set(mem, super::MAP2D_LOADS.get(mem).wrapping_add(1));
            call(mem, load, &[name, 1]);
            super::MAP_KIND.set(mem, 1);
        }
        super::PENDING_MAP.set(mem, 0);
        mem.write(name, 0u8);
        let f = mem.rebase(super::GRAPHIC_FIELD_MAP_INITIALIZE);
        call(mem, f, &[]);
        let f = mem.rebase(super::GRAPHIC_VFX_LOAD_LEVEL_DATA);
        Some(call(mem, f, &[]))
    }

    /// `yiKeyFrame_attachDObj(scene, obj, anim, arg)` (`0x009144a0`): puts
    /// animation `anim` on display object `obj` in the first free slot:
    /// with a keyframe record (`+0x9d4c4` list) its kind (`+8`; kind 2
    /// turns keyframe animation on) and the record at `+0x24`, else (a
    /// `+0x9d5c4` packet list entry) kind 5; the start count (`yiGetFCount`,
    /// or `yiGetVCount` with `g_YiAnimUseVCount`) at `+0x14`/`+0x18`. Any
    /// earlier slot of that object and kind goes first ([`yi_anim_detach`]).
    /// Returns 0.
    pub fn yi_key_frame_attach_dobj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        obj: u32,
        anim: u32,
        arg: u16,
        call: &mut C,
    ) -> u32 {
        let now = if super::ANIM_USE_VCOUNT.get(mem) == 0 {
            yi_get_fcount(mem)
        } else {
            yi_get_vcount(mem)
        };
        let slot = scene.wrapping_add(anim.wrapping_add(0x27531).wrapping_mul(4));
        let packets = mem.read::<u32>(slot) == 0;
        if packets
            && mem.read::<u32>(
                scene
                    .wrapping_add(0x9d5c4)
                    .wrapping_add(anim.wrapping_mul(4)),
            ) == 0
        {
            return 0;
        }
        if obj >= mem.read::<u32>(scene.wrapping_add(0x68788)) {
            return 0;
        }
        let Some(i) =
            (0..0x40u32).find(|i| mem.read::<u8>(scene.wrapping_add(0x9cac0 + 0x28 * i)) & 1 == 0)
        else {
            return 0;
        };
        let (kind, record) = if packets {
            yi_anim_detach(mem, scene, obj, 5, call);
            (5u16, None)
        } else {
            let k = mem.read::<u32>(mem.read::<u32>(slot).wrapping_add(8));
            yi_anim_detach(mem, scene, obj, k, call);
            if mem.read::<u32>(mem.read::<u32>(slot).wrapping_add(8)) == 2 {
                let f = mem.rebase(super::ENABLE_KEYFRAME_ANIM);
                call(mem, f, &[1]);
            }
            (0, Some(()))
        };
        let e = scene.wrapping_add(0x28 * i);
        mem.write(e.wrapping_add(0x9cac0), 1u16);
        mem.write(e.wrapping_add(0x9cac2), arg);
        let kind = if record.is_some() {
            mem.read::<u16>(mem.read::<u32>(slot).wrapping_add(8))
        } else {
            kind
        };
        mem.write(e.wrapping_add(0x9cac4), kind);
        #[allow(clippy::cast_possible_truncation)] // the u16 fields
        {
            mem.write(e.wrapping_add(0x9cac8), anim as u16);
            mem.write(e.wrapping_add(0x9caca), 0u16);
            mem.write(e.wrapping_add(0x9cac6), obj as u16);
        }
        mem.write(e.wrapping_add(0x9cad4), now);
        mem.write(e.wrapping_add(0x9cad8), now);
        let r = if record.is_some() {
            mem.read::<u32>(slot)
        } else {
            0
        };
        mem.write(e.wrapping_add(0x9cae4), r);
        let n = mem.read::<u32>(scene.wrapping_add(0x9cabc));
        mem.write(scene.wrapping_add(0x9cabc), n.wrapping_add(1));
        0
    }

    /// `yiAnimAttachDefault(scene)` (`0x00914250`): each object's default
    /// animations (`u16` ids after the count at `+0x8c` of each
    /// `0xb4`-byte object) attached ([`yi_key_frame_attach_dobj`]), then
    /// each track set's characters (`+0x9d9d0`, `0x720` bytes a set)
    /// ([`yi_key_frame_attach_chr`]).
    pub fn yi_anim_attach_all<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        call: &mut C,
    ) {
        let mut i = 0u32;
        while i < mem.read::<u32>(scene.wrapping_add(0x68788)) {
            let e = scene.wrapping_add(0x68818 + 0xb4 * i);
            let mut j = 0u32;
            while j < mem.read::<u32>(e) {
                let a = u32::from(mem.read::<u16>(e.wrapping_add(4 + 2 * j)));
                yi_key_frame_attach_dobj(mem, scene, i, a, 0, call);
                j += 1;
            }
            i += 1;
        }
        let mut k = 0u32;
        while k < mem.read::<u32>(scene.wrapping_add(0x9d9c4)) {
            let set = scene.wrapping_add(0x9d9d0 + 0x720 * k);
            let mut j = 0u32;
            while j < mem.read::<u32>(set) {
                let kf = mem.read::<u32>(set.wrapping_add(4 + 4 * j));
                yi_key_frame_attach_chr(mem, scene, k, kf, 0);
                j += 1;
            }
            k += 1;
        }
    }

    /// `yiFree(p)` (`0x0090bbc0`): a jump to `user_free`.
    pub fn yi_free<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(crate::model::USER_FREE);
        call(mem, f, &[p])
    }

    /// `yiRender_resetobj(r)` (`0x0090cd90`): the render state reset for a
    /// new map: the packet block `+0x9caa8` freed ([`yi_free`]), the
    /// object and camera counts 0, flags `+0x67f08` `0x10000`, colour scales
    /// 1.0 and shifts 0.0, the background colour 0, the matrix `+0x68038`
    /// the identity, then the VU1 (`yiRender_setupparamVU1`, through
    /// `call`) and GS set-ups, [`yi_anim_info_init`], [`yi_tex_swap_init`],
    /// `+0x67f30` -1.
    pub fn yi_render_resetobj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        r: u32,
        call: &mut C,
    ) {
        let p = mem.read::<u32>(r.wrapping_add(0x9caa8));
        if p != 0 {
            yi_free(mem, p, call);
            mem.write(r.wrapping_add(0x9caa8), 0u32);
        }
        let v = mem.read::<u32>(r.wrapping_add(0xba3cc));
        mem.write(r.wrapping_add(0xba3d0), v);
        let v = mem.read::<u32>(r.wrapping_add(0xba3dc));
        mem.write(r.wrapping_add(0xba3e0), v);
        mem.write(r.wrapping_add(0x68788), 0u32);
        mem.write(r.wrapping_add(0x6e18c), 0u32);
        mem.write(r.wrapping_add(0x67f0c), 0u32);
        mem.write(r.wrapping_add(0x67f08), 0x1_0000u32);
        for off in [0x68204u32, 0x68200, 0x681fc, 0x681f8] {
            mem.write(r.wrapping_add(off), 1.0f32.to_bits());
        }
        for off in [
            0x68378u32, 0x6837c, 0x68380, 0x68228, 0x68224, 0x68220, 0x6821c,
        ] {
            mem.write(r.wrapping_add(off), 0u32);
        }
        for off in [0x68218u32, 0x68214, 0x68210, 0x6820c] {
            mem.write(r.wrapping_add(off), 1.0f32.to_bits());
        }
        ffx_ps2::vu0::bridge::matrix_identity(mem, r.wrapping_add(0x68038));
        let f = mem.rebase(super::RENDER_SETUP_VU1);
        call(mem, f, &[r]);
        yi_render_setupparam_gs(mem, r);
        yi_anim_info_init(mem, r.wrapping_add(0x9cab8), call);
        yi_tex_swap_init(mem, r);
        mem.write(r.wrapping_add(0x67f30), u32::MAX);
    }

    /// The three-call sync step `yiResetObj` repeats: `TkRestartSyncH(5)`,
    /// `TkGetSyncH(5)` (unless `restart_only`), `0x0088c820(5)`.
    fn sync_step<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        get: bool,
    ) {
        let f = mem.rebase(super::TK_RESTART_SYNC_H);
        call(mem, f, &[5]);
        if get {
            let f = mem.rebase(super::TK_GET_SYNC_H);
            call(mem, f, &[5]);
        }
        let f = mem.rebase(super::TK_SYNC_H_WAIT);
        call(mem, f, &[5]);
    }

    /// `yiResetObj()` (`0x00909ea0`): the field map torn down for the next
    /// one: GS path sync, `TkClearAlarm(0)`, `Yn_MapSystemFini` and a new
    /// background system (`Yn_RcbgInit` → `rcbgInit`), the map section
    /// pointer cleared, the `ppp` data (`pppFpFree`) and the map read
    /// buffer (`0x019138a4`, [`yi_free`]) released, [`yi_render_resetobj`]
    /// of `g_YiRender`, the camera ids -1, the cluster cache cleared once a
    /// 2D map was loaded, and the graphics map and VFX level unloaded. The
    /// engine calls go through `call`.
    pub fn yi_reset_obj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(mem: &mut M, call: &mut C) {
        let c = |mem: &M, f: u32| mem.rebase(f);
        let f = c(mem, super::GS_SYNC_PATH);
        call(mem, f, &[0, 0]);
        sync_step(mem, call, false);
        let f = c(mem, super::TK_CLEAR_ALARM);
        call(mem, f, &[0]);
        crate::guide::bridge::yn_map_system_fini(mem, call);
        crate::guide::bridge::rcbg_init(mem, call);
        let f = c(mem, super::TK_CLEAR_ALARM);
        call(mem, f, &[0]);
        sync_step(mem, call, true);
        sync_step(mem, call, true);
        super::MAP_SECTION.set(mem, 0);
        if super::state::MAP_READ.get(mem) != 0 {
            let f = c(mem, super::PPP_FP_FREE);
            call(mem, f, &[]);
            super::state::MAP_READ.set(mem, 0);
        }
        let b = super::MAP_READ_BUFFER.get(mem);
        if b != 0 {
            yi_free(mem, b, call);
            super::MAP_READ_BUFFER.set(mem, 0);
        }
        sync_step(mem, call, true);
        let r = super::state::RENDER.ptr(mem).addr;
        yi_render_resetobj(mem, r, call);
        sync_step(mem, call, true);
        let loaded = super::MAP2D_LOADS.get(mem).cast_signed();
        super::CAMERA_IDS.set(mem, [u32::MAX; 2]);
        if loaded > 0 {
            for f in [super::GRAPHIC_CLUSTER_DELAY_RELEASE, super::CACHE_CLEAR] {
                let f = c(mem, f);
                call(mem, f, &[]);
            }
        }
        super::MAP2D_LOADS.set(mem, 0);
        for f in [
            super::GRAPHIC_FIELD_MAP_SHUTDOWN,
            super::GRAPHIC_FIELD_MAP_UNLOAD,
            super::GRAPHIC_VFX_UNLOAD,
        ] {
            let f = c(mem, f);
            call(mem, f, &[]);
        }
    }

    /// `yiObjReadFinish(read)` (`0x0090a9e0`): with a `MAP1` file
    /// (`read +0x10`): its render section (`+0x14`) read
    /// ([`yi_arc_readmem`] into `g_YiRender`), section 2 handed to
    /// `0x0072aa40` (through `call`) and the file freed; then render
    /// status bits 6 and 7 cleared and `read +0x18` to `0x018ded34`.
    pub fn yi_obj_read_finish<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        read: u32,
        image: u32,
        call: &mut C,
    ) {
        let file = mem.read::<u32>(read.wrapping_add(0x10));
        let mut tag = [0u8; 4];
        mem.read_bytes(file, &mut tag);
        if &tag == b"MAP1" {
            let s = mem.read::<u32>(file.wrapping_add(0x14));
            if s != 0 && s.wrapping_add(file) != 0 {
                let r = super::state::RENDER.ptr(mem).addr;
                yi_arc_readmem(mem, r, s.wrapping_add(file), image, call);
            }
        }
        let sec = yi_map_get_section(mem, file, 2);
        if sec != 0 {
            let f = mem.rebase(super::MAP_SECTION2_HANDLER);
            call(mem, f, &[sec]);
        }
        yi_free(mem, file, call);
        let st = super::RENDER_STATUS.get(mem);
        super::RENDER_STATUS.set(mem, st & 0xffff_ff3f);
        let v = mem.read::<u32>(read.wrapping_add(0x18));
        super::READ_TAIL.set(mem, v);
    }

    /// The section of a `MAP1` file at the offset in `+off` (as a
    /// pointer), or `None` for another tag or an empty offset.
    fn map1_section(mem: &impl Mem, file: u32, off: u32) -> Option<u32> {
        let mut tag = [0u8; 4];
        mem.read_bytes(file, &mut tag);
        if &tag != b"MAP1" {
            return None;
        }
        let o = mem.read::<u32>(file.wrapping_add(off));
        (o != 0 && o.wrapping_add(file) != 0).then(|| o.wrapping_add(file))
    }

    /// `yiObjReadCallback(read)` (`0x009097c0`): a field map file
    /// (`read +0x10`, `MAP1`) taken in, a sync step between each part: the
    /// background (`+0x10`: [`crate::guide::bridge::yn_pre_bg_set_data`],
    /// render status bit 14), the render data (`+0x14`:
    /// [`yi_arc_readmem`]), the alarm of section `read +0x14` + 2
    /// (`TkClearAlarm`), the `ppp` data (`+0x38`: `g_YiMapRead`,
    /// `pppSetFpPdt`), two more background parts (`+0x40`, `+0x3c`: jumps
    /// to `Yn_PreBgSetData`); then render status bits 0 and 4, the draw
    /// mask (-9, -1 with the battle map circle), the default animations
    /// ([`yi_anim_attach_all`]), `read +0x18` to `0x018ded30` and status
    /// bits 5 and 7 cleared. `work` and `image` stand for the stack
    /// buffers of the callees; the engine calls go through `call`.
    pub fn yi_obj_read_callback<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        read: u32,
        work: u32,
        image: u32,
        call: &mut C,
    ) {
        let c = |mem: &M, f: u32| mem.rebase(f);
        let f = c(mem, super::TK_RESTART_SYNC_H);
        call(mem, f, &[5]);
        let ext = mem.read::<u32>(read.wrapping_add(0x14));
        let file = mem.read::<u32>(read.wrapping_add(0x10));
        super::MAP_DATA.set(mem, file);
        sync_step(mem, call, false);
        if let Some(p) = map1_section(mem, file, 0x10) {
            super::MAP_SECTION.set(mem, p);
            crate::guide::bridge::yn_pre_bg_set_data(mem, p, work, call);
            let s = super::RENDER_STATUS.get(mem);
            super::RENDER_STATUS.set(mem, s | 0x4000);
        }
        sync_step(mem, call, true);
        if let Some(p) = map1_section(mem, file, 0x14) {
            let r = super::state::RENDER.ptr(mem).addr;
            yi_arc_readmem(mem, r, p, image, call);
        }
        sync_step(mem, call, true);
        let sec = yi_map_get_section(mem, file, ext.wrapping_add(2));
        if sec != 0 {
            let f = c(mem, super::TK_CLEAR_ALARM);
            call(mem, f, &[sec]);
        }
        sync_step(mem, call, true);
        if let Some(p) = map1_section(mem, file, 0x38) {
            super::state::MAP_READ.set(mem, p);
            let f = c(mem, super::PPP_SET_FP_PDT);
            call(mem, f, &[p]);
        }
        sync_step(mem, call, true);
        // `0x00919b90` and `0x00919d30` are jumps to `Yn_PreBgSetData`
        for off in [0x40u32, 0x3c] {
            if let Some(p) = map1_section(mem, file, off) {
                crate::guide::bridge::yn_pre_bg_set_data(mem, p, work, call);
            }
            sync_step(mem, call, true);
        }
        let s = super::RENDER_STATUS.get(mem);
        super::RENDER_STATUS.set(mem, s | 0x11);
        let f = c(mem, super::MAP_CIRCLE_ON);
        let on = call(mem, f, &[]) != 0;
        super::DRAW_MASK.set(
            mem,
            if on {
                0u32.wrapping_sub(1)
            } else {
                0u32.wrapping_sub(9)
            },
        );
        let r = super::state::RENDER.ptr(mem).addr;
        yi_anim_attach_all(mem, r, call);
        let v = mem.read::<u32>(read.wrapping_add(0x18));
        let s = super::RENDER_STATUS.get(mem);
        super::RENDER_STATUS.set(mem, s & 0xffff_ff5f);
        super::READ_DONE.set(mem, v);
        sync_step(mem, call, true);
    }

    /// `yiSwizzleColumns(src, dst)` (`0x0090e000`): a 16x16 block of 8-bit
    /// pixels reordered for the GS: four column groups of 64 bytes, each
    /// byte from the group's position in the table at `0x00c5da80` (even and
    /// odd groups use its two halves). Returns 0.
    pub fn yi_swizzle_columns(mem: &mut impl Mem, src: u32, dst: u32) -> u32 {
        let table = mem.rebase(super::SWIZZLE_COLUMNS);
        for c in 0..4u32 {
            let t = table + (c & 1) * 0x40;
            for i in 0..0x40u32 {
                let k = u32::from(mem.read::<u8>(t + i));
                let v = mem.read::<u8>(src.wrapping_add(c * 0x40).wrapping_add(k));
                mem.write(dst.wrapping_add(c * 0x40 + i), v);
            }
        }
        0
    }

    /// `yiSwizzlePage(w, h, src, dst)` (`0x0090e2e0`): an 8-bit image
    /// (lines of 0x80 bytes) to GS block order, 16x16 blocks: the block
    /// numbers of `0x00c5db40` (`w / 16` per block row) placed through the
    /// 4x8 block table of `0x00c5db60` (column, row: two `yiMalloc` maps of
    /// 0x800 bytes), each block's 16 lines gathered, swizzled
    /// ([`yi_swizzle_columns`]) and written as eight 0x20-byte rows 0x100
    /// apart. `work` is the original's two 0x100-byte stack buffers.
    pub fn yi_swizzle_page<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        w: i32,
        h: i32,
        mut src: u32,
        dst: u32,
        work: u32,
        call: &mut C,
    ) {
        let col = crate::model::bridge::yi_malloc(mem, call, 0x800);
        let row = crate::model::bridge::yi_malloc(mem, call, 0x800);
        let blocks = mem.rebase(super::SWIZZLE_BLOCKS);
        for r in 0..4u32 {
            for k in 0..8u32 {
                let e = u32::from(mem.read::<u8>(blocks + r * 8 + k));
                mem.write(col.wrapping_add(e * 4), k);
                mem.write(row.wrapping_add(e * 4), r);
            }
        }
        let rows = h / 16;
        let cols = w / 16;
        let (lines, swizzled) = (work, work + 0x100);
        for i in 0..0x200u32 {
            mem.write(work + i, 0u8);
        }
        let mut order = mem.rebase(super::SWIZZLE_ORDER);
        for _ in 0..rows.max(0) {
            let mut s = src;
            for x in 0..cols.max(0).cast_unsigned() {
                let e = u32::from(mem.read::<u8>(order.wrapping_add(x)));
                for k in 0..16u32 {
                    for j in 0..4u32 {
                        let v = mem.read::<u32>(s.wrapping_add(k * 0x80 + 4 * j));
                        mem.write(lines + k * 0x10 + 4 * j, v);
                    }
                }
                yi_swizzle_columns(mem, lines, swizzled);
                let r = mem.read::<u32>(row.wrapping_add(e * 4));
                let c = mem.read::<u32>(col.wrapping_add(e * 4));
                let out = r
                    .wrapping_shl(6)
                    .wrapping_add(c)
                    .wrapping_shl(5)
                    .wrapping_add(dst);
                for k in 0..8u32 {
                    for j in 0..8u32 {
                        let v = mem.read::<u32>(swizzled + k * 0x20 + 4 * j);
                        mem.write(out.wrapping_add(k * 0x100 + 4 * j), v);
                    }
                }
                s = s.wrapping_add(0x10);
            }
            src = src.wrapping_add(0x800);
            order = order.wrapping_add(cols.cast_unsigned());
        }
        yi_free(mem, col, call);
        yi_free(mem, row, call);
    }

    /// `yiSwizzleImage(w, h, src, dst)` (`0x0090e0b0`): a format-0x13 image
    /// of `w` x `h` bytes (each a power of two up to 0x400, else "Error :
    /// width is not 2^n" to `stderr` through `__iob_func`/`fprintf` and -1)
    /// swizzled 128x64 tile by tile: each tile's lines into a 0x20000-byte
    /// buffer (lines of 0x80), [`yi_swizzle_page`] into a 0x10000-byte one
    /// (both `yiMalloc`, their first 0x2000 bytes cleared), and its rows (of
    /// `2 w` bytes, at most 0x100; `h / 2` of them, at most 0x20) out to
    /// `dst`. Returns 0. `work` as for [`yi_swizzle_page`].
    pub fn yi_swizzle_image<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        w: i32,
        h: i32,
        src: u32,
        dst: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let pow2 = |n: i32| (0..11).any(|c| n == 0x400 >> c);
        if !pow2(w) || !pow2(h) {
            let iob = mem.read::<u32>(mem.rebase(super::CRT_IOB_FUNC));
            let file = call(mem, iob, &[]);
            let fprintf = mem.read::<u32>(mem.rebase(super::CRT_FPRINTF));
            let text = mem.rebase(super::SWIZZLE_ERROR);
            call(mem, fprintf, &[file.wrapping_add(0x40), text]);
            return u32::MAX;
        }
        let lines = crate::model::bridge::yi_malloc(mem, call, 0x20000);
        let page = crate::model::bridge::yi_malloc(mem, call, 0x10000);
        for i in 0..0x2000u32 {
            mem.write(lines.wrapping_add(i), 0u8);
        }
        for i in 0..0x2000u32 {
            mem.write(page.wrapping_add(i), 0u8);
        }
        let nx = (w - 1) / 128 + 1;
        let ny = (h - 1) / 64 + 1;
        let (w, pitch) = if nx == 1 { (w, w * 2) } else { (0x80, 0x100) };
        let (h, half) = if ny == 1 { (h, h / 2) } else { (0x40, 0x20) };
        let copy = |mem: &mut M, to: u32, from: u32, n: i32| {
            for i in 0..n.max(0).cast_unsigned() {
                let v = mem.read::<u8>(from.wrapping_add(i));
                mem.write(to.wrapping_add(i), v);
            }
        };
        for y in 0..ny {
            let mut out = dst.wrapping_add((half * pitch * nx * y).cast_unsigned());
            let mut inp = src.wrapping_add((w * nx * y).cast_unsigned().wrapping_shl(6));
            for _ in 0..nx {
                for k in 0..h.max(0) {
                    let from = inp.wrapping_add((k * w * nx).cast_unsigned());
                    copy(mem, lines + k.cast_unsigned() * 0x80, from, w);
                }
                yi_swizzle_page(mem, 0x80, 0x40, lines, page, work, call);
                for k in 0..half.max(0) {
                    let to = out.wrapping_add((k * pitch * nx).cast_unsigned());
                    copy(mem, to, page + k.cast_unsigned() * 0x100, pitch);
                }
                inp = inp.wrapping_add(w.cast_unsigned());
                out = out.wrapping_add(pitch.cast_unsigned());
            }
        }
        yi_free(mem, lines, call);
        yi_free(mem, page, call);
        0
    }

    /// The start of `yiMapReadMaster` and `yiMapSetup`: the renderer's
    /// objects and textures reset (with `sceGsSyncPath` around the first in
    /// the former), the alarm cleared.
    fn map_read_reset<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        sync_first: bool,
        call: &mut C,
    ) {
        let sync = mem.rebase(super::GS_SYNC_PATH);
        if sync_first {
            call(mem, sync, &[0, 0]);
        }
        yi_reset_obj(mem, call);
        call(mem, sync, &[0, 0]);
        let r = super::state::RENDER.ptr(mem).addr;
        yi_render_resettex(mem, r);
        super::MAP_IDS.set(mem, [u32::MAX; 2]);
        let f = mem.rebase(super::TK_CLEAR_ALARM);
        call(mem, f, &[0]);
    }

    /// `yiMapReadMaster(map, file, cd_map, cd_file, alarm)` (`0x009093b0`):
    /// reads a field map. After the reset ([`map_read_reset`]) the map ids
    /// (`0x01934edc`) are `map`, `file` unless both are -1. With a CD file
    /// (`cd_map`, `cd_file` not both -1): no 2D or 3D map, "RAM Inuse"
    /// to `stdout`, the file read through `GetCD`'s table into a `yiMalloc64`
    /// buffer ([`MAP_DATA`]; -1 when it is empty or the allocation fails),
    /// then its MAP1 sections: 0 the 2D map (`graphicFieldMapLoad(name,
    /// 0)`, [`super::MAP_SECTION`], `Yn_PreBgSetData`, status bit 0x4000), 1
    /// the 3D map (`graphicFieldMapLoad(name, 1)`, `yiArc_readmem`),
    /// `graphicFieldMapInitialize`, `graphicVFXLoadLevelData`, `alarm + 2`
    /// the alarms (`TkClearAlarm`), 10 the effect data (`pppSetFpPdt`), 12
    /// and 11 more `Yn_PreBgSetData`; the CD ids kept at `0x01934ed4`. Then
    /// status `| 0x11`, the draw mask (-1 with the battle map circle, else
    /// -9) and the default animations. Returns 0. `image` and `work` as
    /// for `yiArc_readmem` and `Yn_PreBgSetData`; `local` is 0x1c bytes for
    /// the CD read.
    #[allow(clippy::too_many_arguments)]
    pub fn yi_map_read_master<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        map: u32,
        file: u32,
        cd_map: u32,
        cd_file: u32,
        alarm: u32,
        image: u32,
        work: u32,
        local: u32,
        call: &mut C,
    ) -> u32 {
        map_read_reset(mem, true, call);
        super::CAMERA_IDS.set(mem, [u32::MAX; 2]);
        super::MAP_IDS.set(mem, [u32::MAX; 2]);
        if map != u32::MAX || file != u32::MAX {
            super::MAP_IDS.set(mem, [map, file]);
        }
        let r = super::state::RENDER.ptr(mem).addr;
        if cd_map != u32::MAX || cd_file != u32::MAX {
            let c = |mem: &M, f: u32| mem.rebase(f);
            let f = c(mem, super::GRAPHIC_SET_HAS_2D_MAP);
            call(mem, f, &[0]);
            let f = c(mem, super::GRAPHIC_SET_HAS_3D_MAP);
            call(mem, f, &[0]);
            let iob = mem.read::<u32>(c(mem, super::CRT_IOB_FUNC));
            let out = call(mem, iob, &[]);
            let fprintf = mem.read::<u32>(c(mem, super::CRT_FPRINTF));
            let text = c(mem, super::RAM_INUSE_TEXT);
            call(mem, fprintf, &[out.wrapping_add(0x20), text, 0, 0]);
            let get_cd = c(mem, super::GET_CD);
            let cd = call(mem, get_cd, &[]);
            let f = mem.read::<u32>(cd.wrapping_add(0x10));
            call(mem, f, &[cd_map]);
            let f = mem.read::<u32>(cd.wrapping_add(0x14));
            let size = call(mem, f, &[cd_file]);
            if size == 0 {
                return u32::MAX;
            }
            let buf = crate::model::bridge::yi_malloc64(mem, call, size);
            let f = mem.read::<u32>(cd);
            call(mem, f, &[cd_file, buf, 0, local]);
            let f = mem.read::<u32>(cd.wrapping_add(8));
            call(mem, f, &[0]);
            if buf == 0 {
                return u32::MAX;
            }
            MAP_DATA.set(mem, buf);
            for (index, has, flag) in [
                (0u32, super::GRAPHIC_SET_HAS_2D_MAP, 0u32),
                (1, super::GRAPHIC_SET_HAS_3D_MAP, 1),
            ] {
                let s = yi_map_get_section(mem, buf, index);
                if s == 0 {
                    continue;
                }
                let cd = call(mem, get_cd, &[]);
                let f = mem.read::<u32>(cd.wrapping_add(0x10));
                call(mem, f, &[map]);
                let f = c(mem, super::GET_CD_FILE_NAME_PC);
                let name = call(mem, f, &[file]);
                let f = c(mem, has);
                call(mem, f, &[1]);
                let n = super::MAP2D_LOADS.get(mem);
                super::MAP2D_LOADS.set(mem, n.wrapping_add(1));
                let f = c(mem, super::GRAPHIC_FIELD_MAP_LOAD);
                call(mem, f, &[name, flag]);
                super::MAP_KIND.set(mem, flag);
                if flag == 0 {
                    super::MAP_SECTION.set(mem, s);
                    crate::guide::bridge::yn_pre_bg_set_data(mem, s, work, call);
                    let st = RENDER_STATUS.get(mem);
                    RENDER_STATUS.set(mem, st | 0x4000);
                } else {
                    yi_arc_readmem(mem, r, s, image, call);
                }
            }
            let f = c(mem, super::GRAPHIC_FIELD_MAP_INITIALIZE);
            call(mem, f, &[]);
            let f = c(mem, super::GRAPHIC_VFX_LOAD_LEVEL_DATA);
            call(mem, f, &[]);
            let s = yi_map_get_section(mem, buf, alarm.wrapping_add(2));
            if s != 0 {
                let f = c(mem, super::TK_CLEAR_ALARM);
                call(mem, f, &[s]);
            }
            let s = yi_map_get_section(mem, buf, 10);
            if s != 0 {
                super::state::MAP_READ.set(mem, s);
                let f = c(mem, super::PPP_SET_FP_PDT);
                call(mem, f, &[s]);
            }
            for index in [0xc, 0xb] {
                let s = yi_map_get_section(mem, buf, index);
                if s != 0 {
                    crate::guide::bridge::yn_pre_bg_set_data(mem, s, work, call);
                }
            }
            super::CAMERA_IDS.set(mem, [cd_map, cd_file]);
        }
        let st = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, st | 0x11);
        let f = mem.rebase(super::MAP_CIRCLE_ON);
        let on = call(mem, f, &[]);
        DRAW_MASK.set(mem, if on != 0 { u32::MAX } else { 0xffff_fff7 });
        yi_anim_attach_all(mem, r, call);
        0
    }

    /// `yiMapSetup(map)` (`0x00909680`): the debug map loader: after the
    /// reset ([`map_read_reset`]) the map's directory and name
    /// (`0x009161d0`) make `host0:<dir><name>.vpa`, which is read whole
    /// (`sceOpen`/`sceLseek`/`sceRead`/`sceClose`) into a `yiMalloc`
    /// buffer, handed to `yiArc_readmem`, kept as [`MAP_DATA`] with status
    /// `| 0x11`, every object drawn and the default animations attached;
    /// the camera and map ids end -1. Returns 0. `work` is the original's
    /// four 0x400-byte strings, `image` as for `yiArc_readmem`.
    pub fn yi_map_read_host<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        map: u32,
        work: u32,
        image: u32,
        call: &mut C,
    ) -> u32 {
        map_read_reset(mem, false, call);
        let (extra, name, path, dir) = (work, work + 0x400, work + 0x800, work + 0xc00);
        let c = |mem: &M, f: u32| mem.rebase(f);
        let f = c(mem, super::MAP_PATH_PARTS);
        call(mem, f, &[map, dir, name, extra]);
        let sprintf = mem.read::<u32>(c(mem, super::CRT_SPRINTF));
        let format = c(mem, super::HOST_VPA_FORMAT);
        call(mem, sprintf, &[path, format, dir, name]);
        let f = c(mem, super::SCE_OPEN);
        let fd = call(mem, f, &[path, 1]);
        if fd != 0 {
            let seek = c(mem, super::SCE_LSEEK);
            let size = call(mem, seek, &[fd, 0, 2]);
            call(mem, seek, &[fd, 0, 0]);
            let buf = crate::model::bridge::yi_malloc(mem, call, size);
            let f = c(mem, super::SCE_READ);
            call(mem, f, &[fd, buf, size]);
            let f = c(mem, super::SCE_CLOSE);
            call(mem, f, &[fd]);
            if buf != 0 {
                let r = super::state::RENDER.ptr(mem).addr;
                yi_arc_readmem(mem, r, buf, image, call);
                let st = RENDER_STATUS.get(mem);
                RENDER_STATUS.set(mem, st | 0x11);
                MAP_DATA.set(mem, buf);
                DRAW_MASK.set(mem, u32::MAX);
                yi_anim_attach_all(mem, r, call);
            }
        }
        super::CAMERA_IDS.set(mem, [u32::MAX; 2]);
        super::MAP_IDS.set(mem, [u32::MAX; 2]);
        0
    }

    /// `yiAttachAnimDObj(obj, anim, arg)` (`0x00908330`):
    /// `yiKeyFrame_attachDObj` on the renderer.
    pub fn yi_attach_anim_dobj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        anim: u32,
        arg: u32,
        call: &mut C,
    ) -> u32 {
        let r = super::state::RENDER.ptr(mem).addr;
        #[allow(clippy::cast_possible_truncation)] // the callee keeps 16 bits
        yi_key_frame_attach_dobj(mem, r, obj, anim, arg as u16, call)
    }

    /// `yiMallocQuiet(size)` (`0x0090bc10`): `Sg_MemSetErrorIgnore(1)`,
    /// then `user_malloc(size)` (a tail call).
    pub fn yi_malloc_quiet<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        size: u32,
        call: &mut C,
    ) -> u32 {
        super::MEM_ERROR_IGNORE.set(mem, 1);
        crate::model::bridge::rc_alloc(mem, call, size)
    }

    /// `yiRender_resetall(r)` (`0x0090cd50`): `yiRender_resetobj`, then
    /// `+0x9c198`, `+0xbe0ac`, `+0xbe0b0` cleared and `+0x67f34` -1.
    pub fn yi_render_resetall<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        r: u32,
        call: &mut C,
    ) {
        yi_render_resetobj(mem, r, call);
        for o in [0x9c198u32, 0xbe0ac, 0xbe0b0] {
            mem.write(r.wrapping_add(o), 0u32);
        }
        mem.write(r.wrapping_add(0x67f34), u32::MAX);
    }

    /// `yiRender_initpacket(r)` (`0x0090cf00`): the fixed GS packet at
    /// `+0x27624` ([`super::RENDER_PACKET`]). Returns 0.
    pub fn yi_render_initpacket(mem: &mut impl Mem, r: u32) -> u32 {
        for (i, w) in (0u32..).zip(super::RENDER_PACKET) {
            mem.write(r.wrapping_add(0x27624 + 4 * i), w);
        }
        0
    }

    /// `yiRender_initfog(r)` (`0x00913f50`): the fog block at `+0xbe07c`:
    /// zeros, 0x60, a grey `0x80 0x80 0x80`, three small factors and 64.0
    /// (the `.rdata` floats of [`super::FOG_DEFAULTS`]).
    pub fn yi_render_initfog(mem: &mut impl Mem, r: u32) {
        let at = |o: u32| r.wrapping_add(o);
        for o in [0xbe080, 0xbe084] {
            mem.write(at(o), 0u32);
        }
        mem.write(at(0xbe0a0), 0x60u32);
        mem.write(at(0xbe088), 0u32);
        mem.write(at(0xbe0a4), 0x8080u16);
        mem.write(at(0xbe08c), 0u32);
        mem.write(at(0xbe0a6), 0x80u8);
        for (o, c) in [0xbe090, 0xbe094, 0xbe098]
            .into_iter()
            .zip(super::FOG_DEFAULTS)
        {
            let v = mem.read::<u32>(mem.rebase(c));
            mem.write(at(o), x87::copy(v));
        }
        mem.write(at(0xbe07c), 0u32);
        let v = mem.read::<u32>(mem.rebase(super::FOG_DEFAULTS[3]));
        mem.write(at(0xbe09c), x87::copy(v));
    }

    /// `yiRotMatrixSeq(m, angles, order)` (`0x009151c0`): `m` rotated in
    /// place by the three angles (`FFXVu0RotMatrixX/Y/Z`) in one of six
    /// orders (0 XYZ, 1 YXZ, 2 ZXY, 3 XZY, 4 YZX, 5 ZYX; others none).
    /// Returns 0.
    pub fn yi_rot_matrix_seq<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        angles: u32,
        order: u32,
        call: &mut C,
    ) -> u32 {
        const ORDERS: [[u32; 3]; 6] = [
            [0, 1, 2],
            [1, 0, 2],
            [2, 0, 1],
            [0, 2, 1],
            [1, 2, 0],
            [2, 1, 0],
        ];
        let Some(seq) = ORDERS.get(order as usize) else {
            return 0;
        };
        for &axis in seq {
            let f = mem.rebase(super::VU0_ROT_MATRIX[axis as usize]);
            let a = x87::copy(mem.read::<u32>(angles.wrapping_add(4 * axis)));
            call(mem, f, &[m, m, a]);
        }
        0
    }

    /// `yiFieldMapRead(map, alarm)` (`0x009085d0`): `yiMapReadMaster(0x13,
    /// 2 map, 0x13, 2 map + 1, alarm)` (through `call`), status bit 0x2000.
    pub fn yi_field_map_read<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        map: u32,
        alarm: u32,
        call: &mut C,
    ) {
        map_read(mem, 0x13, map, alarm, 0x2000, call);
    }

    /// `yiBattleMapRead(map, alarm)` (`0x00908370`): the same with CD
    /// directory 0x1a, status bit 0x1000.
    pub fn yi_battle_map_read<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        map: u32,
        alarm: u32,
        call: &mut C,
    ) {
        map_read(mem, 0x1a, map, alarm, 0x1000, call);
    }

    fn map_read<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        dir: u32,
        map: u32,
        alarm: u32,
        bit: u32,
        call: &mut C,
    ) {
        let n = map.wrapping_mul(2);
        let f = mem.rebase(super::YI_MAP_READ_MASTER);
        call(mem, f, &[dir, n, dir, n.wrapping_add(1), alarm]);
        let st = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, st | bit);
    }

    /// `yiGetMapName(out)` (`0x00907e70`): the current map's name
    /// (`sprintf(out, "%s", …)`: the entry of the name table `0x025d5888`,
    /// `0x01934ec8` entries of 16 bytes with the map at `+0xc`, whose map
    /// is `g_CurrentMapNo`; a default string without one, another without
    /// a map).
    pub fn yi_get_map_name<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        out: u32,
        call: &mut C,
    ) -> u32 {
        let cur = super::CURRENT_MAP.get(mem);
        let name = if cur == 0 {
            mem.rebase(super::MAP_NAME_NONE)
        } else {
            let n = super::MAP_NAME_COUNT.get(mem).cast_signed();
            let table = super::MAP_NAMES.get(mem);
            (0..n.max(0).cast_unsigned())
                .find(|&i| mem.read::<u32>(table.wrapping_add(16 * i + 0xc)) == cur)
                .map_or_else(
                    || mem.rebase(super::MAP_NAME_UNKNOWN),
                    |i| table.wrapping_add(16 * i),
                )
        };
        let sprintf = mem.read::<u32>(mem.rebase(super::CRT_SPRINTF));
        let fmt = mem.rebase(super::FORMAT_S);
        call(mem, sprintf, &[out, fmt, name])
    }

    /// The battle view set-up of `yiSetBattleView`/`yiSetViewMatrices`:
    /// `view * g_YiViewAdjust` (`FFXVu0MulMatrix` into `work`, 0x40 bytes)
    /// copied to the three view matrices (`0x018ded78`, `0x018dedb8`,
    /// `0x018dedf8`), their translation words `0x018dede8..f0` zero, and the
    /// screen depth (`MsCameraGetScrDpt`, `ST0`) to `0x018df160`.
    fn battle_view<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        view: u32,
        work: u32,
        call: &mut C,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) {
        let adjust = VIEW_ADJUST.ptr(mem).addr;
        let f = mem.rebase(super::VU0_MUL_MATRIX);
        call(mem, f, &[work, view, adjust]);
        for m in super::BATTLE_VIEWS {
            let at = mem.rebase(m);
            ffx_ps2::vu0::bridge::matrix_copy(mem, at, work);
        }
        for o in [0x018d_edec, 0x018d_edf0, 0x018d_ede8] {
            let at = mem.rebase(o);
            mem.write(at, 0u32);
        }
        let f = mem.rebase(super::MS_CAMERA_GET_SCR_DPT);
        let d = fcall(mem, f, crate::camera::FArgs::Stack(&[]));
        let at = mem.rebase(super::SCREEN_DEPTH);
        mem.write(at, ffx_bridge::x87::Reg::constant(d).store());
    }

    /// `yiSetBattleView(view)` (`0x0090a140`): unless status bit 0x800 is
    /// set or `view` is null, the battle view set-up ([`battle_view`]).
    pub fn yi_set_battle_view<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        view: u32,
        work: u32,
        call: &mut C,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) {
        if RENDER_STATUS.get(mem) & 0x800 == 0 && view != 0 {
            battle_view(mem, view, work, call, fcall);
        }
    }

    /// `yiSetViewMatrices(m, view)` (`0x0090a640`): with `m`, the view
    /// matrix (`0x018ded38`) and the two floats after it from `m + 0x20`;
    /// with `view`, the battle view set-up ([`battle_view`]).
    pub fn yi_set_view_matrices<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        view: u32,
        work: u32,
        call: &mut C,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) {
        if m != 0 {
            let at = super::state::VIEW_MATRIX.ptr(mem).addr;
            ffx_ps2::vu0::bridge::matrix_copy(mem, at, m);
            let a = x87::copy(mem.read::<u32>(m + 0x20));
            let b = x87::copy(mem.read::<u32>(m + 0x24));
            super::state::VIEW_EXTRA.set(mem, [a, b]);
        }
        if view != 0 {
            battle_view(mem, view, work, call, fcall);
        }
    }

    /// `yiSetLightMatrices()` (`0x00908d00`): the three light directions
    /// (`Sg_LightGetDir(i, …)`, normalised) and four colours
    /// (`Sg_LightGetColor`) into the normal light matrix (`0x018deeb8`,
    /// `FFXVu0NormalLightMatrix`) and the light colour matrix
    /// (`0x018deef8`, `FFXVu0LightColorMatrix`). `work` holds the seven
    /// vectors (0x70 bytes). Returns 0.
    pub fn yi_set_light_matrices<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        work: u32,
        call: &mut C,
    ) -> u32 {
        // the original's frame from ebp - 0x74: colours 0, 1, 2, 3 at
        // 0x00, 0x20, 0x10, 0x30; directions 0, 2, 1 at 0x40, 0x50, 0x60
        let (c0, c2, c1, c3) = (work, work + 0x10, work + 0x20, work + 0x30);
        let (d0, d2, d1) = (work + 0x40, work + 0x50, work + 0x60);
        let f = mem.rebase(super::SG_LIGHT_GET_DIR);
        for (i, d) in [(0u32, d0), (1, d1), (2, d2)] {
            call(mem, f, &[i, d]);
        }
        for d in [d0, d1, d2] {
            ffx_ps2::vu0::bridge::normalize(mem, d, d);
        }
        let f = mem.rebase(super::SG_LIGHT_GET_COLOR);
        for (i, c) in [(0u32, c0), (1, c1), (2, c2), (3, c3)] {
            call(mem, f, &[i, c]);
        }
        let f = mem.rebase(super::VU0_NORMAL_LIGHT_MATRIX);
        let m = mem.rebase(super::NORMAL_LIGHT);
        call(mem, f, &[m, d0, d1, d2]);
        let f = mem.rebase(super::VU0_LIGHT_COLOR_MATRIX);
        let m = mem.rebase(super::LIGHT_COLOR);
        call(mem, f, &[m, c0, c1, c2, c3]);
        0
    }

    /// `yiImageVramAlloc(img)` (`0x00917d80`): GS memory for an image by its
    /// format (`+0xc`): 0 and 1 0x10000 bytes, 2 0x8000, 0x13 0x4000 (below
    /// page 0x2e00 it becomes 0x1b), 0x14 the cached page `0x025d17a4` when
    /// set (format 0x24, the cache cleared) else 0x2000 (below 0x2e00 format
    /// 0x2c and cached); the address to `+0x4` (`yiVramAllocPsm`). Other
    /// formats nothing. Returns 0.
    pub fn yi_image_vram_alloc(mem: &mut impl Mem, img: u32) -> u32 {
        let (bytes, psm) = match mem.read::<u32>(img + 0xc) {
            0 | 1 => (0x10000, 0),
            2 => (0x8000, 2),
            0x13 => (0x4000, 0x13),
            0x14 => {
                let c = super::IMAGE_PAGE_CACHE.get(mem);
                if c != u32::MAX {
                    mem.write(img + 4, c);
                    mem.write(img + 0xc, 0x24u32);
                    super::IMAGE_PAGE_CACHE.set(mem, u32::MAX);
                    return 0;
                }
                (0x2000, 0x14)
            }
            _ => return 0,
        };
        let a = yi_vram_alloc_psm(mem, bytes, psm);
        mem.write(img + 4, a);
        if a < 0x2e00 {
            match psm {
                0x13 => mem.write(img + 0xc, 0x1bu32),
                0x14 => {
                    mem.write(img + 0xc, 0x2cu32);
                    super::IMAGE_PAGE_CACHE.set(mem, a);
                }
                _ => {}
            }
        }
        0
    }

    // ---- round 12 ----

    /// `yiImage_dumpall(rman)` (`0x00918680`): prints every page's images
    /// through `dbgPrintf`, a bare `ret` on PC; returns 0.
    #[must_use]
    pub fn yi_image_dumpall(_rman: u32) -> u32 {
        0
    }

    /// `yiRender_addDirectBlock(render, src, n)` (`0x0090c120`): aligns
    /// the data pointer (`+0xba3e0`) to 64 bytes (in 16-byte steps), adds
    /// a DMA ref tag (`0x30000000 | (n + 1)`, the block) at `+0xba3f0`
    /// (counted at `+0xba3f8`), then the block: VIF FLUSH, STMOD 0,
    /// STCYCL 1,1, DIRECTHL `n`, and `n` quadwords from `src`. Returns
    /// `src + 16n`.
    pub fn yi_render_add_direct_block(mem: &mut impl Mem, r: u32, src: u32, n: u32) -> u32 {
        let data = r.wrapping_add(0xba3e0);
        let tags = mem.read::<u32>(r.wrapping_add(0xba3f0));
        let d = mem.read::<u32>(data);
        if d & 0x3f != 0 {
            mem.write(data, ((0x40 - (d & 0x3f)) & 0xffff_fff0).wrapping_add(d));
        }
        let block = mem.read::<u32>(data);
        mem.write(data, n.wrapping_add(1).wrapping_shl(4).wrapping_add(block));
        mem.write(tags, (n.wrapping_add(1) & 0xffff) | 0x3000_0000);
        mem.write(tags.wrapping_add(4), block);
        mem.write(tags.wrapping_add(8), 0u32);
        mem.write(tags.wrapping_add(0xc), 0u32);
        let at = r.wrapping_add(0xba3f0);
        let v = mem.read::<u32>(at).wrapping_add(0x10);
        mem.write(at, v);
        let at = r.wrapping_add(0xba3f8);
        let v = mem.read::<u32>(at).wrapping_add(1);
        mem.write(at, v);
        mem.write(block.wrapping_add(0xc), n | 0x5100_0000);
        mem.write(block, 0x1100_0000u32);
        mem.write(block.wrapping_add(4), 0x0500_0000u32);
        mem.write(block.wrapping_add(8), 0x0100_0404u32);
        let (mut s, mut d) = (src, block.wrapping_add(0x10));
        for _ in 0..n {
            let q = mem.read::<[u32; 4]>(s);
            mem.write(d, q);
            s = s.wrapping_add(0x10);
            d = d.wrapping_add(0x10);
        }
        s
    }

    /// `yiTriContainsXZ(tri, p)` (`0x00910270`): 1 when `p`'s x, z lie
    /// inside the triangle of the three vectors at `tri` (projected on
    /// x, z), by two edge cross products (`f32`) against the full one
    /// (their sum at 53 bits), for either winding; else 0.
    pub fn yi_tri_contains_xz(mem: &impl Mem, tri: u32, p: u32) -> u32 {
        let f = |o: u32| x87::load(mem.read(tri.wrapping_add(o)));
        let a = f(0x28) - f(0x8);
        let b = f(0x10) - f(0);
        let c = f(0x18) - f(0x8);
        let d = f(0x20) - f(0);
        let full = x87::load(x87::store(b * a - d * c));
        let e = x87::load(mem.read(p)) - f(0);
        let g = x87::load(mem.read(p.wrapping_add(8))) - f(0x8);
        let s1 = x87::load(x87::store(a * e - d * g));
        let s3 = x87::load(x87::store(-c * e + b * g));
        if 0.0 < s1 && 0.0 < s3 && s3 + s1 < full {
            return 1;
        }
        u32::from(s1 < 0.0 && s3 < 0.0 && full < s1 + s3)
    }

    /// `yiBoundsMinMax(obj, min, max)` (`0x009130f0`): `min` and `max`
    /// start at the largest finite floats ([`super::BOUNDS_INIT`]) with w
    /// 1.0, then take each of the `+0x12` (`i16`) vectors at `obj + +8`
    /// (16 bytes apart), x, y, z in turn. Returns the count when there are
    /// vectors, else 0.
    pub fn yi_bounds_min_max(mem: &mut impl Mem, obj: u32, min: u32, max: u32) -> u32 {
        let first = mem.read::<u32>(obj.wrapping_add(8));
        let hi = x87::copy(mem.read(mem.rebase(super::BOUNDS_INIT[0])));
        let lo = x87::copy(mem.read(mem.rebase(super::BOUNDS_INIT[1])));
        for i in 0..3u32 {
            mem.write(min.wrapping_add(4 * i), hi);
            mem.write(max.wrapping_add(4 * i), lo);
        }
        mem.write(min.wrapping_add(0xc), 1f32);
        mem.write(max.wrapping_add(0xc), 1f32);
        let mut v = obj.wrapping_add(first);
        let mut n = 0i32;
        let mut r = 0u32;
        while i32::from(mem.read::<i16>(obj.wrapping_add(0x12))) > n {
            for i in 0..3u32 {
                let x = mem.read::<f32>(v.wrapping_add(4 * i));
                if x < mem.read::<f32>(min.wrapping_add(4 * i)) {
                    mem.write(min.wrapping_add(4 * i), x87::copy(x.to_bits()));
                }
                let x = mem.read::<f32>(v.wrapping_add(4 * i));
                if x > mem.read::<f32>(max.wrapping_add(4 * i)) {
                    mem.write(max.wrapping_add(4 * i), x87::copy(x.to_bits()));
                }
            }
            r = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
            n += 1;
            v = v.wrapping_add(0x10);
        }
        r
    }

    // ---- round 13 ----

    /// `yiKeyFrameWrapAngles(v)` (`0x00915120`): each of the four floats
    /// past pi brought down by `(1 - ftol(x / -2pi)) * 2pi`, then each below
    /// -pi brought up the same way (the constants are `.rdata` doubles,
    /// [`super::WRAP_CONSTS`]).
    pub fn yi_key_frame_wrap_angles(mem: &mut impl Mem, v: u32) {
        let [pi, neg_two_pi, two_pi, neg_pi] =
            super::WRAP_CONSTS.map(|at| mem.read::<f64>(mem.rebase(at)));
        for i in 0..4u32 {
            let at = v.wrapping_add(4 * i);
            let x = x87::load(mem.read(at));
            if x > pi {
                let k = 1i32.wrapping_sub(ftol(mem, x / neg_two_pi).cast_signed());
                let kf = x87::load(x87::store(f64::from(k)));
                mem.write(at, x87::store(x - kf * two_pi));
            }
            let x = x87::load(mem.read(at));
            if x < neg_pi {
                let k = 1i32.wrapping_sub(ftol(mem, x / neg_two_pi).cast_signed());
                let kf = x87::load(x87::store(f64::from(k)));
                let x = x87::load(mem.read(at));
                mem.write(at, x87::store(kf * two_pi + x));
            }
        }
    }

    /// `yiImageTakeLastPage(img, page)` (`0x00917e70`): when the image's
    /// pages (`page +0x140`) fit the last area
    /// (`(0x025d1798 - 0x025d17a0) >> 8`): with the area unused (`0x025d179c` at its start), a PSMT8
    /// image becomes 0x1b and a PSMT4 0x2c (remembered at `0x0193bb24`),
    /// placed there (`+4`, `+0x14`) and the area marked used; with it used
    /// and a PSMT4 remembered, a PSMT4 image becomes 0x24 and shares it
    /// (returns 1). Else 0.
    pub fn yi_image_take_last_page(mem: &mut impl Mem, img: u32, page: u32) -> u32 {
        let (end, used, start) = (
            super::LAST_AREA_END,
            super::LAST_AREA_USED,
            super::LAST_AREA_START,
        );
        let start_v = start.get(mem);
        let pages = end.get(mem).wrapping_sub(start_v) >> 8;
        if mem.read::<u32>(page.wrapping_add(0x140)) > pages {
            return 0;
        }
        let flag = super::LAST_AREA_PSMT4;
        if used.get(mem) == start_v {
            flag.set(mem, 0);
            match mem.read::<u32>(img.wrapping_add(0xc)) {
                0x13 => mem.write(img.wrapping_add(0xc), 0x1bu32),
                0x14 => {
                    flag.set(mem, 1);
                    mem.write(img.wrapping_add(0xc), 0x2cu32);
                }
                _ => return 0,
            }
            mem.write(img.wrapping_add(0x14), pages);
            let s = start.get(mem) >> 6;
            mem.write(img.wrapping_add(4), s);
            let e = end.get(mem);
            used.set(mem, e);
            return 0;
        }
        if flag.get(mem) == 0 || mem.read::<u32>(img.wrapping_add(0xc)) != 0x14 {
            return 0;
        }
        mem.write(img.wrapping_add(0xc), 0x24u32);
        let s = start.get(mem) >> 6;
        mem.write(img.wrapping_add(4), s);
        mem.write(img.wrapping_add(0x14), pages);
        flag.set(mem, 0);
        1
    }

    /// `yiImageRectFree(page, list, img)` (`0x00917490`): 1 when the
    /// image's rectangle (`+0x11c` x, `+0x120` y, `+0x13c` w, `+0x140` h)
    /// lies inside the page (`+0x10`, `+0x14`), aligned to its size when
    /// that is a power of two, and overlaps none of the other images in
    /// the list (linked by `+0x15c`); else 0. A zero power-of-two size
    /// divides by zero in the original; this gives 0.
    pub fn yi_image_rect_free(mem: &impl Mem, page: u32, list: u32, img: u32) -> u32 {
        let y = mem.read::<u32>(img.wrapping_add(0x120));
        let w = mem.read::<u32>(img.wrapping_add(0x13c));
        let x = mem.read::<u32>(img.wrapping_add(0x11c));
        let h = mem.read::<u32>(img.wrapping_add(0x140));
        let right = x.wrapping_sub(1).wrapping_add(w);
        let bottom = h.wrapping_sub(1).wrapping_add(y);
        if right >= mem.read::<u32>(page.wrapping_add(0x10))
            || bottom >= mem.read::<u32>(page.wrapping_add(0x14))
            || x.cast_signed() < 0
            || y.cast_signed() < 0
        {
            return 0;
        }
        for (v, size) in [(x, w), (y, h)] {
            if super::is_pow2(size) && v.checked_rem(size) != Some(0) {
                return 0;
            }
        }
        let (x, y, right, bottom) = (
            x.cast_signed(),
            y.cast_signed(),
            right.cast_signed(),
            bottom.cast_signed(),
        );
        let within = |v: i32, lo: i32, hi: i32| lo <= v && v <= hi;
        let mut o = list;
        while o != 0 {
            if o != img {
                let ox = mem.read::<i32>(o.wrapping_add(0x11c));
                let oy = mem.read::<i32>(o.wrapping_add(0x120));
                let oright = mem
                    .read::<i32>(o.wrapping_add(0x13c))
                    .wrapping_sub(1)
                    .wrapping_add(ox);
                let obottom = mem
                    .read::<i32>(o.wrapping_add(0x140))
                    .wrapping_sub(1)
                    .wrapping_add(oy);
                let across = within(x, ox, oright)
                    || within(right, ox, oright)
                    || within(ox, x, right)
                    || within(oright, x, right);
                let down = within(y, oy, obottom)
                    || within(bottom, oy, obottom)
                    || within(oy, y, bottom)
                    || within(obottom, y, bottom);
                if across && down {
                    return 0;
                }
            }
            o = ptr32(mem.read(o.wrapping_add(0x15c)));
        }
        1
    }

    /// `yiVramSetup(pages, mode)` (`0x00917f20`): the image VRAM areas for
    /// mode 0, 1 or 2 (others keep theirs), then the shared ones: next
    /// 0xbc000, the limit `(0x38 - pages) << 14`, 0xf0000, 0x100000 and
    /// the page cache -1. Returns the limit.
    pub fn yi_vram_setup(mem: &mut impl Mem, pages: u32, mode: u32) -> u32 {
        let mut put = |off: u32, v: u32| {
            let at = mem.rebase(super::VRAM_AREAS + off);
            mem.write(at, v);
        };
        // offsets from 0x025d1798
        match mode {
            0 => {
                for (o, v) in [
                    (0x1c, 0x9_0000),
                    (0x18, 0),
                    (0x28, 0),
                    (0x38, 0),
                    (4, 0x9_0000),
                    (8, 0x9_0000),
                    (0, 0x9_c000),
                ] {
                    put(o, v);
                }
            }
            1 => {
                for (o, v) in [
                    (0x2c, 0x1_c000),
                    (0x3c, 0x1_c000),
                    (0x1c, 0xa_c000),
                    (0x18, 0),
                    (0x28, 0),
                    (0x38, 0),
                    (4, 0xa_c000),
                    (8, 0xa_c000),
                    (0, 0xb_8000),
                ] {
                    put(o, v);
                }
            }
            2 => {
                for (o, v) in [
                    (0x1c, 0x6_0000),
                    (0x28, 0x8_4000),
                    (0x38, 0x8_4000),
                    (0x18, 0xb_4000),
                    (4, 0x6_0000),
                    (8, 0x6_0000),
                    (0, 0x6_8000),
                ] {
                    put(o, v);
                }
            }
            _ => {}
        }
        if mode == 0 || mode == 2 {
            put(0x3c, 0);
            put(0x2c, 0);
        }
        let limit = 0x38u32.wrapping_sub(pages).wrapping_shl(14);
        for (o, v) in [
            (0x24, 0xb_c000),
            (0x34, 0xb_c000),
            (0x14, limit),
            (0x20, 0xf_0000),
            (0x30, 0xf_0000),
            (0x10, 0x10_0000),
            (0xc, u32::MAX),
        ] {
            put(o, v);
        }
        limit
    }

    // ---- round 14 ----

    /// `yiImageRman_serch(rman, page, img)` (`0x00917b70`): a free place
    /// for the image in the page: right of, below, left of and above each
    /// image already there (from `+0x1c`, linked by `+0x15c`), then the
    /// page's bottom-left and bottom-right corners, each tried with
    /// [`yi_image_rect_free`]; 1 when one fits (the image's `+0x11c`/
    /// `+0x120` left there), else 0.
    pub fn yi_image_rman_serch(mem: &mut impl Mem, page: u32, img: u32) -> u32 {
        let mut o = mem.read::<u32>(page.wrapping_add(0x1c));
        while o != 0 {
            let x = mem
                .read::<u32>(o.wrapping_add(0x13c))
                .wrapping_add(mem.read(o.wrapping_add(0x11c)));
            let y = mem.read::<u32>(o.wrapping_add(0x120));
            if image_fits_at(mem, page, img, x, y) {
                return 1;
            }
            let x = mem.read::<u32>(o.wrapping_add(0x11c));
            let y = mem
                .read::<u32>(o.wrapping_add(0x140))
                .wrapping_add(mem.read(o.wrapping_add(0x120)));
            if image_fits_at(mem, page, img, x, y) {
                return 1;
            }
            let x = mem
                .read::<u32>(o.wrapping_add(0x11c))
                .wrapping_sub(mem.read(img.wrapping_add(0x13c)));
            let y = mem.read::<u32>(o.wrapping_add(0x120));
            if image_fits_at(mem, page, img, x, y) {
                return 1;
            }
            let x = mem.read::<u32>(o.wrapping_add(0x11c));
            let y = mem
                .read::<u32>(o.wrapping_add(0x120))
                .wrapping_sub(mem.read(img.wrapping_add(0x140)));
            if image_fits_at(mem, page, img, x, y) {
                return 1;
            }
            o = ptr32(mem.read(o.wrapping_add(0x15c)));
        }
        let y = mem
            .read::<u32>(page.wrapping_add(0x14))
            .wrapping_sub(mem.read(img.wrapping_add(0x140)));
        if image_fits_at(mem, page, img, 0, y) {
            return 1;
        }
        let x = mem
            .read::<u32>(page.wrapping_add(0x10))
            .wrapping_sub(mem.read(img.wrapping_add(0x13c)));
        let y = mem
            .read::<u32>(page.wrapping_add(0x14))
            .wrapping_sub(mem.read(img.wrapping_add(0x140)));
        u32::from(image_fits_at(mem, page, img, x, y))
    }

    /// The image placed at (`x`, `y`) (`+0x11c`, `+0x120`), then
    /// [`yi_image_rect_free`] against the page's list.
    fn image_fits_at(mem: &mut impl Mem, page: u32, img: u32, x: u32, y: u32) -> bool {
        mem.write(img.wrapping_add(0x11c), x);
        mem.write(img.wrapping_add(0x120), y);
        let list = mem.read::<u32>(page.wrapping_add(0x1c));
        yi_image_rect_free(mem, page, list, img) != 0
    }

    /// `yiKeyFrameAdvance(r, a, now)` (`0x00914180`): a key frame track
    /// timed by `now`. Paused (`Sg_GetPause`): its start (`+0x14`) moves on
    /// by the time since `+0x18` (returned). Else the start is clamped to
    /// `now`, and while `now` passes the current frame's end (start + 2 x
    /// the record's `+4`; records at `r +0x9d5c4[+8]`, 32 bytes, indexed by
    /// `+0xa`) the frame advances; past the last (count `r +0x9d6c4[+8]`)
    /// a looping animation (`+2` 0) restarts, a one-shot one is cleared
    /// (`r +0x9cabc` counts down). Returns `a` otherwise.
    pub fn yi_key_frame_advance<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        r: u32,
        a: u32,
        now: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::SG_GET_PAUSE);
        if call(mem, f, &[]) != 0 {
            let d = now.wrapping_sub(mem.read::<u32>(a.wrapping_add(0x18)));
            mem.write(a.wrapping_add(0x18), now);
            let s = mem.read::<u32>(a.wrapping_add(0x14)).wrapping_add(d);
            mem.write(a.wrapping_add(0x14), s);
            return d;
        }
        if mem.read::<u32>(a.wrapping_add(0x14)) > now {
            mem.write(a.wrapping_add(0x14), now);
        }
        let idx = u32::from(mem.read::<u16>(a.wrapping_add(8)));
        let mut start = mem.read::<u32>(a.wrapping_add(0x14));
        let rec = u32::from(mem.read::<u16>(a.wrapping_add(0xa)))
            .wrapping_shl(5)
            .wrapping_add(mem.read::<u32>(r.wrapping_add(0x9d5c4 + idx * 4)));
        mem.write(a.wrapping_add(0x18), now);
        let end = |mem: &M, start: u32| {
            start.wrapping_add(mem.read::<u32>(rec.wrapping_add(4)).wrapping_mul(2))
        };
        if now < end(mem, start) {
            return a;
        }
        loop {
            let fr = mem.read::<u16>(a.wrapping_add(0xa)).wrapping_add(1);
            mem.write(a.wrapping_add(0xa), fr);
            let frames = mem.read::<u32>(r.wrapping_add(idx * 4 + 0x9d6c4));
            if frames > u32::from(mem.read::<u16>(a.wrapping_add(0xa))) {
                let e = end(mem, start);
                mem.write(a.wrapping_add(0x14), e);
            } else if mem.read::<u16>(a.wrapping_add(2)) != 0 {
                mem.write(a, 0u16);
                mem.write(a.wrapping_add(8), 0u32);
                mem.write(a.wrapping_add(6), 0u16);
                mem.write(a.wrapping_add(0x14), now);
                mem.write(a.wrapping_add(0x18), now);
                let at = r.wrapping_add(0x9cabc);
                let c = mem.read::<u32>(at).wrapping_sub(1);
                mem.write(at, c);
                mem.write(a.wrapping_add(0x24), 0u32);
                return a;
            } else {
                let e = end(mem, start);
                mem.write(a.wrapping_add(0x14), e);
                mem.write(a.wrapping_add(0xa), 0u16);
            }
            start = mem.read::<u32>(a.wrapping_add(0x14));
            mem.write(a.wrapping_add(0x18), now);
            if now < end(mem, start) {
                return a;
            }
        }
    }

    // ---- round 15 ----

    /// The cubic Hermite weights for `t` (`f32` bits) as the original
    /// stores them (`f32` each): `(t - 1)^2 (2t + 1)`, `t^2 (C - 2t)`,
    /// `(1 - t)^2 t`, `t^2 (t - 1)`, `C` the `.rdata` double 3.0
    /// ([`super::HERMITE_THREE`]); products in the original's order.
    fn hermite_weights(mem: &impl Mem, t: u32) -> [f64; 4] {
        let t = x87::load(t);
        let c = mem.read::<f64>(mem.rebase(super::HERMITE_THREE));
        let tm1 = t - 1.0;
        let two_t = t + t;
        let h0 = tm1 * tm1 * (two_t + 1.0);
        let tt = t * t;
        let h1 = (c - two_t) * tt;
        let omt = 1.0 - t;
        let h2 = omt * omt * t;
        let h3 = tt * tm1;
        [h0, h1, h2, h3].map(|h| x87::load(x87::store(h)))
    }

    /// `yiHermiteVec(t, p, out, n)`: `out[i] = ((h0 p0[i] + h1 p1[i]) + h2
    /// p2[i]) + h3 p3[i]` for the first `n` floats, each stored before the
    /// next is read.
    fn hermite(mem: &mut impl Mem, t: u32, p: [u32; 4], out: u32, n: u32) {
        let [h0, h1, h2, h3] = hermite_weights(mem, t);
        for i in 0..n {
            let v = |mem: &mut _, k: usize| {
                x87::load(Mem::read::<u32>(&*mem, p[k].wrapping_add(4 * i)))
            };
            let (a, b, c, d) = (v(mem, 0), v(mem, 1), v(mem, 2), v(mem, 3));
            let s = h0 * a + h1 * b + h2 * c + h3 * d;
            mem.write(out.wrapping_add(4 * i), x87::store(s));
        }
    }

    /// `yiHermiteVec2(t, p0, p1, p2, p3, out)` (`0x00914700`): two floats
    /// of a cubic Hermite curve ([`hermite_weights`]).
    pub fn yi_hermite_vec2(mem: &mut impl Mem, t: u32, p: [u32; 4], out: u32) {
        hermite(mem, t, p, out, 2);
    }

    /// `yiHermiteVec4(t, p0, p1, p2, p3, out)` (`0x009147b0`): four floats.
    pub fn yi_hermite_vec4(mem: &mut impl Mem, t: u32, p: [u32; 4], out: u32) {
        hermite(mem, t, p, out, 4);
    }

    /// `yiKeyFrameAdvanceLinked(r, track, now)` (`0x00914070`): as
    /// [`yi_key_frame_advance`] for tracks whose 0x80-byte records chain
    /// (`+0x24` the current one, its `+4` the length, byte bit 0 the end):
    /// paused, the start moves on (returns `now`); else frames advance
    /// while `now` passes the current end, past the count
    /// (`r +0x9d6c4[+8]`) restarting from `r +0x9d4c4[+8]` or, for a
    /// one-shot track, clearing once the last frame ended; a record of
    /// length 0 stops. Returns `track`.
    pub fn yi_key_frame_advance_linked<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        r: u32,
        a: u32,
        now: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::SG_GET_PAUSE);
        if call(mem, f, &[]) != 0 {
            let d = now.wrapping_sub(mem.read::<u32>(a.wrapping_add(0x18)));
            mem.write(a.wrapping_add(0x18), now);
            let s = mem.read::<u32>(a.wrapping_add(0x14)).wrapping_add(d);
            mem.write(a.wrapping_add(0x14), s);
            return now;
        }
        if mem.read::<u32>(a.wrapping_add(0x14)) > now {
            mem.write(a.wrapping_add(0x14), now);
        }
        let mut rec = mem.read::<u32>(a.wrapping_add(0x24));
        let mut start = mem.read::<u32>(a.wrapping_add(0x14));
        mem.write(a.wrapping_add(0x18), now);
        let len = |mem: &M, rec: u32| mem.read::<u32>(rec.wrapping_add(4));
        if now < len(mem, rec).wrapping_add(start) {
            return a;
        }
        loop {
            if mem.read::<u8>(rec) & 1 != 0 {
                mem.write(a, 0u16);
                mem.write(a.wrapping_add(0x14), now);
                mem.write(a.wrapping_add(0x18), now);
                mem.write(a.wrapping_add(8), 0u32);
                mem.write(a.wrapping_add(6), 0u16);
                mem.write(a.wrapping_add(0x24), 0u32);
                return a;
            }
            let fr = mem.read::<u16>(a.wrapping_add(0xa)).wrapping_add(1);
            mem.write(a.wrapping_add(0xa), fr);
            let idx = u32::from(mem.read::<u16>(a.wrapping_add(8)));
            let fr = u32::from(mem.read::<u16>(a.wrapping_add(0xa)));
            let next = if mem.read::<u32>(r.wrapping_add(idx * 4 + 0x9d6c4)) > fr {
                let e = len(mem, rec).wrapping_add(start);
                mem.write(a.wrapping_add(0x14), e);
                mem.write(a.wrapping_add(0x18), now);
                let n = rec.wrapping_add(0x80);
                mem.write(a.wrapping_add(0x24), n);
                n
            } else if mem.read::<u16>(a.wrapping_add(2)) != 0 {
                let cur = mem.read::<u32>(a.wrapping_add(0x24));
                if now <= len(mem, cur).wrapping_add(mem.read(a.wrapping_add(0x14))) {
                    return a;
                }
                mem.write(a, 0u16);
                mem.write(a.wrapping_add(8), 0u32);
                mem.write(a.wrapping_add(6), 0u16);
                mem.write(a.wrapping_add(0x14), now);
                mem.write(a.wrapping_add(0x18), now);
                let at = r.wrapping_add(0x9cabc);
                let c = mem.read::<u32>(at).wrapping_sub(1);
                mem.write(at, c);
                mem.write(a.wrapping_add(0x24), 0u32);
                return a;
            } else {
                let e = len(mem, rec).wrapping_add(start);
                mem.write(a.wrapping_add(0x14), e);
                mem.write(a.wrapping_add(0x18), now);
                let first = mem.read::<u32>(r.wrapping_add(idx * 4 + 0x9d4c4));
                mem.write(a.wrapping_add(0x24), first);
                mem.write(a.wrapping_add(0xa), 0u16);
                mem.read::<u32>(a.wrapping_add(0x24))
            };
            if len(mem, next) == 0 {
                return a;
            }
            rec = mem.read::<u32>(a.wrapping_add(0x24));
            start = mem.read::<u32>(a.wrapping_add(0x14));
            if now < len(mem, rec).wrapping_add(start) {
                return a;
            }
        }
    }

    /// `yiRectTriOverlapXZ(tri, m, a, b, work)` (`0x00910070`): 1 when the
    /// x/z rectangle from `a` to `b` and the triangle `tri` overlap on x,
    /// z (a triangle corner in either half of the rectangle, or a
    /// rectangle corner in the triangle, by [`yi_tri_contains_xz`]); else
    /// 0. The triangle is three vectors transformed by `m`
    /// (`FFXVu0ApplyMatrix`, their w set to 1.0 first), or without `m`
    /// three times four unsigned integers as floats. `work` holds the
    /// original's locals (0x80 bytes: the rectangle's corners, then the
    /// triangle).
    pub fn yi_rect_tri_overlap_xz<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        tri: u32,
        m: u32,
        ab: [u32; 2],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [a, b] = ab;
        let (rect, t) = (work, work.wrapping_add(0x50));
        let corners = [(a, a), (b, a), (b, b), (a, b), (a, a)];
        for (i, (x, z)) in (0u32..).zip(corners) {
            let vx = x87::copy(mem.read(x));
            mem.write(rect.wrapping_add(0x10 * i), vx);
            let vz = x87::copy(mem.read(z.wrapping_add(8)));
            mem.write(rect.wrapping_add(0x10 * i + 8), vz);
        }
        if m == 0 {
            for i in 0..12u32 {
                let u = mem.read::<u32>(tri.wrapping_add(4 * i));
                mem.write(t.wrapping_add(4 * i), x87::store(f64::from(u)));
            }
        } else {
            let f = mem.rebase(super::VU0_APPLY_MATRIX);
            for i in 0..3u32 {
                let src = tri.wrapping_add(0x10 * i);
                mem.write(src.wrapping_add(0xc), 1f32);
                call(mem, f, &[t.wrapping_add(0x10 * i), m, src]);
            }
        }
        for i in 0..3u32 {
            let p = t.wrapping_add(0x10 * i);
            if yi_tri_contains_xz(mem, rect, p) != 0
                || yi_tri_contains_xz(mem, rect.wrapping_add(0x20), p) != 0
            {
                return 1;
            }
        }
        for i in 0..4u32 {
            if yi_tri_contains_xz(mem, t, rect.wrapping_add(0x10 * i)) != 0 {
                return 1;
            }
        }
        0
    }

    // ---- round 17 ----

    /// `yiSegIntersect2(a, b, c, d, out)` (`0x0091cd00`): where the 2D
    /// segments `a..b` and `c..d` cross. `D = (b-a).y (d-c).x - (b-a).x
    /// (d-c).y`; parallel within the `.rdata` epsilon (`|D| <= 1e-5`) is 0.
    /// `t` (along `c..d`) and `u` (along `a..b`) must lie in `0..=1`; then
    /// `out = a (1-u) + b u` (x, y) and 1. Each difference and quotient is
    /// stored as `f32` where the original stores it (`D`'s first product
    /// as `f64`).
    pub fn yi_seg_intersect2<M2: Mem>(mem: &mut M2, p: [u32; 4], out: u32) -> u32 {
        let [a, b, c, d] = p;
        let ld = |mem: &M2, at: u32| x87::load(mem.read(at));
        let st = |x: f64| x87::load(x87::store(x));
        let dx = st(ld(mem, b) - ld(mem, a));
        let dy = st(ld(mem, b.wrapping_add(4)) - ld(mem, a.wrapping_add(4)));
        let ex = st(ld(mem, d) - ld(mem, c));
        let ey = st(ld(mem, d.wrapping_add(4)) - ld(mem, c.wrapping_add(4)));
        let pp = ex * dy;
        let q = ey * dx;
        let dd = st(pp - q);
        let eps = ld(mem, mem.rebase(super::SEG_EPSILON[0]));
        let neg = ld(mem, mem.rebase(super::SEG_EPSILON[1]));
        if dd <= eps && dd >= neg || dd.is_nan() {
            return 0;
        }
        let r = dx * (ld(mem, c.wrapping_add(4)) - ld(mem, a.wrapping_add(4)));
        let s = dy * (ld(mem, c) - ld(mem, a));
        let t = st((r - s) / dd);
        if t < 0.0 || t.is_nan() || 1.0 < t {
            return 0;
        }
        let uu = ex * (ld(mem, a.wrapping_add(4)) - ld(mem, c.wrapping_add(4)));
        let n = uu - (ld(mem, a) - ld(mem, c)) * ey;
        let u = st(n / st(q - pp));
        if u < 0.0 || u.is_nan() || 1.0 < u {
            return 0;
        }
        let x = ld(mem, b) * u + ld(mem, a) * (1.0 - u);
        mem.write(out, x87::store(x));
        let y = ld(mem, b.wrapping_add(4)) * u + (1.0 - u) * ld(mem, a.wrapping_add(4));
        mem.write(out.wrapping_add(4), x87::store(y));
        1
    }

    // ---- round 19 ----

    /// `yiGetFogColor(out)` (`0x00908980`): four ints. With render status
    /// bit 2 the fog colour bytes are clamped through the colour scale and
    /// offset as in [`yi_get_back_color`], else the raw bytes; then the
    /// first three are replaced by the PC renderer's colour times 255.0
    /// (`graphicGetGlobalAmbientColor` with bit 2, else
    /// `graphicGetFogColor`, into the original's locals at `work`, 12
    /// bytes), each product kept at the x87's precision. Returns the last
    /// conversion.
    pub fn yi_get_fog_color<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        out: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{Reg, ftol};
        let at = mem.rebase(super::fog::COLOUR);
        let c: [u8; 4] = mem.read(at);
        let status = RENDER_STATUS.get(mem);
        let top: f64 = mem.read(mem.rebase(super::fog::BYTE_MAX));
        if status & 4 == 0 {
            mem.write(out, c.map(u32::from));
        } else {
            let top_f = state::BACK_MAX_F32.get(mem);
            let scale = state::COLOR_SCALE.get(mem);
            let offset = COLOR_OFFSET.get(mem);
            let mut w = [0u32; 4];
            for i in 0..4 {
                let x = Reg::load(scale[i]).mul_m32(f32::from(c[i])).store();
                let mut v = Reg::load(x).add_m32(f32::from_bits(offset[i])).store();
                // below 0.0 (not unordered): +0.0
                if f64::from(v) < 0.0 {
                    v = 0.0;
                }
                // above 255.0 (not equal, not unordered): 255.0f
                if f64::from(v) > top {
                    v = top_f;
                }
                w[i] = ftol(mem, f64::from(v));
            }
            mem.write(out, w);
        }
        let f = if status & 4 == 0 {
            super::fog::GET_COLOUR
        } else {
            super::fog::GET_AMBIENT
        };
        let f = mem.rebase(f);
        call(mem, f, &[work, work.wrapping_add(4), work.wrapping_add(8)]);
        let mut last = 0;
        for i in 0..3u32 {
            let v = x87::load(mem.read(work.wrapping_add(4 * i)));
            last = ftol(mem, v * top);
            mem.write(out.wrapping_add(4 * i), last);
        }
        last
    }

    /// The fog distances' unit: scale factor times the two `.rdata`
    /// factors, at the x87's precision.
    fn fog_unit(mem: &impl Mem) -> f64 {
        let sf = x87::load(mem.read(mem.rebase(super::fog::SCALE)));
        let c1: f64 = mem.read(mem.rebase(super::fog::UNIT[0]));
        let c2: f64 = mem.read(mem.rebase(super::fog::UNIT[1]));
        sf * c1 * c2
    }

    /// An `int` loaded with `fild` and stored as `f32`, divided by 255.0
    /// and stored again: a colour channel for the PC renderer.
    fn byte_channel(mem: &impl Mem, v: u32) -> u32 {
        let top: f64 = mem.read(mem.rebase(super::fog::BYTE_MAX));
        #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // fild
        let f = f64::from((v as i32) as f32);
        x87::store(f / top)
    }

    /// `yiGetFogParam(near, far, limit)` (`0x00908bb0`): far and near are
    /// the stored distances times the unit ([`fog_unit`]), limit copied;
    /// on map 0xe0 the PC renderer's near, far and limit (times 255.0)
    /// replace them. Written far, near, limit.
    pub fn yi_get_fog_param<M: Mem>(
        mem: &mut M,
        out: [u32; 3],
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) {
        let [near, far, limit] = out;
        let u = fog_unit(mem);
        let f = x87::load(mem.read(mem.rebase(super::fog::FAR)));
        mem.write(far, x87::store(u * f));
        let n = x87::load(mem.read(mem.rebase(super::fog::NEAR)));
        mem.write(near, x87::store(u * n));
        let l = x87::copy(mem.read(mem.rebase(super::fog::LIMIT)));
        mem.write(limit, l);
        if super::CURRENT_MAP.get(mem) != super::fog::PC_FOG_MAP {
            return;
        }
        let g = mem.rebase(super::fog::GET_NEAR);
        let v = fcall(mem, g, crate::camera::FArgs::Stack(&[]));
        mem.write(near, x87::store(v));
        let g = mem.rebase(super::fog::GET_FAR);
        let v = fcall(mem, g, crate::camera::FArgs::Stack(&[]));
        mem.write(far, x87::store(v));
        let g = mem.rebase(super::fog::GET_LIMIT);
        let v = fcall(mem, g, crate::camera::FArgs::Stack(&[]));
        let top: f64 = mem.read(mem.rebase(super::fog::BYTE_MAX));
        mem.write(limit, x87::store(v * top));
    }

    /// `yiSetBackColor(c)` (`0x0090a040`): the low bytes of four ints are
    /// the background colour; unless the current magic is 0x2c2 the PC
    /// renderer gets them as floats (`r, g, b / 255.0`, `a` times the
    /// `.rdata` factor; each int stored as `f32` first).
    pub fn yi_set_back_color<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) {
        let v: [u32; 4] = mem.read(c);
        #[allow(clippy::cast_possible_truncation)] // the low bytes
        let b = v.map(|x| x as u8);
        state::BACK_COLOR.set(mem, b);
        let id = mem.rebase(super::fog::MAGIC_GET_ID);
        if call(mem, id, &[]) == super::fog::MAGIC_KEEP_BACK {
            return;
        }
        let v: [u32; 4] = mem.read(c);
        let k: f64 = mem.read(mem.rebase(super::fog::ALPHA_SCALE));
        #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // fild
        let a = x87::store(f64::from((v[3] as i32) as f32) * k);
        let rgb = [0, 1, 2].map(|i| byte_channel(mem, v[i]));
        let f = mem.rebase(super::fog::SET_BACKGROUND);
        call(mem, f, &[rgb[0], rgb[1], rgb[2], a]);
    }

    /// `yiSetFogColor(c)` (`0x0090a2c0`): the low bytes of four ints are
    /// the fog colour; on map 0xe0 the PC renderer gets `r, g, b / 255.0`.
    pub fn yi_set_fog_color<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) {
        let v: [u32; 4] = mem.read(c);
        #[allow(clippy::cast_possible_truncation)] // the low bytes
        let b = v.map(|x| x as u8);
        let at = mem.rebase(super::fog::COLOUR);
        mem.write(at, b);
        if super::CURRENT_MAP.get(mem) != super::fog::PC_FOG_MAP {
            return;
        }
        let rgb = [0, 1, 2].map(|i| byte_channel(mem, v[i]));
        let f = mem.rebase(super::fog::SET_COLOUR);
        call(mem, f, &rgb);
    }

    /// `yiSetFogParam(near, far, limit)` (`0x0090a350`): near and far over
    /// the unit ([`fog_unit`]) and limit are stored; on map 0xe0 the PC
    /// renderer gets near and far times its distance factor, `(1 -
    /// limit / 255) * ` its limit factor, and 1.0 for `0x00643180`.
    pub fn yi_set_fog_param<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut C,
    ) {
        let [near, far, limit] = args;
        let u = fog_unit(mem);
        let far = x87::store(x87::load(far) / u);
        let near = x87::store(x87::load(near) / u);
        let at = mem.rebase(super::fog::NEAR);
        mem.write(at, near);
        let at = mem.rebase(super::fog::FAR);
        mem.write(at, far);
        let at = mem.rebase(super::fog::LIMIT);
        mem.write(at, x87::copy(limit));
        if super::CURRENT_MAP.get(mem) != super::fog::PC_FOG_MAP {
            return;
        }
        let d: f64 = mem.read(mem.rebase(super::fog::PC_DISTANCE));
        let f = mem.rebase(super::fog::SET_NEAR);
        call(mem, f, &[x87::store(x87::load(near) * d)]);
        let f = mem.rebase(super::fog::SET_FAR);
        call(mem, f, &[x87::store(x87::load(far) * d)]);
        let top: f64 = mem.read(mem.rebase(super::fog::BYTE_MAX));
        let k: f64 = mem.read(mem.rebase(super::fog::PC_LIMIT));
        let l = x87::store((1.0 - x87::load(limit) / top) * k);
        let f = mem.rebase(super::fog::SET_LIMIT);
        call(mem, f, &[l]);
        let f = mem.rebase(super::fog::SET_FOG_643180);
        call(mem, f, &[1.0f32.to_bits()]);
    }

    /// `yiDisableHamonEff()` (`0x00908450`): clears render status bit 24
    /// and the distortion word, turns the PC renderer's distortion off;
    /// returns 0.
    pub fn yi_disable_hamon_eff<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s & 0xfeff_ffff);
        let at = mem.rebase(super::fog::HAMON);
        mem.write(at, 0u32);
        let f = mem.rebase(super::fog::SET_DISTORTION);
        call(mem, f, &[0]);
        0
    }

    /// `Sg_MemSetFree(block)` (`0x0083bdb0`, name ours; from
    /// `Sg_MemCreateFree`, `Sg_MemMoveWork` and `0x0083b720`): marks the
    /// block (its header) free: `+8` 0, `+0xc` down to its heap bits
    /// (27..28); that heap (`0x01301968`) gets its freed byte set and its
    /// lowest freed block (`+0x14`, unsigned) lowered to the block.
    /// Returns three times the heap index (the original's `eax`).
    pub fn sg_mem_set_free(mem: &mut impl Mem, block: u32) -> u32 {
        let w = mem.read::<u32>(block.wrapping_add(0xc));
        let heap = (w >> 27) & 3;
        mem.write(block.wrapping_add(8), 0u32);
        mem.write(block.wrapping_add(0xc), w & 0x1800_0000);
        let h = mem.rebase(super::MEM_HEAPS) + 0x18 * heap;
        mem.write(h, 1u8);
        if block < mem.read::<u32>(h + 0x14) {
            mem.write(h + 0x14, block);
        }
        heap * 3
    }
}
