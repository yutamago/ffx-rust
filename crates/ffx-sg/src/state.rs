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

    // Round 3.
    /// The memory manager's alignment less one (`Sg_MemSetAlign`).
    pub const MEM_ALIGN: Global<u32> = Global::new(0x0130_1964);

    /// `Sg_MemSetAlign(align)` (`0x0083c4a0`): `align - 1` (the warning for
    /// a small alignment goes to the empty `dbgPrintf`); returns it.
    pub fn mem_set_align(mem: &mut impl Mem, align: u32) -> u32 {
        let v = align.wrapping_sub(1);
        MEM_ALIGN.set(mem, v);
        v
    }

    /// `Sg_MemCache(p)` (`0x0083b700`): `p` (the PS2's cache flush is gone).
    #[must_use]
    pub const fn mem_cache(p: u32) -> u32 {
        p
    }

    /// `Sg_FadeIn(time)` (`0x0082cb80`): `Sg_Fade_Common(time, 1, 0)`.
    pub fn fade_in(mem: &mut impl Mem, time: u32) -> u32 {
        fade_common(mem, time, 1, 0)
    }
    /// `Sg_FadeInW(time)` (`0x0082cba0`): from white, `Sg_Fade_Common(time, 1, 1)`.
    pub fn fade_in_w(mem: &mut impl Mem, time: u32) -> u32 {
        fade_common(mem, time, 1, 1)
    }
    /// `Sg_FadeOut(time)` (`0x0082cbc0`): `Sg_Fade_Common(time, 0, 0)`.
    pub fn fade_out(mem: &mut impl Mem, time: u32) -> u32 {
        fade_common(mem, time, 0, 0)
    }
    /// `Sg_FadeOutW(time)` (`0x0082cbe0`): to white.
    pub fn fade_out_w(mem: &mut impl Mem, time: u32) -> u32 {
        fade_common(mem, time, 0, 1)
    }

    /// `Sg_FilterSub(r, g, b, time)` (`0x0082cc80`): `Sg_FilterAdd`, then
    /// mode 8 (subtractive); returns on.
    pub fn filter_sub(mem: &mut impl Mem, r: u32, g: u32, b: u32, time: u32) -> u32 {
        let on = filter_add(mem, r, g, b, time);
        FILTER_STATE.set(mem, 8);
        on
    }

    /// `Sg_FocusReset()` (`0x0082ccf0`): focus values `0x10000` and
    /// `0x20000`, then focus off (`Sg_EnFocus(0)`, whose old value it
    /// returns).
    pub fn focus_reset(mem: &mut impl Mem) -> u32 {
        set_focus(mem, 0, 0x1_0000);
        set_focus(mem, 1, 0x2_0000);
        en_focus(mem, 0)
    }

    /// `Sg_ToggleFocus()` (`0x0082cd80`): flips focus on; when it comes on,
    /// both focus values are set again (their bits).
    pub fn toggle_focus(mem: &mut impl Mem) {
        let v = FOCUS_ENABLE.get(mem).wrapping_sub(1) & 1;
        FOCUS_ENABLE.set(mem, v);
        if v != 0 {
            let f0 = get_focus(mem, 0);
            set_focus(mem, 0, f0);
            let f1 = get_focus(mem, 1);
            set_focus(mem, 1, f1);
        }
    }

    /// The vertical blank counter (`g_SgVCount`).
    pub const VCOUNT: Global<u32> = Global::new(0x012f_b7f0);
    /// The counter at the last `Sg_MainCalcRate`.
    pub const VCOUNT_LAST: Global<u32> = Global::new(0x012f_bc30);
    /// The rate measurement mode (1: from the history).
    pub const RATE_MODE: Global<u32> = Global::new(0x012f_b8a0);
    /// The rate history: index pair and entries (`Sg_MainCalcRate`).
    pub const RATE_HIST_HEAD: Global<u32> = Global::new(0x012f_b814);
    /// See [`RATE_HIST_HEAD`].
    pub const RATE_HIST_BASE: Global<u32> = Global::new(0x012f_b808);
    /// See [`RATE_HIST_HEAD`].
    pub const RATE_HIST: Global<u32> = Global::new(0x012f_b828);
    /// The frame's rate in 1/256 frames (`0x100` at normal speed).
    pub const RATE: Global<u32> = Global::new(0x023c_bbec);
    /// [`RATE`] as a float (1.0 at normal speed).
    pub const RATE_F: Global<f32> = Global::new(0x023c_bbe8);
    /// 2^32 (`f32` in `.rdata`): an unsigned conversion's correction.
    pub const TWO_32: Global<f32> = Global::new(0x00b1_1f14);
    /// 1/256 (`f64` in `.rdata`).
    pub const RATE_SCALE: Global<f64> = Global::new(0x00b5_0198);

    /// `Sg_MainCalcRate()` (`0x008206c0`): the frame's rate from the
    /// vertical blanks since the last call (`<< 7`, or the history entry in
    /// mode 1), as an integer and a float (`/ 256`); normal speed (`0x100`,
    /// 1.0) unless exactly one frame runs per game frame. Returns
    /// `Sg_GetCurExecFrames()`.
    pub fn main_calc_rate(mem: &mut impl Mem) -> i32 {
        use ffx_bridge::x87::Reg;
        let vc = VCOUNT.get(mem);
        let mut rate = vc.wrapping_sub(VCOUNT_LAST.get(mem)) << 7;
        RATE.set(mem, rate);
        if RATE_MODE.get(mem) == 1 {
            let i = RATE_HIST_HEAD
                .get(mem)
                .wrapping_sub(RATE_HIST_BASE.get(mem));
            rate = mem.read(RATE_HIST.ptr(mem).addr.wrapping_add(i.wrapping_mul(4)));
            RATE.set(mem, rate);
        }
        let mut x = Reg::constant(f64::from(rate.cast_signed()));
        if rate.cast_signed() < 0 {
            x = x.add_m32(TWO_32.get(mem));
        }
        let fr = Reg::load(x.store());
        let rf = (fr * Reg::constant(RATE_SCALE.get(mem))).store();
        VCOUNT_LAST.set(mem, vc);
        RATE_F.set(mem, rf);
        let n = get_cur_exec_frames(mem);
        if n != 1 {
            RATE.set(mem, 0x100);
            RATE_F.set(mem, 1.0);
        }
        n
    }

    /// The development user name `Sg_GetUserName2` gives in viewer mode
    /// (`"sugimoto"`).
    pub const DEV_USER_NAME: Global<u8> = Global::new(0x00b5_9078);

    /// `Sg_GetUserName2()` (`0x00820620`): [`DEV_USER_NAME`] in viewer
    /// mode, else `Sg_GetUserName()`.
    pub fn get_user_name2(mem: &impl Mem) -> u32 {
        if get_viewer_mode(mem) == 0 {
            get_user_name(mem)
        } else {
            DEV_USER_NAME.ptr(mem).addr
        }
    }

    /// `Sg_IdGetShade(c)` (`0x0083e980`): bits 17..32 of a map id's colour
    /// word when shading is on (`Sg_IdExistShade`), else `0x4210` (grey).
    pub fn id_get_shade(mem: &impl Mem, c: u32) -> u32 {
        if id_exist_shade(mem) == 0 {
            0x4210
        } else {
            (c.cast_signed() >> 17).cast_unsigned() & 0x7fff
        }
    }

    /// `Sg_Debug(what, v)` (`0x0083e5c0`): `what == 0x1113` sets the byte
    /// at `0x01301a80`; anything else goes to the empty
    /// `debug_exit_trace`.
    pub fn debug(mem: &mut impl Mem, what: u32, v: u32) {
        if what == 0x1113 {
            let at = mem.rebase(0x0130_1a80);
            mem.write(at, lo8(v));
        }
    }

    /// `getUCStatus()` (`0x00822460`): bit 1 of the current script control
    /// work's first byte (`AtelGetCurCtrlWork`), 0 without one.
    pub fn get_uc_status(mem: &impl Mem) -> u32 {
        let work = ffx_script::vm::bridge::atel_get_cur_ctrl_work(mem);
        if work == 0 {
            0
        } else {
            (u32::from(mem.read::<u8>(work)) >> 1) & 1
        }
    }

    /// The dark Yojimbo flag (`set0434isDarkYojimbo`).
    pub const DARK_YOJIMBO: Global<u32> = Global::new(0x00ce_c198);

    /// `set0434isDarkYojimbo(id)` (`0x00829c60`): sets the flag when the
    /// `u16` at `id` is `0x1154` or, failing that, the battle scene
    /// (`MsGetBattleScene()`) is `0x24b0006`.
    pub fn set_dark_yojimbo(mem: &mut impl Mem, id: u32) {
        let hit = id != 0 && mem.read::<u16>(id) == 0x1154;
        if hit || ffx_battle::state::bridge::ms_get_battle_scene(mem) == 0x24b_0006 {
            DARK_YOJIMBO.set(mem, 1);
        }
    }

    /// The player's direction: fixed on (`u8`) and the angle (`f32`)
    /// (`Sg_PlayerSetDirFix`).
    pub const PLAYER_DIR_FIX: Global<u8> = Global::new(0x0130_07c4);
    /// See [`PLAYER_DIR_FIX`].
    pub const PLAYER_DIR: Global<f32> = Global::new(0x0130_07c8);

    /// `Sg_PlayerSetDirFix(on, degrees)` (`0x0082d990`): the angle in
    /// radians (`x π / 180`, stored as `f32`), wrapped (`Angle_Wrap`).
    pub fn player_set_dir_fix(mem: &mut impl Mem, on: u32, degrees: f32) {
        use ffx_bridge::x87::Reg;
        let pi = Reg::constant(crate::quat::bridge::PI.get(mem));
        let k = Reg::constant(crate::quat::bridge::DEG_180.get(mem));
        let r = (Reg::load(degrees) * pi / k).store();
        PLAYER_DIR_FIX.set(mem, lo8(on));
        PLAYER_DIR.set(mem, crate::vec::bridge::angle_wrap(r));
    }

    /// The mouse buttons: the raw state, this frame's and last frame's
    /// decoded bits, the double-click position and timer
    /// (`Sg_MouseButtonMain`).
    pub const MOUSE_RAW: Global<u32> = Global::new(0x0132_5260);
    /// See [`MOUSE_RAW`]: the decoded bits (`g_*`, read by the pad layer).
    pub const MOUSE_BUTTONS: Global<u32> = Global::new(0x023c_bc10);
    /// See [`MOUSE_RAW`].
    pub const MOUSE_BUTTONS_LAST: Global<u32> = Global::new(0x0132_5250);
    /// See [`MOUSE_RAW`].
    pub const MOUSE_CLICK_POS: Global<[u32; 2]> = Global::new(0x0132_5254);
    /// See [`MOUSE_RAW`].
    pub const MOUSE_CLICK_TIMER: Global<u32> = Global::new(0x023c_bc00);

    /// The decoded bits of one button: held (`hold`), pressed this frame
    /// (`hold << 1`), released this frame (`hold << 2`).
    fn edges(now: u32, last: u32, hold: u32) -> u32 {
        let mut v = now;
        if v & hold != 0 && last & hold == 0 {
            v |= hold << 1;
        }
        if v & hold == 0 && last & hold != 0 {
            v |= hold << 2;
        }
        v
    }

    /// `Sg_MouseButtonMain()` (`0x008517c0`): the three buttons' held,
    /// pressed and released bits (left 1/2/4, right `0x100`.., middle
    /// `0x10000`..), and a double click (8) when the left button is
    /// pressed again at the same position within 50 frames. Returns the
    /// new bits.
    pub fn mouse_button_main(mem: &mut impl Mem) -> u32 {
        let last = MOUSE_BUTTONS.get(mem);
        let raw = MOUSE_RAW.get(mem);
        MOUSE_BUTTONS_LAST.set(mem, last);
        let mut v = u32::from(raw & 1 != 0);
        v = edges(v, last, 1);
        let mut timer = MOUSE_CLICK_TIMER.get(mem);
        if v & 2 != 0 {
            let pos = [MOUSE.get(mem)[0], MOUSE.get(mem)[1]];
            if MOUSE_CLICK_POS.get(mem) == pos && timer != 0 {
                v |= 8;
                MOUSE_CLICK_POS.set(mem, [u32::MAX; 2]);
            } else {
                timer = 0x32;
                MOUSE_CLICK_POS.set(mem, pos);
            }
        }
        if timer != 0 {
            timer -= 1;
            MOUSE_CLICK_TIMER.set(mem, timer);
        }
        if raw & 2 != 0 {
            v |= 0x100;
        }
        v = edges(v, last, 0x100);
        if raw & 4 != 0 {
            v |= 0x1_0000;
        }
        v = edges(v, last, 0x1_0000);
        MOUSE_BUTTONS.set(mem, v);
        v
    }

    /// `Sg_IdSetPtr(id)` (`0x0083ea00`): the current map id data: its
    /// table count (`i16` at `+8` of `+0x1c`), the table at `+0xc` of that,
    /// the data at `+0x18`, the zoom (`+0xc / 10`); for null all 0 and zoom 1.
    /// Then every used map object (`0x880` bytes each, `+2` not 0) gets
    /// `-1` at `+0x824`.
    pub fn id_set_ptr(mem: &mut impl Mem, id: u32) {
        use ffx_bridge::x87::Reg;
        ID_PTR.set(mem, id);
        let base = ID_PTR.ptr(mem).addr;
        // 0x01301a84: table, 0x88: count, 0x8c: data, 0x90: zoom.
        let zoom = if id == 0 {
            mem.write(base.wrapping_sub(0x10), [0u32, 0, 0]);
            1.0
        } else {
            let hdr: u32 = mem.read(id.wrapping_add(0x1c));
            let count = i32::from(mem.read::<i16>(hdr.wrapping_add(8))).cast_unsigned();
            let table: u32 = mem.read(hdr.wrapping_add(0xc));
            let data: u32 = mem.read(id.wrapping_add(0x18));
            mem.write(base.wrapping_sub(0x10), [table, count, data]);
            let z: f32 = mem.read(id.wrapping_add(0xc));
            (Reg::load(z) / Reg::constant(ZOOM_DIVISOR.get(mem))).store()
        };
        ID_ZOOM.set(mem, zoom);
        let n = MAP_OBJECT_COUNT.get(mem);
        let objs = MAP_OBJECTS.get(mem);
        let end = objs.wrapping_add(n.wrapping_mul(0x880));
        let mut p = objs;
        while p != end {
            if mem.read::<u8>(p.wrapping_add(2)) != 0 {
                mem.write(p.wrapping_add(0x824), 0xffffu16);
            }
            p = p.wrapping_add(0x880);
        }
    }

    /// `g_ChrIdShadeOn` (`i8`).
    pub const CHR_ID_SHADE_ON: Global<u8> = Global::new(0x0130_34b8);
    /// `graphicCharacterLightingSetMapIdFlagChanged(old, new)`.
    pub const LIGHTING_SET_MAP_ID_FLAG_CHANGED: u32 = 0x0063_cd20;

    /// `Sg_IdFixupAndSet(map)` (`0x00844bc0`): for a map's id data, the
    /// id shades go on for a version (`i16` at `+4`) above `0x1202` (the
    /// renderer told of a change, old value sign-extended), the data's
    /// offsets become pointers (word 0 holds the base they were taken from:
    /// `+0x1c`, `+0x18`, and `+0xc` of each of the `i16` `+6` 16-byte
    /// entries in `+0x1c`'s table, each moved by `map - base`), then
    /// `Sg_IdSetPtr(map)`. A null map only goes to `Sg_IdSetPtr`.
    /// (`get_ptr32_func`/`set_ptr32` are identities on the PC.)
    pub fn id_fixup_and_set(h: &mut impl ffx_bridge::Host, map: u32) {
        if map != 0 {
            let old = i32::from(CHR_ID_SHADE_ON.get(h).cast_signed()).cast_unsigned();
            let on = u32::from(h.read::<i16>(map.wrapping_add(4)) > 0x1202);
            if on != old {
                let at = h.rebase(LIGHTING_SET_MAP_ID_FLAG_CHANGED);
                h.call(at, &[old, on]);
            }
            let on = u8::from(h.read::<i16>(map.wrapping_add(4)) > 0x1202);
            CHR_ID_SHADE_ON.set(h, on);
            let delta = map.wrapping_sub(h.read::<u32>(map));
            if delta != 0 {
                h.write(map, delta);
                for off in [0x1c, 0x18] {
                    let p: u32 = h.read(map.wrapping_add(off));
                    h.write(map.wrapping_add(off), p.wrapping_add(delta));
                }
                let mut k = 0i32;
                while k < i32::from(h.read::<i16>(map.wrapping_add(6))) {
                    let table: u32 = h.read(map.wrapping_add(0x1c));
                    let e = table.wrapping_add(0xc).wrapping_add(16 * k.cast_unsigned());
                    let p: u32 = h.read(e);
                    let table: u32 = h.read(map.wrapping_add(0x1c));
                    let e = table.wrapping_add(0xc).wrapping_add(16 * k.cast_unsigned());
                    h.write(e, p.wrapping_add(delta));
                    k += 1;
                }
            }
        }
        id_set_ptr(h, map);
    }

    /// `Sg_IdInitAll()` (`0x00844ca0`): `Sg_IdSetPtr(0)`.
    pub fn id_init_all(mem: &mut impl Mem) {
        id_set_ptr(mem, 0);
    }

    /// `Sg_PlayerInitAll()` (`0x0082d910`): no player (`Sg_SetPlayer(0)`),
    /// diving on (`Sg_PlayerSetDiving(1)`). Returns `eax`: the player's
    /// upper bytes (0) under the diving byte, so 1.
    pub fn player_init_all(mem: &mut impl Mem) -> u32 {
        let r = set_player(mem, 0);
        r & !0xff | u32::from(player_set_diving(mem, 1))
    }

    /// `SgStrdupD`, called as the game's.
    pub const SG_STRDUP_D: u32 = 0x0083_f050;

    /// `Sg_SetUserName(name)` (`0x00821d30`): [`USER_NAME`] becomes a copy
    /// (`SgStrdupD(name)`, not freed before); returns it.
    pub fn set_user_name(h: &mut impl ffx_bridge::Host, name: u32) -> u32 {
        let at = h.rebase(SG_STRDUP_D);
        let p = h.call(at, &[name]);
        USER_NAME.set(h, p);
        p
    }

    /// The filter requests (`0x01300708..0x01300758`: cross-fade, blur
    /// and their neighbours), cleared by `Sg_InitFilter`.
    pub const FILTER_BLOCK: u32 = 0x0130_0708;
    /// The request flags (`Sg_AccSetAlpha`, `Sg_Blur`) and `0x01300768`.
    pub const FILTER_FLAGS: Global<u32> = Global::new(0x0130_0704);
    pub const FILTER_1300768: Global<u32> = Global::new(0x0130_0768);

    /// `Sg_InitFilter()` (`0x0082cd50`): the filter block (`0x50` bytes)
    /// and the two words cleared, then `Sg_FocusReset` (a tail call: its
    /// `eax`).
    pub fn init_filter(mem: &mut impl Mem) -> u32 {
        let at = mem.rebase(FILTER_BLOCK);
        mem.write_bytes(at, &[0; 0x50]);
        FILTER_FLAGS.set(mem, 0);
        FILTER_1300768.set(mem, 0);
        focus_reset(mem)
    }

    /// `Sg_PlayerInitDefault()` (`0x0082d8d0`): [`player_init_all`], then
    /// the speeds of kinds 1, 2 and `0x12` from `.rdata` (`0x00b9231c` 16,
    /// `0x00b59830` 38, `0x00b4ae10` 20). Returns the last
    /// `Sg_PlayerSetSpeed`'s `eax` (2).
    pub fn player_init_default(mem: &mut impl Mem) -> u32 {
        player_init_all(mem);
        let mut r = 0;
        for (kind, at) in [(1, 0x00b9_231c), (2, 0x00b5_9830), (0x12, 0x00b4_ae10)] {
            let v = ffx_bridge::x87::round_trip(Global::<f32>::new(at).get(mem));
            r = player_set_speed(mem, kind, v);
        }
        r
    }

    /// The current id table (`Sg_IdSetPtr`'s, 16-byte entries).
    pub const ID_TABLE: Global<u32> = Global::new(0x0130_1a84);
    /// `g_ChrIdMapTable`: the kinds `0x30..0x3f` stand for.
    pub const CHR_ID_MAP_TABLE: u32 = 0x0130_19d8;

    /// `Sg_IdMatchChr(chr, i)` (`0x0083e4e0`): whether entry `i`'s kind
    /// (low 7 bits of `+0xc`) applies to `chr`: 1 everyone, 2 anyone but
    /// the player, 13 characters with bit 2 of `+0x198`, 14 the player;
    /// kinds `0x30..0x3f` stand for the kind in `g_ChrIdMapTable` (a whole
    /// word, followed again); anything else 0.
    pub fn id_match_chr(mem: &impl Mem, chr: u32, i: u32) -> u32 {
        let e = ID_TABLE.get(mem).wrapping_add(16 * i).wrapping_add(0xc);
        let mut kind = mem.read::<u32>(e) & 0x7f;
        let map = mem.rebase(CHR_ID_MAP_TABLE);
        loop {
            match kind {
                1 => return 1,
                2 => return u32::from(chr != get_player(mem)),
                13 => return (mem.read::<u32>(chr.wrapping_add(0x198)) >> 2) & 1,
                14 => return u32::from(chr == get_player(mem)),
                0x30..=0x3f => kind = mem.read(map.wrapping_add(4 * kind)),
                _ => return 0,
            }
        }
    }

    /// The current id data's vertices (`Sg_IdSetPtr`'s, 8 bytes each).
    pub const ID_VERTICES: Global<u32> = Global::new(0x0130_1a8c);

    /// `Sg_IdGetNormal(out, i)` (`0x0083e130`): the normal of id triangle
    /// `i` (three `i16` vertex numbers at the entry's start) into `out`
    /// (`Sg_CalcTriangleNormal`); -1 without id data or for `i` -1, else
    /// 0.
    pub fn id_get_normal(mem: &mut impl Mem, out: u32, i: u32) -> u32 {
        let table = ID_TABLE.get(mem);
        if table == 0 || i == u32::MAX {
            return u32::MAX;
        }
        let e = table.wrapping_add(i << 4);
        let verts = ID_VERTICES.get(mem);
        let [v0, v1, v2] = [0u32, 2, 4].map(|k| {
            let n = i32::from(mem.read::<i16>(e.wrapping_add(k)));
            verts.wrapping_add((n << 3).cast_unsigned())
        });
        crate::vec::bridge::calc_triangle_normal(mem, v0, v1, v2, out);
        0
    }

    /// The id edge a move last crossed (found by `0x0083d960`):
    /// its two `i16` vertices.
    pub const ID_EDGE_B: Global<u32> = Global::new(0x0130_1aec);
    pub const ID_EDGE_A: Global<u32> = Global::new(0x0130_1af0);

    /// `Sg_IdSlideEdge(from, to)` (`0x0083e180`): a move from `from` to
    /// `to` (`f32` x, z) that crossed the edge [`ID_EDGE_B`] to
    /// [`ID_EDGE_A`] slides along it. The move, in the edge's frame (`M`,
    /// the rotation about y taking x to the edge's unit direction;
    /// `FFXVu0ApplyMatrix`), loses its across component. While the result
    /// back in the world (`FFXVu0InversMatrix`) is still on the far side
    /// (`w.z * n.x - w.x * n.z` above 0), it steps one unit further back
    /// along z. `to` becomes `from + w` in x and z (y as it was).
    pub fn id_slide_edge(mem: &mut impl Mem, from: u32, to: u32) {
        use crate::scratch::{SCRATCH, Scratch};
        use ffx_bridge::x87;
        use ffx_ps2::vu0::bridge as vu0;
        let ld = |b: u32| x87::load(b);
        let st = |x: f64| f32::from_bits(x87::store(x));
        let a = ID_EDGE_A.get(mem);
        let b = ID_EDGE_B.get(mem);
        let comp = |m: &_, p: u32, k: u32| i32::from(Mem::read::<i16>(m, p.wrapping_add(k)));
        let ex = st(f64::from(comp(mem, a, 0) - comp(mem, b, 0)));
        let ez = st(f64::from(comp(mem, a, 4) - comp(mem, b, 4)));
        let f = |m: &_, p: u32, k: u32| ld(Mem::read::<u32>(m, p.wrapping_add(4 * k)));
        let vx = st(x87::sub(f(mem, to, 0), f(mem, from, 0)));
        let vz = st(x87::sub(f(mem, to, 2), f(mem, from, 2)));
        let mut s = Scratch::new(mem);
        let (e, v, n, m, u, inv, w) = (
            SCRATCH,
            SCRATCH + 0x10,
            SCRATCH + 0x20,
            SCRATCH + 0x40,
            SCRATCH + 0x80,
            SCRATCH + 0xc0,
            SCRATCH + 0x100,
        );
        s.write(e, [ex, 0.0, ez, 0.0]);
        s.write(v, [vx, 0.0, vz, 0.0]);
        vu0::normalize(&mut s, n, e);
        vu0::matrix_identity(&mut s, m);
        let [nx, _, nz]: [f32; 3] = s.read(n);
        let (nx, nz) = (x87::round_trip(nx), x87::round_trip(nz));
        s.write(m, [nx, 0.0, -nz]);
        s.write(m + 0x10, [0f32, 1.0, 0.0]);
        s.write(m + 0x20, [nz, 0.0, nx]);
        vu0::apply_matrix(&mut s, u, m, v);
        ffx_system::math::bridge::orthonormal_inverse3x4(&mut s, inv, m);
        s.write(u + 8, 0f32);
        loop {
            vu0::apply_matrix(&mut s, w, inv, u);
            let [wx, _, wz]: [u32; 3] = s.read(w);
            let (nxb, nzb) = (ld(nx.to_bits()), ld(nz.to_bits()));
            let side = x87::sub(x87::mul(ld(wz), nxb), x87::mul(ld(wx), nzb));
            // a NaN ends it too
            if side.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                let x = st(x87::add(ld(wx), ld(s.read(from))));
                s.write(to, x);
                let z = st(x87::add(ld(wz), ld(s.read(from.wrapping_add(8)))));
                s.write(to.wrapping_add(8), z);
                return;
            }
            let uz = ld(s.read(u + 8));
            s.write(u + 8, st(x87::sub(uz, 1.0)));
        }
    }

    /// The free camera `Sg_ControlCamera` steers: yaw and pitch
    /// (radians), height and distance (`f32`), the distance preset counter.
    pub const FREE_CAM_YAW: Global<f32> = Global::new(0x00c4_def0);
    pub const FREE_CAM_PITCH: Global<f32> = Global::new(0x00c4_def4);
    pub const FREE_CAM_HEIGHT: Global<f32> = Global::new(0x00c4_dee4);
    pub const FREE_CAM_DIST: Global<f32> = Global::new(0x0130_1bb8);
    pub const FREE_CAM_PRESET: Global<u32> = Global::new(0x0130_1bb0);
    /// The pad as `Sg_ControlCamera` reads it: the right stick's x and y
    /// bytes, the buttons pressed this frame and held.
    pub const PAD_RSTICK_X: Global<u8> = Global::new(0x023c_44a6);
    pub const PAD_RSTICK_Y: Global<u8> = Global::new(0x023c_44a7);
    pub const PAD_PRESSED: Global<u32> = Global::new(0x023c_44a8);
    pub const PAD_HELD: Global<u32> = Global::new(0x023c_44ac);
    /// `Sg_ControlCamera`'s constants: degrees to radians (`f64`), 12, pi,
    /// 2 pi, -pi, the steps 0.2 and 0.4 (`f64`), and the six distances
    /// (`f32`: 50, 100, 180, 250, 1000, 30).
    pub const DEG_TO_RAD: Global<f64> = Global::new(0x00b5_a478);
    pub const TWELVE: Global<f64> = Global::new(0x00b4_b468);
    pub const PI_F64: Global<f64> = Global::new(0x00b5_82f0);
    pub const TWO_PI_F64: Global<f64> = Global::new(0x00b5_9638);
    pub const MINUS_PI_F64: Global<f64> = Global::new(0x00b5_9648);
    pub const STEP_HEIGHT: Global<f64> = Global::new(0x00b3_eb60);
    pub const STEP_DIST: Global<f64> = Global::new(0x00b4_8318);
    pub const DIST_PRESETS: [u32; 6] = [
        0x00b1_f2c8,
        0x00b3_04f8,
        0x00b5_a480,
        0x00b5_a484,
        0x00b1_f2cc,
        0x00b4_1d80,
    ];

    /// `Sg_ControlCamera()` (`0x0083f4b0`): the free camera from the pad.
    /// The right stick (outside the dead zone `0x51..0xaf`) turns yaw and
    /// pitch by `(stick - 0x80)` degrees / 12, each wrapped once into `-pi..
    /// pi`. A debug build captures on button `0x100` when a capture is
    /// selected. With the L/R button mode, buttons 2 and 8 raise and lower
    /// the height by 0.2, 1 and 4 the distance by 0.4. Button `0x400`
    /// pressed steps through six distances. Returns the switch's `eax`:
    /// the preset counter's low byte / 6 after a preset, else the held
    /// buttons with the L/R mode, else 0.
    pub fn control_camera(mem: &mut impl Mem) -> u32 {
        use ffx_bridge::x87;
        let ld = |v: f32| x87::load(v.to_bits());
        let st = |x: f64| f32::from_bits(x87::store(x));
        let stick = |b: u8| {
            let v = u32::from(b);
            if v.wrapping_sub(0x51) <= 0x5e {
                0x80
            } else {
                v
            }
        };
        let sx = stick(PAD_RSTICK_X.get(mem));
        let sy = stick(PAD_RSTICK_Y.get(mem));
        let (k, twelve) = (DEG_TO_RAD.get(mem), TWELVE.get(mem));
        let turn = |s: u32| {
            let a = ld(st(f64::from(s.cast_signed() - 0x80)));
            x87::div(x87::mul(a, k), twelve)
        };
        let yaw = st(x87::sub(ld(FREE_CAM_YAW.get(mem)), turn(sx)));
        FREE_CAM_YAW.set(mem, yaw);
        let pitch = st(x87::sub(ld(FREE_CAM_PITCH.get(mem)), turn(sy)));
        FREE_CAM_PITCH.set(mem, pitch);
        let (pi, two, mpi) = (PI_F64.get(mem), TWO_PI_F64.get(mem), MINUS_PI_F64.get(mem));
        for g in [FREE_CAM_YAW, FREE_CAM_PITCH] {
            let a = ld(g.get(mem));
            if a > pi {
                g.set(mem, st(x87::sub(a, two)));
            }
            let a = ld(g.get(mem));
            if a < mpi {
                g.set(mem, st(x87::add(a, two)));
            }
        }
        if IS_DEBUG.get(mem) != 0 && PAD_HELD.get(mem) & 0x100 != 0 && capture_get_select(mem) != 0
        {
            capture_enable(mem);
        }
        let mut r = 0;
        if get_lr_button_mode(mem) != 0 {
            let b = PAD_HELD.get(mem);
            let step = STEP_HEIGHT.get(mem);
            if b & 2 != 0 {
                FREE_CAM_HEIGHT.set(mem, st(x87::add(ld(FREE_CAM_HEIGHT.get(mem)), step)));
            }
            if b & 8 != 0 {
                FREE_CAM_HEIGHT.set(mem, st(x87::sub(ld(FREE_CAM_HEIGHT.get(mem)), step)));
            }
            let step = STEP_DIST.get(mem);
            if b & 1 != 0 {
                FREE_CAM_DIST.set(mem, st(x87::add(ld(FREE_CAM_DIST.get(mem)), step)));
            }
            if b & 4 != 0 {
                FREE_CAM_DIST.set(mem, st(x87::sub(ld(FREE_CAM_DIST.get(mem)), step)));
            }
            r = b;
        }
        if PAD_PRESSED.get(mem) & 0x400 != 0 {
            let n = FREE_CAM_PRESET.get(mem).wrapping_add(1);
            FREE_CAM_PRESET.set(mem, n);
            let low = n & 0xff;
            let at = DIST_PRESETS[(low % 6) as usize];
            let d = x87::round_trip(Global::<f32>::new(at).get(mem));
            FREE_CAM_DIST.set(mem, d);
            r = low / 6;
        }
        r
    }

    /// The diving byte (`0x013007c5`): while set, `Sg_ControlMain` adds
    /// gravity (0.2, below 2.0) to the player in motion type 2 with flag 0x40.
    pub const PLAYER_DIVING: Global<u8> = Global::new(0x0130_07c5);

    /// `Sg_PlayerSetDiving(v)` (`0x0082d9d0`): [`PLAYER_DIVING`]. Returns
    /// it (`al`).
    pub fn player_set_diving(mem: &mut impl Mem, v: u32) -> u8 {
        PLAYER_DIVING.set(mem, lo8(v));
        lo8(v)
    }

    /// The player's speeds `Sg_PlayerSetSpeed` sets: kinds 1, 2, `0x12`,
    /// `0x13` (`f32` each).
    pub const PLAYER_SPEED: [Global<f32>; 4] = [
        Global::new(0x0130_07b4),
        Global::new(0x0130_07b8),
        Global::new(0x0130_07bc),
        Global::new(0x0130_07c0),
    ];

    /// `Sg_PlayerSetSpeed(kind, v)` (`0x0082d9e0`): the speed of kind 1, 2,
    /// `0x12` or `0x13` (through `fld`/`fstp`); other kinds are ignored.
    /// Returns the switch's `eax`: the jump-table byte for kinds 1 to
    /// `0x13` (0, 1, 2, 3 for the four, 4 for the rest), else `kind - 1`.
    pub fn player_set_speed(mem: &mut impl Mem, kind: u32, v: f32) -> u32 {
        let i = kind.wrapping_sub(1);
        if i > 0x12 {
            return i;
        }
        let slot: u32 = match kind {
            1 => 0,
            2 => 1,
            0x12 => 2,
            0x13 => 3,
            _ => return 4,
        };
        PLAYER_SPEED[slot as usize].set(mem, ffx_bridge::x87::round_trip(v));
        slot
    }

    /// The camera-follow bits (`0x00c496d5`): bit 0 set by `0x0082d030`,
    /// bit 1 by `Sg_SetCameraFollow`; with both, `Sg_ControlMain` turns
    /// the camera after the player's heading.
    pub const CAMERA_FOLLOW: Global<u8> = Global::new(0x00c4_96d5);

    /// `Sg_SetCameraFollow(on)` (`0x0082dab0`): bit 1 of
    /// [`CAMERA_FOLLOW`]. Returns the other bits (`al`: the old byte
    /// without bit 1).
    pub fn set_camera_follow(mem: &mut impl Mem, on: u32) -> u8 {
        let rest = CAMERA_FOLLOW.get(mem) & 0xfd;
        CAMERA_FOLLOW.set(mem, rest | if on != 0 { 2 } else { 0 });
        rest
    }

    /// `Sg_IdSetPtr`'s zoom divisor (`f64` 10 in `.rdata`).
    pub const ZOOM_DIVISOR: Global<f64> = Global::new(0x00b9_22b0);
    /// The map objects (`0x880` bytes each) and their count.
    pub const MAP_OBJECTS: Global<u32> = Global::new(0x023c_44e4);
    /// See [`MAP_OBJECTS`].
    pub const MAP_OBJECT_COUNT: Global<u32> = Global::new(0x023c_44e0);

    /// The pending menu request (`Sg_MenuSetReserve`) and its argument.
    pub const MENU_RESERVE: Global<[u32; 2]> = Global::new(0x012f_bc38);
    /// Not 0 while a menu runs (`Sg_MenuSetReserve` drops the 0x42000000
    /// request otherwise).
    pub const MENU_ACTIVE: Global<u32> = Global::new(0x0134_082c);

    /// `Sg_MenuSetReserve(request)` (`0x00821750`): with the save screen
    /// closed (`SaveDataIsScreenClosed`), the save (`0x40200000`) and load
    /// (`0x40100000`) requests open it at once (`SaveDataSetScreenState(1)`,
    /// `TkMenuSaveLoadActive`, `SaveDataSetSave(1 or 0)`); any other
    /// request (`0x42000000` only while a menu runs) is kept for the main
    /// loop. `save_load_active` is the menu module's `TkMenuSaveLoadActive`
    /// (passed in: `ffx-menu` builds on this crate).
    pub fn menu_set_reserve<M: Mem>(
        mem: &mut M,
        request: u32,
        save_load_active: impl FnOnce(&mut M),
    ) {
        use ffx_system::save::bridge as save;
        if save::is_screen_closed(mem) != 0 && (request == 0x4010_0000 || request == 0x4020_0000) {
            save::set_screen_state(mem, 1);
            save_load_active(mem);
            save::set_save(mem, u32::from(request == 0x4020_0000));
            return;
        }
        if MENU_ACTIVE.get(mem) == 0 && request == 0x4200_0000 {
            return;
        }
        MENU_RESERVE.set(mem, [request, u32::MAX]);
    }

    /// `Sg_ToggleEnableTkAnti()` (`0x0083b100`): flips [`TK_ANTI_ENABLE`].
    pub fn toggle_enable_tk_anti(mem: &mut impl Mem) {
        toggle(mem, TK_ANTI_ENABLE);
    }

    /// The map id table (`g_ChrIdMapTable`).
    pub const ID_MAP_TABLE: Global<u32> = Global::new(0x0130_19d8);

    /// `Sg_IdGetMapTable(i)` (`0x0083e960`).
    pub fn id_get_map_table(mem: &impl Mem, i: u32) -> u32 {
        mem.read(elem(mem, ID_MAP_TABLE, i, 4))
    }

    /// `Sg_IdSetMapTable(i, v)` (`0x0083e9e0`); `EAX` ends as `i`.
    pub fn id_set_map_table(mem: &mut impl Mem, i: u32, v: u32) -> u32 {
        let at = elem(mem, ID_MAP_TABLE, i, 4);
        mem.write(at, v);
        i
    }

    /// The map id events (`Sg_IdEventInitAll` clears them).
    pub const ID_EVENTS: Global<[u32; 20]> = Global::new(0x0130_1a98);

    /// `Sg_IdEventInitAll()` (`0x0083e8d0`): the events, both effect table
    /// sets, then `Sg_IdSetEffectTableIndex(0)`.
    pub fn id_event_init_all(mem: &mut impl Mem) {
        ID_EVENTS.set(mem, [0; 20]);
        let t = ID_EFFECT_TABLE.ptr(mem).addr;
        mem.write(t, [0u8; 8]);
        id_set_effect_table_index(mem, 0);
    }

    /// The debug print ring: head, tail (`eidprintfPrint`).
    pub const EID_PRINT_HEAD: Global<u32> = Global::new(0x010e_787c);
    /// See [`EID_PRINT_HEAD`].
    pub const EID_PRINT_TAIL: Global<u32> = Global::new(0x010e_7878);

    /// `eidprintfPrint()` (`0x0076b8d0`): empties the ring of 256 queued
    /// messages (each printed through the empty `dbgPrintf`).
    pub fn eid_printf_print(mem: &mut impl Mem) {
        let tail = EID_PRINT_TAIL.get(mem);
        let mut head = EID_PRINT_HEAD.get(mem);
        while head != tail {
            head = head.wrapping_add(1);
            if head.cast_signed() >= 0x100 {
                head = 0;
            }
            EID_PRINT_HEAD.set(mem, head);
        }
    }

    // The memory manager's blocks: a 16-byte header before the data, the
    // next header at `+0`, the previous at `+4`, the owner bits in the low
    // nibble of `+0xc` (0 free); a debug name id in the low 11 bits of the
    // `u16` at `-6` from the data.
    /// The memory manager's first block (`Sg_MemCache_SerchFree`).
    pub const MEM_FIRST: Global<u32> = Global::new(0x0130_1978);
    /// The return address recorded with allocations (`Sg_MemSetRtAddr`).
    pub const MEM_RT_ADDR: Global<u32> = Global::new(0x0130_19b0);

    /// `Sg_MemIsFree(p)` (`0x0083c320`): 1 when the block of data `p` has
    /// no owner, 0 for an owned one or null.
    pub fn mem_is_free(mem: &impl Mem, p: u32) -> u32 {
        if p == 0 {
            return 0;
        }
        u32::from(mem.read::<u8>(p.wrapping_sub(4)).trailing_zeros() >= 4)
    }

    /// `Sg_MemGetSum(header)` (`0x0083c0b0`): the 16-bit sum of the
    /// block's data halfwords, in `AX`.
    pub fn mem_get_sum(mem: &impl Mem, header: u32) -> u16 {
        let next: u32 = mem.read(header);
        let n = next.wrapping_sub(header).wrapping_sub(0x10) >> 1;
        let mut sum = 0u16;
        for k in 0..n {
            sum = sum.wrapping_add(mem.read(header.wrapping_add(0x10 + 2 * k)));
        }
        sum
    }

    /// The debug name of block data `p`: its id's 32-byte slot in the
    /// debug name area at `0x05fb0000`, or none.
    fn mem_name(mem: &impl Mem, p: u32) -> Option<u32> {
        if IS_DEBUG.get(mem) == 0 || p == 0 {
            return None;
        }
        let id = u32::from(mem.read::<u16>(p.wrapping_sub(6))) & 0x7ff;
        (id != 0).then(|| (id + 0x002f_d800) << 5)
    }

    /// `Sg_MemGetStr(p)` (`0x0083c070`): the block's debug name, or the
    /// empty string ([`FUNC_NAME`]).
    pub fn mem_get_str(mem: &impl Mem, p: u32) -> u32 {
        mem_name(mem, p).unwrap_or_else(|| FUNC_NAME.ptr(mem).addr)
    }

    /// `Sg_MemClearStr(p)` (`0x0083bab0`): empties the block's debug name
    /// and drops its id.
    pub fn mem_clear_str(mem: &mut impl Mem, p: u32) {
        if let Some(at) = mem_name(mem, p) {
            mem.write(at, 0u8);
            let w: u16 = mem.read(p.wrapping_sub(6));
            mem.write(p.wrapping_sub(6), w & 0xf800);
        }
    }

    /// `Sg_MemSetRtAddr(a)` (`0x0083c560`): `(a - 8) / 4` in 23 bits (an
    /// arithmetic shift); returns it.
    pub fn mem_set_rt_addr(mem: &mut impl Mem, a: u32) -> u32 {
        let v = (a.wrapping_sub(8).cast_signed() >> 2).cast_unsigned() & 0x7f_ffff;
        MEM_RT_ADDR.set(mem, v);
        v
    }

    /// `Sg_MemCache_SerchFree(p)` (`0x0083b770`): the data of the first
    /// free block before block data `p` (in list order from the first
    /// block) that is bigger than `p`'s block, else 0.
    pub fn mem_cache_serch_free(mem: &impl Mem, p: u32) -> u32 {
        let q = p.wrapping_sub(0x10);
        let size = |b: u32| mem.read::<u32>(b).wrapping_sub(b).wrapping_sub(0x10);
        let mut b = MEM_FIRST.get(mem);
        while b != q {
            if mem.read::<u8>(b.wrapping_add(0xc)).trailing_zeros() >= 4 && size(b) > size(q) {
                return b.wrapping_add(0x10);
            }
            b = mem.read(b.wrapping_add(4));
        }
        0
    }

    /// The PC port's stand-ins for PS2 assembly routines it never
    /// implemented (`SPAD_CALL`, `rgb32_to_24_asm`, `sa_printf_set`,
    /// `sa_meth01dec_ex`, `sa_meth01dec2_ex`): only a warning through the
    /// empty `dbgPrintf`. The two decoders return 0.
    #[must_use]
    pub const fn not_implemented() -> u32 {
        0
    }
}
