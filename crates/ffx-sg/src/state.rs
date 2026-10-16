//! The main loop's switches and small accessors: what is calculated and
//! drawn each frame (`Sg_Enable*`), pause, frame pacing, focus, filters,
//! the accumulation buffer, capture, the map selection, the memory
//! manager's options, resources, the debug menu's cursor.
//!
//! Most of these are one global each; the originals that return the old
//! value of a switch sign-extend it (`movsx`).

/// The originals over game memory (`ffx_bridge::Mem`).
#[allow(clippy::many_single_char_names)] // colour channels
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::lo8;

    // The main loop's switches (`i8`, 0 or 1 where an `Sg_Enable*` sets them).
    /// `Sg_GetViewerMode`: the viewer build's mode (`Sg_GetUserName2` then names "sugimoto").
    pub const VIEWER_MODE: Global<u8> = Global::new(0x012f_b8f8);
    /// Effect matrices on (`Sg_EnableEffectMatirx` toggles it).
    pub const EFFECT_MATRIX: Global<u8> = Global::new(0x012f_b8f9);
    /// `Sg_GetDispMode`.
    pub const DISP_MODE: Global<u8> = Global::new(0x012f_b8fb);
    /// Characters are calculated (`Sg_EnableCalcChr`).
    pub const CALC_CHR: Global<u8> = Global::new(0x012f_b8fe);
    /// Characters are drawn (`Sg_EnableDrawChr`).
    pub const DRAW_CHR: Global<u8> = Global::new(0x012f_b8ff);
    /// The double buffer's draw context, returned as an address by `Sg_GetDBuffDC`.
    pub const DBUFF: Global<u8> = Global::new(0x012f_b900);
    /// The camera is calculated (`Sg_EnableCalcCam`).
    pub const CALC_CAM: Global<u8> = Global::new(0x012f_bbb0);
    /// The map is calculated (`Sg_EnableCalcMap`).
    pub const CALC_MAP: Global<u8> = Global::new(0x012f_bbb1);
    /// The map is drawn (`Sg_EnableDrawMap`).
    pub const DRAW_MAP: Global<u8> = Global::new(0x012f_bbb2);
    /// The GUI is drawn (`Sg_EnableDrawGui`).
    pub const DRAW_GUI: Global<u8> = Global::new(0x012f_bbb3);
    /// The menu is drawn (`Sg_EnableDrawMenu`).
    pub const DRAW_MENU: Global<u8> = Global::new(0x012f_bbb4);
    /// `Sg_GetMapAlphaFast`.
    pub const MAP_ALPHA_FAST: Global<u8> = Global::new(0x012f_bbb6);
    /// Full-screen anti-aliasing of the `Tk` layer (`Sg_EnableTkAnti`).
    pub const TK_ANTI: Global<u8> = Global::new(0x012f_bbb7);
    /// `Sg_EffectRoutineEnable`.
    pub const EFFECT_ROUTINE: Global<u8> = Global::new(0x012f_bbb8);
    /// `Sg_SetthDispEnable` (0 or 1).
    pub const TH_DISP: Global<u8> = Global::new(0x012f_bbb9);
    /// `Sg_GuiSetDrawChr`.
    pub const GUI_DRAW_CHR: Global<u8> = Global::new(0x012f_bbba);
    /// The game is paused (`Sg_SetPause`).
    pub const PAUSE: Global<u8> = Global::new(0x012f_bbbb);
    /// Keep the frame rate (`Sg_SetKeepFps`).
    pub const KEEP_FPS: Global<u8> = Global::new(0x012f_bc34);
    /// The draw context `Sg_GetDBuffDC` stores through its argument.
    pub const DBUFF_DC: Global<u32> = Global::new(0x023c_bbf0);

    /// The packet buffer (`Sg_PacketGetPtr`).
    pub const PACKET: Global<u32> = Global::new(0x00c4_98dc);
    /// The user name string (`Sg_GetUserName`).
    pub const USER_NAME: Global<u32> = Global::new(0x00c4_94a8);
    /// The printf hook (`SetPrintfFunc`).
    pub const PRINTF_FUNC: Global<u32> = Global::new(0x00c4_dfb0);
    /// The player character (`Sg_SetPlayer`, `Sg_GetPlayer`).
    pub const PLAYER: Global<u32> = Global::new(0x0130_0788);
    /// `Sg_GetIgnoreID`.
    pub const IGNORE_ID: Global<u32> = Global::new(0x023c_44b4);

    // Frame pacing.
    /// Frames run per game frame (`Sg_SetExecFrames`, 0..=99).
    pub const EXEC_FRAMES: Global<u8> = Global::new(0x00c4_96d4);
    /// The L/R button mode (0 or 1).
    pub const LR_BUTTON_MODE: Global<u8> = Global::new(0x0130_07b0);
    /// `Sg_SetExecSpecialSpeed`.
    pub const EXEC_SPECIAL_SPEED: Global<u8> = Global::new(0x0130_07b2);
    /// Set when the debug pad stepped the frame count below 0 (`Sg_GetExecStep`).
    pub const EXEC_STEP: Global<u8> = Global::new(0x0130_07b3);

    // Filters, focus, blur and the accumulation buffer.
    /// `Sg_AccGetAlpha` (`i16`; `Sg_AccSetAlpha` sets it).
    pub const ACC_ALPHA: Global<u16> = Global::new(0x0130_070e);
    /// `Sg_GetBlur` (`i16`).
    pub const BLUR: Global<u16> = Global::new(0x0130_071e);
    /// The flash (`Sg_Flash`; `Sg_FlashReset` clears it).
    pub const FLASH: Global<u16> = Global::new(0x0130_072c);
    /// The filter (`Sg_FilterAdd`; `Sg_FilterReset` clears it).
    pub const FILTER: Global<u16> = Global::new(0x0130_073c);
    /// Focus values (`Sg_GetFocus(i)`), words.
    pub const FOCUS: Global<u32> = Global::new(0x0130_0758);
    /// Focus on (bit 0, `Sg_EnFocus`).
    pub const FOCUS_ENABLE: Global<u32> = Global::new(0x0130_076c);
    /// The accumulation buffer's map layer (`Sg_AccSetDrawMap`).
    pub const ACC_DRAW_MAP: Global<u8> = Global::new(0x0130_0776);
    /// The accumulation buffer's background (`Sg_AccEnableBG`).
    pub const ACC_BG: Global<u8> = Global::new(0x0130_0777);
    /// The accumulation buffer's character layer (`Sg_AccSetDrawChr`).
    pub const ACC_DRAW_CHR: Global<u8> = Global::new(0x0130_0778);
    /// `Sg_DrawSetClearIgnore`.
    pub const CLEAR_IGNORE: Global<u8> = Global::new(0x012f_fae1);
    /// `Sg_GuiSetTextureAnim`.
    pub const GUI_TEXTURE_ANIM: Global<u8> = Global::new(0x0130_07eb);
    /// `Sg_SetMovieGomiAvoid`.
    pub const MOVIE_GOMI_AVOID: Global<u8> = Global::new(0x0130_08be);

    // Capture and the `Tk` layer's anti-aliasing.
    /// `Sg_PcReadGetLen`: the length of the last PC file read.
    pub const PC_READ_LEN: Global<u32> = Global::new(0x0130_1818);
    /// The capture selection (`Sg_CaptureSetSelect`).
    pub const CAPTURE_SELECT: Global<u8> = Global::new(0x0130_181c);
    /// Anti-aliasing enabled (`Sg_EnableTkAnti_0083add0`).
    pub const TK_ANTI_ENABLE: Global<u8> = Global::new(0x0130_181e);
    /// Capture is running (`Sg_CaptureGetGuiHide`).
    pub const CAPTURE_ON: Global<u32> = Global::new(0x0130_1804);
    /// The capture hides the GUI (`Sg_CaptureGetGuiHide`).
    pub const CAPTURE_GUI_HIDE: Global<u32> = Global::new(0x0130_1940);
    /// `Sg_SetTkAntiXYB(x, y, b)`: three words.
    pub const TK_ANTI_XYB: Global<[u32; 3]> = Global::new(0x0130_194c);

    // The memory manager's options.
    /// `Sg_MemSetErrorIgnore`.
    pub const MEM_ERROR_IGNORE: Global<u32> = Global::new(0x0130_1958);
    /// The heap's start (`Sg_MemSetMoveHeapStartAddr` falls back to it).
    pub const MEM_HEAP_START: Global<u32> = Global::new(0x0130_1974);
    /// `Sg_MemSetCheck`.
    pub const MEM_CHECK: Global<u8> = Global::new(0x0130_19b4);
    /// `Sg_MemSetFast`.
    pub const MEM_FAST: Global<u8> = Global::new(0x0130_19b5);
    /// Where moved blocks start (`Sg_MemSetMoveHeapStartAddr`).
    pub const MEM_MOVE_START: Global<u32> = Global::new(0x0130_19b8);

    // Map ids and resources.
    /// The specular texture's record (`Sg_TextureGetSpecular`).
    pub const TEXTURE_SPECULAR: Global<u8> = Global::new(0x0130_1a60);
    /// `Sg_IdGetZoom`.
    pub const ID_ZOOM: Global<f32> = Global::new(0x0130_1a90);
    /// `Sg_IdGetPtr`.
    pub const ID_PTR: Global<u32> = Global::new(0x0130_1a94);
    /// The effect table entries, 4 bytes a set (`g_ChrIdEffectTable`).
    pub const ID_EFFECT_TABLE: Global<u8> = Global::new(0x0130_1af8);
    /// The effect table set (`g_ChrIdEffectTableIndex`, `i8`).
    pub const ID_EFFECT_TABLE_INDEX: Global<u8> = Global::new(0x0130_1b00);
    /// `Sg_IdExistShade`.
    pub const ID_EXIST_SHADE: Global<u8> = Global::new(0x0130_34b8);
    /// `Sg_MapSetEnableShade`.
    pub const MAP_ENABLE_SHADE: Global<u8> = Global::new(0x0130_34b9);
    /// The id collision test switch (`Sg_Id_coltest` toggles it).
    pub const ID_COLTEST: Global<u32> = Global::new(0x0130_34b0);
    /// The id drawing mode, 0..=3 (`Sg_SwitchDrawId` cycles it).
    pub const ID_DRAW: Global<i32> = Global::new(0x0130_34b4);
    /// The map selection of the debug map menu (`Sg_MapSetDirSelect`).
    pub const MAP_DIR_SELECT: Global<u32> = Global::new(0x00c4_df50);
    /// Map names, 64 bytes each, by selection (`Sg_MapGetName(1)`).
    pub const MAP_NAMES: Global<u8> = Global::new(0x0130_34c0);
    /// Map names of the other list (`Sg_MapGetName(0)`).
    pub const MAP_NAMES_ALT: Global<u8> = Global::new(0x0130_3580);
    /// Resource table: 4-byte entries, the `i16` at `+2` an offset into
    /// [`RESOURCE_STR`] (`Sg_GetResourceSTR`).
    pub const RESOURCE_TABLE: Global<u8> = Global::new(0x00c4_df00);
    /// The resource strings.
    pub const RESOURCE_STR: Global<u8> = Global::new(0x0130_1bf0);
    /// The current vertex and normal buffers (`sg_setvtex_and_normal`).
    pub const VTEX: Global<u32> = Global::new(0x0130_3690);
    /// See [`VTEX`].
    pub const NORM: Global<u32> = Global::new(0x0130_3694);
    /// The y position of the debug menu's next line (`sg_GetY`).
    pub const MENU_Y: Global<u32> = Global::new(0x0132_412c);
    /// `Sg_MouseGetMove`.
    pub const MOUSE_MOVE: Global<u32> = Global::new(0x0132_525c);
    /// `g_FFXIsDebug` (`isDebugMode`).
    pub const IS_DEBUG: Global<u8> = Global::new(0x0133_c910);
    /// `enableGameControlTextureAnimation`.
    pub const GAME_CONTROL_TEXTURE_ANIM: Global<u32> = Global::new(0x00c4_9720);
    /// `Ch_GuiSetMouthTh`'s value (`g_ChrMouthTh`, `i16`).
    pub const CHR_MOUTH_TH: Global<u16> = Global::new(0x00c4_9718);
    /// `g_ChrShadeByLight`.
    pub const CHR_SHADE_BY_LIGHT: Global<u8> = Global::new(0x00c4_96ea);
    /// The CD reader's buffer (`get_read_buffer`).
    pub const READ_BUFFER: Global<u32> = Global::new(0x0112_8ac8);
    /// Show read errors (`read_err_disp_set`).
    pub const READ_ERR_DISP: Global<u8> = Global::new(0x0112_8a87);
    /// The HDD error (`set_hdd_err`).
    pub const HDD_ERR: Global<u8> = Global::new(0x0112_a50a);
    /// The function name string of the assertion messages (`Sg_GetFuncName`).
    pub const FUNC_NAME: Global<u8> = Global::new(0x00b8_fbeb);
    /// The scratchpad selection (`Sg_GetSpadPtr`).
    pub const SPAD_SELECT: Global<u32> = Global::new(0x0130_3344);
    /// The scratchpad used while [`SPAD_SELECT`] is set (`SendSpadToMFIFO`).
    pub const SPAD_A: Global<u8> = Global::new(0x0230_36e0);
    /// The scratchpad used otherwise.
    pub const SPAD_B: Global<u8> = Global::new(0x0230_16e0);

    /// A switch, sign-extended.
    fn get(mem: &impl Mem, g: Global<u8>) -> i32 {
        i32::from(g.get(mem).cast_signed())
    }

    /// Sets a switch to bit 0 of `v`; the old value, sign-extended.
    fn enable(mem: &mut impl Mem, g: Global<u8>, v: u32) -> i32 {
        let old = get(mem, g);
        g.set(mem, lo8(v) & 1);
        old
    }

    /// Flips a byte switch: `(v - 1) & 1`.
    fn toggle(mem: &mut impl Mem, g: Global<u8>) {
        let v = g.get(mem).wrapping_sub(1) & 1;
        g.set(mem, v);
    }

    /// Element `i` of a table of `size`-byte entries at `g`.
    fn elem<T>(mem: &impl Mem, g: Global<T>, i: u32, size: u32) -> u32 {
        g.ptr(mem).addr.wrapping_add(i.wrapping_mul(size))
    }

    /// `Sg_EnableCalcCam(on)` (`0x008203a0`).
    pub fn enable_calc_cam(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, CALC_CAM, on)
    }
    /// `Sg_EnableDrawChr(on)` (`0x008203c0`).
    pub fn enable_draw_chr(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, DRAW_CHR, on)
    }
    /// `Sg_EnableCalcChr(on)` (`0x008203e0`).
    pub fn enable_calc_chr(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, CALC_CHR, on)
    }
    /// `Sg_EnableDrawMap(on)` (`0x00820400`).
    pub fn enable_draw_map(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, DRAW_MAP, on)
    }
    /// `Sg_EnableDrawGui(on)` (`0x00820420`).
    pub fn enable_draw_gui(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, DRAW_GUI, on)
    }
    /// `Sg_EnableCalcMap(on)` (`0x00820440`).
    pub fn enable_calc_map(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, CALC_MAP, on)
    }
    /// `Sg_EnableDrawMenu(on)` (`0x00820460`).
    pub fn enable_draw_menu(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, DRAW_MENU, on)
    }
    /// `Sg_EnableTkAnti(on)` (`0x00820500`).
    pub fn enable_tk_anti(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, TK_ANTI, on)
    }
    /// `Sg_EnableTkAnti_0083add0(on)` (`0x0083add0`).
    pub fn set_enable_tk_anti(mem: &mut impl Mem, on: u32) -> i32 {
        enable(mem, TK_ANTI_ENABLE, on)
    }

    /// `Sg_EnableGetCalcCam()` (`0x00820490`).
    pub fn get_calc_cam(mem: &impl Mem) -> i32 {
        get(mem, CALC_CAM)
    }
    /// `Sg_GetDispMode()` (`0x00820550`).
    pub fn get_disp_mode(mem: &impl Mem) -> i32 {
        get(mem, DISP_MODE)
    }
    /// `Sg_EnableGetTkAnti()` (`0x00820570`).
    pub fn get_tk_anti(mem: &impl Mem) -> i32 {
        get(mem, TK_ANTI)
    }
    /// `Sg_GetKeepFps()` (`0x008205a0`).
    pub fn get_keep_fps(mem: &impl Mem) -> i32 {
        get(mem, KEEP_FPS)
    }
    /// `Sg_GetMapAlphaFast()` (`0x008205d0`).
    pub fn get_map_alpha_fast(mem: &impl Mem) -> i32 {
        get(mem, MAP_ALPHA_FAST)
    }
    /// `Sg_GetPause()` (`0x008205e0`).
    pub fn get_pause(mem: &impl Mem) -> i32 {
        get(mem, PAUSE)
    }
    /// `Sg_GetViewerMode()` (`0x00820640`).
    pub fn get_viewer_mode(mem: &impl Mem) -> i32 {
        get(mem, VIEWER_MODE)
    }
    /// `Sg_GetthDispEnable()` (`0x00820650`).
    pub fn get_th_disp_enable(mem: &impl Mem) -> i32 {
        get(mem, TH_DISP)
    }
    /// `Sg_GuiGetDrawChr()` (`0x00820660`).
    pub fn gui_get_draw_chr(mem: &impl Mem) -> i32 {
        get(mem, GUI_DRAW_CHR)
    }
    /// `Sg_GetExecFrames()` (`0x0082d810`).
    pub fn get_exec_frames(mem: &impl Mem) -> i32 {
        get(mem, EXEC_FRAMES)
    }
    /// `Sg_GetExecStep()` (`0x0082d820`).
    pub fn get_exec_step(mem: &impl Mem) -> i32 {
        get(mem, EXEC_STEP)
    }
    /// `Sg_GetLRButtonMode()` (`0x0082d840`).
    pub fn get_lr_button_mode(mem: &impl Mem) -> i32 {
        get(mem, LR_BUTTON_MODE)
    }
    /// `Sg_GuiGetTextureAnim()` (`0x00831930`).
    pub fn gui_get_texture_anim(mem: &impl Mem) -> i32 {
        get(mem, GUI_TEXTURE_ANIM)
    }
    /// `Sg_CaptureGetSelect()` (`0x0083a640`).
    pub fn capture_get_select(mem: &impl Mem) -> i32 {
        get(mem, CAPTURE_SELECT)
    }
    /// `Sg_IdExistShade()` (`0x00844bb0`).
    pub fn id_exist_shade(mem: &impl Mem) -> i32 {
        get(mem, ID_EXIST_SHADE)
    }
    /// `Sg_MapGetEnableShade()` (`0x00844ce0`).
    pub fn map_get_enable_shade(mem: &impl Mem) -> i32 {
        get(mem, MAP_ENABLE_SHADE)
    }
    /// `Sg_AccGetAlpha()` (`0x0082bcc0`).
    pub fn acc_get_alpha(mem: &impl Mem) -> i32 {
        i32::from(ACC_ALPHA.get(mem).cast_signed())
    }
    /// `Sg_GetBlur()` (`0x0082cd20`).
    pub fn get_blur(mem: &impl Mem) -> i32 {
        i32::from(BLUR.get(mem).cast_signed())
    }
    /// `Ch_GuiGetMouthTh()` (`0x00834070`).
    pub fn chr_gui_get_mouth_th(mem: &impl Mem) -> i32 {
        i32::from(CHR_MOUTH_TH.get(mem).cast_signed())
    }

    /// `isDebugMode()` (`0x00822550`): `g_FFXIsDebug` in `AL` (the rest
    /// of `EAX` is the caller's).
    pub fn is_debug_mode(mem: &impl Mem) -> u8 {
        IS_DEBUG.get(mem)
    }

    // Byte setters (`mov al, [ebp+8]; mov [g], al`).
    /// `Sg_EffectRoutineEnable(v)` (`0x00820380`).
    pub fn effect_routine_enable(mem: &mut impl Mem, v: u32) {
        EFFECT_ROUTINE.set(mem, lo8(v));
    }
    /// `Sg_GuiSetDrawChr(v)` (`0x00820670`).
    pub fn gui_set_draw_chr(mem: &mut impl Mem, v: u32) {
        GUI_DRAW_CHR.set(mem, lo8(v));
    }
    /// `Sg_SetPause(v)` (`0x00821cb0`).
    pub fn set_pause(mem: &mut impl Mem, v: u32) {
        PAUSE.set(mem, lo8(v));
    }
    /// `Sg_SetthDispEnable(v)` (`0x00821d50`): bit 0.
    pub fn set_th_disp_enable(mem: &mut impl Mem, v: u32) {
        TH_DISP.set(mem, lo8(v) & 1);
    }
    /// `Sg_AccSetDrawMap(v)` (`0x0082bcb0`).
    pub fn acc_set_draw_map(mem: &mut impl Mem, v: u32) {
        ACC_DRAW_MAP.set(mem, lo8(v));
    }
    /// `Sg_AccSetDrawChr(v)` (`0x0082be00`).
    pub fn acc_set_draw_chr(mem: &mut impl Mem, v: u32) {
        ACC_DRAW_CHR.set(mem, lo8(v));
    }
    /// `Sg_AccEnableBG(v)` (`0x0082be10`).
    pub fn acc_enable_bg(mem: &mut impl Mem, v: u32) {
        ACC_BG.set(mem, lo8(v));
    }
    /// `Sg_DrawSetClearIgnore(v)` (`0x0082c7a0`).
    pub fn draw_set_clear_ignore(mem: &mut impl Mem, v: u32) {
        CLEAR_IGNORE.set(mem, lo8(v));
    }
    /// `Sg_SetExecSpecialSpeed(v)` (`0x0082da90`).
    pub fn set_exec_special_speed(mem: &mut impl Mem, v: u32) {
        EXEC_SPECIAL_SPEED.set(mem, lo8(v));
    }
    /// `Sg_SetLRButtonMode(v)` (`0x0082daa0`): bit 0.
    pub fn set_lr_button_mode(mem: &mut impl Mem, v: u32) {
        LR_BUTTON_MODE.set(mem, lo8(v) & 1);
    }
    /// `Sg_SetMovieGomiAvoid(v)` (`0x00830150`).
    pub fn set_movie_gomi_avoid(mem: &mut impl Mem, v: u32) {
        MOVIE_GOMI_AVOID.set(mem, lo8(v));
    }
    /// `Sg_GuiSetTextureAnim(v)` (`0x00831940`).
    pub fn gui_set_texture_anim(mem: &mut impl Mem, v: u32) {
        GUI_TEXTURE_ANIM.set(mem, lo8(v));
    }
    /// `Ch_SetShadeByLight(v)` (`0x00831d00`).
    pub fn chr_set_shade_by_light(mem: &mut impl Mem, v: u32) {
        CHR_SHADE_BY_LIGHT.set(mem, lo8(v));
    }
    /// `Sg_CaptureSetSelect(v)` (`0x0083a680`).
    pub fn capture_set_select(mem: &mut impl Mem, v: u32) {
        CAPTURE_SELECT.set(mem, lo8(v));
    }
    /// `Sg_MemSetCheck(v)` (`0x0083c4d0`).
    pub fn mem_set_check(mem: &mut impl Mem, v: u32) {
        MEM_CHECK.set(mem, lo8(v));
    }
    /// `Sg_MemSetFast(v)` (`0x0083c520`).
    pub fn mem_set_fast(mem: &mut impl Mem, v: u32) {
        MEM_FAST.set(mem, lo8(v));
    }
    /// `Sg_IdSetEffectTableIndex(v)` (`0x0083e9d0`).
    pub fn id_set_effect_table_index(mem: &mut impl Mem, v: u32) {
        ID_EFFECT_TABLE_INDEX.set(mem, lo8(v));
    }
    /// `Sg_MapSetEnableShade(v)` (`0x00844fa0`).
    pub fn map_set_enable_shade(mem: &mut impl Mem, v: u32) {
        MAP_ENABLE_SHADE.set(mem, lo8(v));
    }
    /// `read_err_disp_set(v)` (`0x0076e5e0`).
    pub fn read_err_disp_set(mem: &mut impl Mem, v: u32) {
        READ_ERR_DISP.set(mem, lo8(v));
    }
    /// `set_hdd_err(v)` (`0x00779f70`).
    pub fn set_hdd_err(mem: &mut impl Mem, v: u32) {
        HDD_ERR.set(mem, lo8(v));
    }

    // Word setters: the original leaves the stored value in `EAX`.
    /// `enableGameControlTextureAnimation(v)` (`0x00836710`).
    pub fn enable_game_control_texture_animation(mem: &mut impl Mem, v: u32) -> u32 {
        GAME_CONTROL_TEXTURE_ANIM.set(mem, v);
        v
    }
    /// `Sg_MemSetErrorIgnore(v)` (`0x0083c510`).
    pub fn mem_set_error_ignore(mem: &mut impl Mem, v: u32) -> u32 {
        MEM_ERROR_IGNORE.set(mem, v);
        v
    }
    /// `Sg_MapSetDirSelect(v)` (`0x00844f90`).
    pub fn map_set_dir_select(mem: &mut impl Mem, v: u32) -> u32 {
        MAP_DIR_SELECT.set(mem, v);
        v
    }
    /// `Sg_SetTkAntiXYB(x, y, b)` (`0x0083b110`): `EAX` ends as `b`.
    pub fn set_tk_anti_xyb(mem: &mut impl Mem, x: u32, y: u32, b: u32) -> u32 {
        TK_ANTI_XYB.set(mem, [x, y, b]);
        b
    }
    /// `sg_setvtex_and_normal(vtex, norm)` (`0x00845ca0`): `EAX` ends as `norm`.
    pub fn set_vtex_and_normal(mem: &mut impl Mem, vtex: u32, norm: u32) -> u32 {
        VTEX.set(mem, vtex);
        NORM.set(mem, norm);
        norm
    }

    // Word getters.
    /// `get_read_buffer()` (`0x0076dc50`).
    pub fn get_read_buffer(mem: &impl Mem) -> u32 {
        READ_BUFFER.get(mem)
    }
    /// `Sg_GetUserName()` (`0x00820610`).
    pub fn get_user_name(mem: &impl Mem) -> u32 {
        USER_NAME.get(mem)
    }
    /// `Sg_GetIgnoreID()` (`0x0082d830`).
    pub fn get_ignore_id(mem: &impl Mem) -> u32 {
        IGNORE_ID.get(mem)
    }
    /// `Sg_GetPlayer()` (`0x0082d860`).
    pub fn get_player(mem: &impl Mem) -> u32 {
        PLAYER.get(mem)
    }
    /// `Sg_PcReadGetLen()` (`0x0083ae70`).
    pub fn pc_read_get_len(mem: &impl Mem) -> u32 {
        PC_READ_LEN.get(mem)
    }
    /// `Sg_PacketGetPtr()` (`0x0083b650`).
    pub fn packet_get_ptr(mem: &impl Mem) -> u32 {
        PACKET.get(mem)
    }
    /// `Sg_IdGetPtr()` (`0x0083e970`).
    pub fn id_get_ptr(mem: &impl Mem) -> u32 {
        ID_PTR.get(mem)
    }
    /// `Sg_MapGetDirSelect()` (`0x00844cd0`).
    pub fn map_get_dir_select(mem: &impl Mem) -> u32 {
        MAP_DIR_SELECT.get(mem)
    }
    /// `sg_getnorm()` (`0x00845c50`).
    pub fn get_norm(mem: &impl Mem) -> u32 {
        NORM.get(mem)
    }
    /// `sg_getvtex()` (`0x00845c60`).
    pub fn get_vtex(mem: &impl Mem) -> u32 {
        VTEX.get(mem)
    }
    /// `sg_GetY()` (`0x00850830`).
    pub fn get_y(mem: &impl Mem) -> u32 {
        MENU_Y.get(mem)
    }
    /// `Sg_MouseGetMove()` (`0x00851910`).
    pub fn mouse_get_move(mem: &impl Mem) -> u32 {
        MOUSE_MOVE.get(mem)
    }

    // Addresses.
    /// `SendSpadToMFIFO()` (`0x00843200`): the first scratchpad.
    pub fn send_spad_to_mfifo(mem: &impl Mem) -> u32 {
        SPAD_A.ptr(mem).addr
    }
    /// `Sg_GetFuncName()` (`0x008456c0`).
    pub fn get_func_name(mem: &impl Mem) -> u32 {
        FUNC_NAME.ptr(mem).addr
    }
    /// `Sg_VramGetAddrZ()` (`0x0082ce80`): the depth buffer's GS address.
    #[must_use]
    pub const fn vram_get_addr_z() -> u32 {
        0x0008_4000
    }

    /// `Sg_GetDBuffDC(&dc)` (`0x00820530`): stores the draw context when
    /// `dc` is not null; returns the double buffer's address.
    pub fn get_dbuff_dc(mem: &mut impl Mem, dc: u32) -> u32 {
        if dc != 0 {
            let v = DBUFF_DC.get(mem);
            mem.write(dc, v);
        }
        DBUFF.ptr(mem).addr
    }

    /// `Sg_IdGetZoom()` (`0x0083e9a0`).
    pub fn id_get_zoom(mem: &impl Mem) -> f32 {
        ID_ZOOM.get(mem)
    }

    /// `Sg_TextureGetSpecular()` (`0x0083d3a0`): the specular texture's
    /// record.
    pub fn texture_get_specular(mem: &impl Mem) -> u32 {
        TEXTURE_SPECULAR.ptr(mem).addr
    }

    /// `Sg_GetSpadPtr()` (`0x00843220`).
    pub fn get_spad_ptr(mem: &impl Mem) -> u32 {
        if SPAD_SELECT.get(mem) == 0 {
            SPAD_B.ptr(mem).addr
        } else {
            SPAD_A.ptr(mem).addr
        }
    }

    /// `Sg_SetKeepFps(v)` (`0x00821ae0`): the old value, sign-extended.
    pub fn set_keep_fps(mem: &mut impl Mem, v: u32) -> i32 {
        let old = get(mem, KEEP_FPS);
        KEEP_FPS.set(mem, lo8(v));
        old
    }

    /// `Sg_EnFocus(on)` (`0x0082cb60`): bit 0; the old word.
    pub fn en_focus(mem: &mut impl Mem, on: u32) -> u32 {
        let old = FOCUS_ENABLE.get(mem);
        FOCUS_ENABLE.set(mem, on & 1);
        old
    }

    /// `Sg_GetFocusEnable()` (`0x0082cd40`).
    pub fn get_focus_enable(mem: &impl Mem) -> u32 {
        FOCUS_ENABLE.get(mem) & 1
    }

    /// `Sg_GetFocus(i)` (`0x0082cd30`).
    pub fn get_focus(mem: &impl Mem, i: u32) -> u32 {
        mem.read(elem(mem, FOCUS, i, 4))
    }

    /// `Sg_FilterReset()` (`0x0082cc70`): `EAX` ends 0.
    pub fn filter_reset(mem: &mut impl Mem) -> u32 {
        FILTER.set(mem, 0);
        0
    }

    /// `Sg_FlashReset()` (`0x0082cce0`): `EAX` ends 0.
    pub fn flash_reset(mem: &mut impl Mem) -> u32 {
        FLASH.set(mem, 0);
        0
    }

    /// `SetPrintfFunc(f)` (`0x008510c0`): the old hook.
    pub fn set_printf_func(mem: &mut impl Mem, f: u32) -> u32 {
        let old = PRINTF_FUNC.get(mem);
        PRINTF_FUNC.set(mem, f);
        old
    }

    /// `Sg_SetExecFrames(n)` (`0x0082da60`): `n` clamped to `0..=99`.
    pub fn set_exec_frames(mem: &mut impl Mem, n: i32) {
        EXEC_FRAMES.set(mem, lo8(n.clamp(0, 99).cast_unsigned()));
    }

    /// `Sg_EnableEffectMatirx()` (`0x00820480`): flips the switch.
    pub fn enable_effect_matrix(mem: &mut impl Mem) {
        toggle(mem, EFFECT_MATRIX);
    }

    /// `Sg_ToggleTkAnti()` (`0x00821e30`): flips [`TK_ANTI`].
    pub fn toggle_tk_anti(mem: &mut impl Mem) {
        toggle(mem, TK_ANTI);
    }

    /// `Sg_Id_coltest()` (`0x00844cb0`): flips the word; the new value.
    pub fn id_coltest(mem: &mut impl Mem) -> u32 {
        let v = ID_COLTEST.get(mem).wrapping_sub(1) & 1;
        ID_COLTEST.set(mem, v);
        v
    }

    /// `Sg_SwitchDrawId()` (`0x00845010`): the next mode, 3 back to 0
    /// (signed: past 3 is 0); the new value.
    pub fn switch_draw_id(mem: &mut impl Mem) -> i32 {
        let c = ID_DRAW.get(mem).wrapping_add(1);
        let v = if c > 3 { 0 } else { c };
        ID_DRAW.set(mem, v);
        v
    }

    /// `Sg_CaptureGetGuiHide()` (`0x0083a620`).
    pub fn capture_get_gui_hide(mem: &impl Mem) -> u32 {
        u32::from(CAPTURE_ON.get(mem) != 0 && CAPTURE_GUI_HIDE.get(mem) != 0)
    }

    /// `Sg_MemSetMoveHeapStartAddr(a)` (`0x0083c530`): `a - 16`, or the
    /// heap's start for null; the value stored.
    pub fn mem_set_move_heap_start_addr(mem: &mut impl Mem, a: u32) -> u32 {
        let v = if a == 0 {
            MEM_HEAP_START.get(mem)
        } else {
            a.wrapping_sub(0x10)
        };
        MEM_MOVE_START.set(mem, v);
        v
    }

    /// `Sg_IdSetEffectTable(i, v)` (`0x0083e9b0`): entry `i` of the
    /// current set; `EAX` ends as `i`.
    pub fn id_set_effect_table(mem: &mut impl Mem, i: u32, v: u32) -> u32 {
        let set = i32::from(ID_EFFECT_TABLE_INDEX.get(mem).cast_signed()).cast_unsigned();
        let at = elem(mem, ID_EFFECT_TABLE, set, 4).wrapping_add(i);
        mem.write(at, lo8(v));
        i
    }

    /// `Sg_GetResourceSTR(i)` (`0x0083fc30`): the resource string of
    /// entry `i`.
    pub fn get_resource_str(mem: &impl Mem, i: u32) -> u32 {
        let off: u16 = mem.read(elem(mem, RESOURCE_TABLE, i, 4).wrapping_add(2));
        RESOURCE_STR
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(off.cast_signed()).cast_unsigned())
    }

    /// `Sg_MapGetName(which)` (`0x00844cf0`): the selected map's name in
    /// one of the two lists.
    pub fn map_get_name(mem: &impl Mem, which: u32) -> u32 {
        let off = MAP_DIR_SELECT.get(mem) << 6;
        let base = if which == 0 { MAP_NAMES_ALT } else { MAP_NAMES };
        base.ptr(mem).addr.wrapping_add(off)
    }

    /// `ReadU16Unaligned(p)` (`0x00838220`): both bytes sign-extended, so a
    /// low byte from `0x80` sets every high bit.
    pub fn read_u16_unaligned(mem: &impl Mem, p: u32) -> i32 {
        let [lo, hi]: [u8; 2] = mem.read(p);
        (i32::from(hi.cast_signed()) << 8) | i32::from(lo.cast_signed())
    }

    /// `sa_task_buff_size_get(n)` (`0x0077a180`): `n` tasks of `0x1330`
    /// bytes plus `0x607`, rounded down to 64.
    #[must_use]
    pub const fn sa_task_buff_size_get(n: u32) -> u32 {
        n.wrapping_mul(0x1330).wrapping_add(0x607) & !0x3f
    }

    /// `Sg_GetIsPrerend()` (`0x00843210`): bit 14 of `yiGetRenderStatus()`
    /// (`status`, which the caller supplies).
    #[must_use]
    pub const fn get_is_prerend(status: u32) -> u32 {
        (status >> 14) & 1
    }

    /// Bytes from `src` up to and including its NUL to `dst` (`strcpy`).
    pub fn strcpy(mem: &mut impl Mem, dst: u32, src: u32) {
        let mut i = 0u32;
        loop {
            let c: u8 = mem.read(src.wrapping_add(i));
            mem.write(dst.wrapping_add(i), c);
            if c == 0 {
                break;
            }
            i = i.wrapping_add(1);
        }
    }

    /// The length of the string at `s` (`strlen`).
    pub fn strlen(mem: &impl Mem, s: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    // Fades, flash, filter, focus, the accumulation buffer.
    /// What the frame's screen effects do (bit 2 flash, 3 filter, 4 fade).
    pub const EFFECTS: Global<u32> = Global::new(0x0130_0704);
    /// The flash: mode `u8` at `+0`, colour at `+1..4`, time `u16` at `+4`
    /// and `+6` (left and total).
    pub const FLASH_STATE: Global<u8> = Global::new(0x0130_0728);
    /// The filter: mode at `+0`, colour at `+1..4`, on (`u16`) at `+4`,
    /// time (`u16`) at `+6`.
    pub const FILTER_STATE: Global<u8> = Global::new(0x0130_0738);
    /// The fade: mode at `+0`, `0xff` at `+1`, `0xffff` at `+2`, time
    /// (`u16`) at `+4` and `+6`.
    pub const FADE_STATE: Global<u8> = Global::new(0x0130_0748);
    /// The focus bits (`Sg_SetFocus`).
    pub const FOCUS_BITS: Global<u32> = Global::new(0x0130_0768);
    /// The accumulation buffer's clear colour (`Sg_AccSetClear`).
    pub const ACC_CLEAR: Global<[u8; 4]> = Global::new(0x0130_0771);
    /// The player's control state after [`PLAYER`] (four words cleared by
    /// `Sg_SetPlayer`).
    pub const PLAYER_STATE: Global<[u32; 4]> = Global::new(0x0130_078c);
    /// The debug pad holds the frame count (`Sg_GetCurExecFrames` then
    /// uses [`EXEC_SPECIAL_SPEED`]).
    pub const EXEC_SPECIAL: Global<u8> = Global::new(0x0130_07b1);

    /// `Sg_Fade_Common(time, out, black)` (`0x0082cdc0`): mode 2 to 5 from
    /// the two flags (`2 + 2 black` fading out, `3 + 2 black` in); `EAX`
    /// ends as `time`.
    pub fn fade_common(mem: &mut impl Mem, time: u32, out: u32, black: u32) -> u32 {
        let e = EFFECTS.get(mem) | 0x10;
        EFFECTS.set(mem, e);
        let mode = u8::from(black != 0) * 2 + if out == 0 { 3 } else { 2 };
        let p = FADE_STATE.ptr(mem).addr;
        mem.write(p, mode);
        let t = crate::lo16(time);
        mem.write(p + 4, [t, t]);
        mem.write(p + 2, 0xffffu16);
        mem.write(p + 1, 0xffu8);
        time
    }

    /// `Sg_Flash(time, r, g, b)` (`0x0082cca0`): mode 6.
    pub fn flash(mem: &mut impl Mem, time: u32, r: u32, g: u32, b: u32) {
        let e = EFFECTS.get(mem) | 4;
        EFFECTS.set(mem, e);
        let p = FLASH_STATE.ptr(mem).addr;
        let t = crate::lo16(time);
        mem.write(p + 4, [t, t]);
        mem.write(p, [6, lo8(r), lo8(g), lo8(b)]);
    }

    /// `Sg_FilterAdd(r, g, b, time)` (`0x0082cc20`): mode 7; with a time,
    /// the colour, the time and on; without, off. Returns on (0 or 1).
    pub fn filter_add(mem: &mut impl Mem, r: u32, g: u32, b: u32, time: u32) -> u32 {
        let e = EFFECTS.get(mem) | 8;
        EFFECTS.set(mem, e);
        let p = FILTER_STATE.ptr(mem).addr;
        mem.write(p, 7u8);
        if time == 0 {
            mem.write(p + 4, 0u16);
            return 0;
        }
        mem.write(p + 1, [lo8(r), lo8(g), lo8(b)]);
        mem.write(p + 6, crate::lo16(time));
        mem.write(p + 4, 1u16);
        1
    }

    /// `Sg_SetFocus(i, v)` (`0x0082ce20`): focus value `i`; a non-zero
    /// value sets bit `v & 31` of the focus bits and turns focus on
    /// (`Sg_EnFocus(1)`, whose old value it returns); 0 clears bit 0 (and
    /// returns `i`).
    pub fn set_focus(mem: &mut impl Mem, i: u32, v: u32) -> u32 {
        mem.write(FOCUS.ptr(mem).addr.wrapping_add(i.wrapping_mul(4)), v);
        let bits = FOCUS_BITS.get(mem);
        if v == 0 {
            FOCUS_BITS.set(mem, bits & !1);
            return i;
        }
        FOCUS_BITS.set(mem, bits | 1 << (v & 31));
        en_focus(mem, 1)
    }

    /// `Sg_AccSetClear(r, g, b, a)` (`0x0082bdc0`).
    pub fn acc_set_clear(mem: &mut impl Mem, r: u32, g: u32, b: u32, a: u32) {
        ACC_CLEAR.set(mem, [r, g, b, a].map(lo8));
    }

    /// `Sg_SetPlayer(chr)` (`0x0082dad0`): also clears the player's control
    /// state; returns `chr`.
    pub fn set_player(mem: &mut impl Mem, chr: u32) -> u32 {
        PLAYER.set(mem, chr);
        PLAYER_STATE.set(mem, [0; 4]);
        chr
    }

    /// `Sg_GetCurExecFrames()` (`0x0082d7e0`): the frames run per game
    /// frame (the special speed while the debug pad holds it) OR-ed with
    /// `Sg_GetExecStep()`.
    pub fn get_cur_exec_frames(mem: &impl Mem) -> i32 {
        let n = if EXEC_SPECIAL.get(mem) == 0 {
            get(mem, EXEC_FRAMES)
        } else {
            get(mem, EXEC_SPECIAL_SPEED)
        };
        get_exec_step(mem) | n
    }

    /// `Sg_GetPlayerPos(&v)` (`0x0082d880`): 1 without a player; else its
    /// position (`Ch_GetPos`) into `v`, and 0.
    pub fn get_player_pos(mem: &mut impl Mem, v: u32) -> u32 {
        let chr = PLAYER.get(mem);
        if chr == 0 {
            return 1;
        }
        for k in 0..3 {
            let x: f32 = mem.read(chr.wrapping_add(ffx_chara::layout::POS + 4 * k));
            mem.write(v.wrapping_add(4 * k), ffx_bridge::x87::round_trip(x));
        }
        0
    }

    // Capture.
    /// Show the GUI in captures (`Sg_CaptureSetGui`).
    pub const CAPTURE_GUI: Global<u8> = Global::new(0x0130_181d);
    /// The capture state words from `0x01301804`: on, 3, the mode, two
    /// flags (`Sg_CaptureEnable*`).
    pub const CAPTURE: Global<u32> = Global::new(0x0130_1804);
    /// The draw environment bytes the capture keeps (`0x01301944..=48`).
    pub const CAPTURE_ENV: Global<[u8; 5]> = Global::new(0x0130_1944);
    /// Where they come from: the display environment's words, in the
    /// order they are kept.
    pub const DISPLAY_ENV: [Global<u8>; 5] = [
        Global::new(0x023c_bc24),
        Global::new(0x023c_bc20),
        Global::new(0x023c_bc30),
        Global::new(0x023c_bc2c),
        Global::new(0x023c_bc28),
    ];

    /// Keeps the display environment's bytes; the last one.
    fn capture_env(mem: &mut impl Mem) -> u32 {
        let env = DISPLAY_ENV.map(|g| g.get(mem));
        CAPTURE_ENV.set(mem, env);
        u32::from(env[4])
    }

    fn capture_words(mem: &mut impl Mem, words: &[(u32, u32)]) {
        let base = CAPTURE.ptr(mem).addr;
        for &(off, v) in words {
            mem.write(base.wrapping_add(off), v);
        }
    }

    /// `Sg_CaptureEnable()` (`0x0083a470`): starts a capture of the GUI
    /// setting; the last kept environment byte.
    pub fn capture_enable(mem: &mut impl Mem) -> u32 {
        let hide = 1 - (u32::from(CAPTURE_GUI.get(mem)) & 1);
        CAPTURE_GUI_HIDE.set(mem, hide);
        let r = capture_env(mem);
        capture_words(mem, &[(0, 1), (4, 3), (8, 0), (0xc, 0)]);
        r
    }

    /// `Sg_CaptureEnable_Kari(mode, on)` (`0x0083a4f0`): with `on`, a
    /// capture in `mode` hiding the GUI.
    pub fn capture_enable_kari(mem: &mut impl Mem, mode: u32, on: u32) {
        if on == 0 {
            return;
        }
        mem.write(CAPTURE.ptr(mem).addr.wrapping_add(8), mode);
        capture_env(mem);
        capture_words(mem, &[(0, on), (4, 3), (0xc, 0), (0x10, 0)]);
        CAPTURE_GUI_HIDE.set(mem, 1);
    }

    /// `Sg_CaptureEnable_KariOn(mode)` (`0x0083a580`); the last kept
    /// environment byte.
    pub fn capture_enable_kari_on(mem: &mut impl Mem, mode: u32) -> u32 {
        mem.write(CAPTURE.ptr(mem).addr.wrapping_add(8), mode);
        let r = capture_env(mem);
        capture_words(mem, &[(0, 1), (0xc, 1), (0x10, 0)]);
        CAPTURE_GUI_HIDE.set(mem, 1);
        r
    }

    /// `Sg_CaptureSetGui(v)` (`0x0083a660`).
    pub fn capture_set_gui(mem: &mut impl Mem, v: u32) {
        CAPTURE_GUI.set(mem, lo8(v));
    }

    /// `Sg_GetTkAntiXYB(&x, &y, &b)` (`0x0083adf0`): null outputs skipped.
    pub fn get_tk_anti_xyb(mem: &mut impl Mem, out: [u32; 3]) {
        let v = TK_ANTI_XYB.get(mem);
        for (o, x) in out.into_iter().zip(v) {
            if o != 0 {
                mem.write(o, x);
            }
        }
    }

    // Resources: entry `i` of [`RESOURCE_TABLE`] has its size (`i8`, 1 or
    // 2 bytes, or the string's room) at `+1` and its offset at `+2`.
    /// The resources changed (`Sg_SetResource`, `Sg_SaveResourceCheck`).
    pub const RESOURCE_DIRTY: Global<u32> = Global::new(0x0130_1bec);

    fn resource(mem: &impl Mem, i: u32) -> (i8, u32) {
        let e = elem(mem, RESOURCE_TABLE, i, 4);
        let size: i8 = mem.read(e.wrapping_add(1));
        let off: i16 = mem.read(e.wrapping_add(2));
        (
            size,
            RESOURCE_STR
                .ptr(mem)
                .addr
                .wrapping_add(i32::from(off).cast_unsigned()),
        )
    }

    /// `Sg_GetResource(i, default)` (`0x0083fbe0`): a one- or two-byte
    /// resource, else `default`.
    pub fn get_resource(mem: &impl Mem, i: u32, default: u32) -> u32 {
        match resource(mem, i) {
            (1, at) => u32::from(mem.read::<u8>(at)),
            (2, at) => u32::from(mem.read::<u16>(at)),
            _ => default,
        }
    }

    /// `Sg_SetResource(i, v)` (`0x0083fe10`): marks the resources changed
    /// and stores a one- or two-byte resource.
    pub fn set_resource(mem: &mut impl Mem, i: u32, v: u32) {
        RESOURCE_DIRTY.set(mem, 1);
        match resource(mem, i) {
            (1, at) => mem.write(at, lo8(v)),
            (2, at) => mem.write(at, [lo8(v), lo8(v >> 8)]),
            _ => {}
        }
    }

    /// `Sg_SetResourceSTR(i, s)` (`0x0083fe60`): copies `s` when shorter
    /// than the resource's room (compared unsigned, so a negative room
    /// takes any string) and marks the resources changed.
    pub fn set_resource_str(mem: &mut impl Mem, i: u32, s: u32) {
        let len = strlen(mem, s);
        let (size, at) = resource(mem, i);
        if len < i32::from(size).cast_unsigned() {
            strcpy(mem, at, s);
            RESOURCE_DIRTY.set(mem, 1);
        }
    }

    // Maps.
    /// The map name lists the debug menu keeps (`Sg_MapInitAll` empties
    /// each first entry).
    pub const MAP_NAME_SLOTS: [Global<u8>; 6] = [
        Global::new(0x0130_3580),
        Global::new(0x0130_35c0),
        Global::new(0x0130_34c0),
        Global::new(0x0130_3500),
        Global::new(0x0130_3600),
        Global::new(0x0130_3540),
    ];

    /// `Sg_MapSetName(name, which)` (`0x00844fb0`): into the selected
    /// entry of one of the two lists ([`map_get_name`]).
    pub fn map_set_name(mem: &mut impl Mem, name: u32, which: u32) {
        let dst = map_get_name(mem, which);
        strcpy(mem, dst, name);
    }

    /// `Sg_MapInitAll()` (`0x00844eb0`): in debug builds the name slots
    /// get the empty string ([`FUNC_NAME`]); then `Sg_SetthDispEnable(1)`.
    pub fn map_init_all(mem: &mut impl Mem) {
        if IS_DEBUG.get(mem) != 0 {
            let src = FUNC_NAME.ptr(mem).addr;
            for g in MAP_NAME_SLOTS {
                let dst = g.ptr(mem).addr;
                strcpy(mem, dst, src);
            }
        }
        set_th_disp_enable(mem, 1);
    }

    // The debug menu and mouse.
    /// The mouse position (two words) and its `f32` copies, x doubled and
    /// y tripled (`Sg_SetMouse`).
    pub const MOUSE: Global<[u32; 4]> = Global::new(0x0132_5240);

    /// `Sg_SetMouse(x, y)` (`0x00851ae0`); `EAX` ends as `3 y`.
    pub fn set_mouse(mem: &mut impl Mem, x: i32, y: i32) -> i32 {
        let fx = ffx_bridge::x87::store(f64::from(x.wrapping_mul(2)));
        let y3 = y.wrapping_mul(3);
        let fy = ffx_bridge::x87::store(f64::from(y3));
        MOUSE.set(mem, [x.cast_unsigned(), y.cast_unsigned(), fx, fy]);
        y3
    }

    /// `sg_AddY(d)` (`0x00850820`); returns `d`.
    pub fn add_y(mem: &mut impl Mem, d: u32) -> u32 {
        let y = MENU_Y.get(mem).wrapping_add(d);
        MENU_Y.set(mem, y);
        d
    }

    /// The sound task buffers (`sa_task_start` clears them).
    pub const SA_TASKS: Global<[u32; 8]> = Global::new(0x0112_a818);

    /// `sa_task_start()` (`0x0077a2c0`).
    pub fn sa_task_start(mem: &mut impl Mem) {
        SA_TASKS.set(mem, [0; 8]);
    }

    /// The start time `FFXApplication::getAppTime` subtracts (`f64`).
    pub const APP_START: Global<f64> = Global::new(0x00c9_0278);

    /// `FFXApplication::getAppTime(this)` (`0x0042f690`, thiscall): `now`
    /// (`Phyre_PTimerWin32_GetTime`, which the caller supplies) less the
    /// start time less the application's pause time (`f32` at `+0x28`),
    /// as `f32`.
    pub fn get_app_time(mem: &impl Mem, this: u32, now: f64) -> f32 {
        use ffx_bridge::x87::Reg;
        let paused: f32 = mem.read(this.wrapping_add(0x28));
        (Reg::constant(now) - Reg::constant(APP_START.get(mem)))
            .sub_m32(paused)
            .store()
    }

    // Character database kinds.
    /// The kind names (`"skl"`, then `"pc"`, `"mon"`, `"npc"`, `"sum"`,
    /// `"wep"`, `"obj"` for kinds 0..=5), 4 bytes apart.
    pub const DB_NAMES: Global<u8> = Global::new(0x00b5_9330);

    /// `sg_getdbnum(c)` (`0x00829d80`): the kind of a kind letter, -1 for
    /// another.
    #[must_use]
    pub const fn get_db_num(c: i32) -> i32 {
        match c {
            0x63 => 0, // c
            0x6d => 1, // m
            0x6e => 2, // n
            0x73 => 3, // s
            0x77 => 4, // w
            0x66 => 5, // f
            0x6b => 6, // k
            _ => -1,
        }
    }

    /// `sg_getdbtype(kind)` (`0x00829e10`): the letter of kinds 0..=6
    /// (`c m n s w f k`, the inverse of [`get_db_num`]), `-` for another, in `AL`;
    /// the rest of `EAX` is the argument's.
    #[must_use]
    pub const fn get_db_type(kind: u32) -> u32 {
        let c = match kind {
            0 => b'c',
            1 => b'm',
            2 => b'n',
            3 => b's',
            4 => b'w',
            5 => b'f',
            6 => b'k',
            _ => b'-',
        };
        (kind & !0xff) | c as u32
    }

    /// `sg_getdbname(kind)` (`0x00829c90`): the name of a kind given as a
    /// number (0..=6) or a letter ([`get_db_num`]); the empty string for
    /// another.
    pub fn get_db_name(mem: &impl Mem, kind: u32) -> u32 {
        let k = if kind <= 6 {
            kind.cast_signed()
        } else {
            get_db_num(kind.cast_signed())
        };
        let names = DB_NAMES.ptr(mem).addr;
        match k {
            // "skl" is first in memory, kind 6.
            6 => names,
            0..=5 => names + 4 + 4 * k.cast_unsigned(),
            _ => FUNC_NAME.ptr(mem).addr,
        }
    }
}
