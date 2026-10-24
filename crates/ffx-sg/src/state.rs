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

    /// The number of entries in [`ID_TABLE`] (`i32`).
    pub const ID_COUNT: Global<i32> = Global::new(0x0130_1a88);

    /// `Sg_IdFindTriangle(p)` (`0x0083de10`): the id triangle under point
    /// `p` (`f32` x, y, z). A triangle holds `p` when, for each of its
    /// edges `(a, b)` taken in order, `(b.x - a.x) p.z + (a.z - b.z) p.x +
    /// a.x b.z - b.x a.z` is not above 0 (the integer products stored as
    /// `f32`, the sums at 53 bits; NaN counts as inside). Of those, the
    /// one whose height ([`id_get_height`], truncated) is lowest while not
    /// below `p.y` wins (the first among equals); without one, the last
    /// triangle that held `p`; else -1 (also without id data).
    pub fn id_find_triangle(mem: &mut impl Mem, p: u32) -> i32 {
        use ffx_bridge::x87::{self, Reg};
        let table = ID_TABLE.get(mem);
        if table == 0 {
            return -1;
        }
        let verts = ID_VERTICES.get(mem);
        let px: f32 = mem.read(p);
        let pz: f32 = mem.read(p.wrapping_add(8));
        let f = |v: i32| f32::from_bits(x87::store(f64::from(v)));
        // an edge's side of the point: not above 0 inside
        let inside = |a: (i32, i32), b: (i32, i32)| {
            let e = Reg::load(f(b.0.wrapping_sub(a.0))).mul_m32(pz)
                + Reg::load(f(a.1.wrapping_sub(b.1))).mul_m32(px);
            let e = e
                .add_m32(f(a.0.wrapping_mul(b.1)))
                .sub_m32(f(b.0.wrapping_mul(a.1)));
            e.value().partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        };
        let (mut best, mut best_h, mut last) = (-1i32, 0i32, -1i32);
        let n = ID_COUNT.get(mem);
        for i in 0..n.max(0) {
            let e = table.wrapping_add(i.cast_unsigned() << 4);
            let [a, b, c] = [0u32, 2, 4].map(|k| {
                let k = i32::from(mem.read::<i16>(e.wrapping_add(k)));
                let v = verts.wrapping_add((k << 3).cast_unsigned());
                (
                    i32::from(mem.read::<i16>(v)),
                    i32::from(mem.read::<i16>(v.wrapping_add(4))),
                )
            });
            if !(inside(a, b) && inside(b, c) && inside(c, a)) {
                continue;
            }
            last = i;
            let h = id_get_height(mem, p, i.cast_unsigned());
            let c = x87::ftol(mem, f64::from(h)).cast_signed();
            let y: f32 = mem.read(p.wrapping_add(4));
            if f64::from(f(c)) < f64::from(y) {
                continue;
            }
            if best == -1 || best_h > c {
                best = i;
                best_h = c;
            }
        }
        if best == -1 { last } else { best }
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

    /// `Sg_IdChrMove`'s callees: `Sg_GetIgnoreID`, `Sg_IdFindTriangle(p)`,
    /// `Sg_IdWalk(chr, p, i)`, `Sg_IdSlideEdge(from, to)`,
    /// `Sg_IdGetNormal(out, i)`, `Sg_IdGetHeight(p, i)` (`ST0`).
    pub const GET_IGNORE_ID: u32 = 0x0082_d830;
    pub const ID_FIND_TRIANGLE: u32 = 0x0083_de10;
    pub const ID_WALK: u32 = 0x0083_d960;
    pub const ID_SLIDE_EDGE: u32 = 0x0083_e180;
    pub const ID_GET_NORMAL: u32 = 0x0083_e130;
    pub const ID_GET_HEIGHT: u32 = 0x0083_e040;
    /// Its data: a step's word `Sg_IdWalk` reads (cleared before each
    /// walk), the scaled value of `0x00c4deb0` it keeps, the map whose
    /// player stands 1 lower (`g_GraphicMapId`), and its constants (10 and
    /// 1 as `f64`, -1 as `f32`).
    pub const ID_WALK_STEP: Global<u32> = Global::new(0x0130_1ae8);
    pub const ID_REACH: Global<f32> = Global::new(0x00c4_deb0);
    pub const ID_REACH_SCALED: Global<f32> = Global::new(0x0130_1af4);
    pub const GRAPHIC_MAP_ID: Global<u32> = Global::new(0x0133_0844);
    pub const ID_TEN: Global<f64> = Global::new(0x00b9_22b0);
    pub const ID_ONE: Global<f64> = Global::new(0x00b9_2248);
    pub const ID_MINUS_ONE: Global<f32> = Global::new(0x00b9_2340);
    /// The character's fields it uses: position (`+0xc`, `+0x10`,
    /// `+0x14`), the id-space position (`+0x3c`), the move (`+0x4c`,
    /// `+0x54`), the ground normal (`+0x7c`), the height (`+0x16c`), the
    /// flags (`+0x194`, 0x200: the id map ignores it), the triangle
    /// (`i16 +0x824`) and its word (`+0x828`, `0xfffe0000` for none).
    pub const CHR_TRIANGLE: u32 = 0x824;
    pub const CHR_TRIANGLE_WORD: u32 = 0x828;

    /// `Sg_IdChrMove(chr)` (`0x0083e5f0`, name ours; the push-apart pass
    /// `0x0083d3b0` calls it per character): the character's move over the
    /// id (collision) map. Without one, the move is added to x and z and
    /// the ground reset (normal (0, -1, 0), height 0). Otherwise in the
    /// map's scale ([`ID_ZOOM`]): the triangle found when unknown (or none
    /// when the map ignores the character), the target point `p` (the
    /// position plus the move, y 0); on no triangle the move is added (times 10 unless
    /// ignored); else up to four `Sg_IdWalk` steps (sliding along crossed
    /// edges), then the triangle's word, normal and height (1 lower for
    /// the player on map 0x12a) and the position `p` back out of the
    /// map's scale. `p` is the frame's point (16 bytes the callees reach;
    /// the walk changes it); `fcall` runs `Sg_IdGetHeight` (its `ST0`).
    #[allow(clippy::too_many_lines)] // one walk, as the original's
    pub fn id_chr_move<H: ffx_bridge::Host>(
        h: &mut H,
        chr: u32,
        p: u32,
        fcall: &mut impl FnMut(&mut H, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{self, Reg};
        let f32_at = |h: &H, o: u32| h.read::<f32>(chr.wrapping_add(o));
        let set = |h: &mut H, o: u32, v: f32| h.write(chr.wrapping_add(o), v);
        let triangle = |h: &H| h.read::<u16>(chr.wrapping_add(CHR_TRIANGLE));
        if ID_TABLE.get(h) == 0 {
            set_word(h, chr, 0xfffe_0000);
            let x = Reg::load(f32_at(h, 0x4c)).add_m32(f32_at(h, 0xc)).store();
            set(h, 0xc, x);
            let z = Reg::load(f32_at(h, 0x54)).add_m32(f32_at(h, 0x14)).store();
            set(h, 0x14, z);
            set(h, 0x16c, 0.0);
            set(h, 0x7c, 0.0);
            let down = x87::round_trip(ID_MINUS_ONE.get(h));
            set(h, 0x80, down);
            set(h, 0x84, 0.0);
            return;
        }
        let scale = ID_ZOOM.get(h);
        let reach = Reg::load(ID_REACH.get(h)).mul_m32(scale).store();
        ID_REACH_SCALED.set(h, reach);
        // x and y as `fmul st, st(n)`, z as `fmulp st(3), st` (scale * z)
        let scaled = [
            Reg::load(f32_at(h, 0xc)).mul_m32(scale).store(),
            Reg::load(f32_at(h, 0x10)).mul_m32(scale).store(),
            Reg::load(scale).mul_m32(f32_at(h, 0x14)).store(),
        ];
        for (k, v) in (0u32..).zip(scaled) {
            set(h, 0x3c + 4 * k, v);
        }
        h.write(p, scaled);
        h.write(p + 0xc, 0u32);
        let call = |h: &mut H, f: u32, args: &[u32]| {
            let f = h.rebase(f);
            h.call(f, args)
        };
        let flags = |h: &H| h.read::<u32>(chr.wrapping_add(0x194));
        let mut ignored = false;
        if call(h, GET_IGNORE_ID, &[]) != 0 && flags(h) & 0x200 != 0 {
            ignored = true;
            h.write(chr.wrapping_add(CHR_TRIANGLE), u16::MAX);
        }
        if triangle(h) == u16::MAX {
            let [lo, hi, _, _] = call(h, ID_FIND_TRIANGLE, &[p]).to_le_bytes();
            h.write(chr.wrapping_add(CHR_TRIANGLE), u16::from_le_bytes([lo, hi]));
        }
        let id = triangle(h);
        let s = Reg::load(scale);
        let px = (Reg::load(f32_at(h, 0x4c)) * s).add_m32(h.read(p)).store();
        h.write(p, px);
        let pz = (s.mul_m32(f32_at(h, 0x54))).add_m32(h.read(p + 8)).store();
        h.write(p + 8, pz);
        h.write(p + 4, 0.0f32);
        if id == u16::MAX {
            let mx = Reg::load(f32_at(h, 0x4c));
            if ignored {
                let x = mx.add_m32(f32_at(h, 0xc)).store();
                set(h, 0xc, x);
                let z = Reg::load(f32_at(h, 0x54)).add_m32(f32_at(h, 0x14)).store();
                set(h, 0x14, z);
            } else {
                let ten = Reg::constant(ID_TEN.get(h));
                let x = (mx * ten).add_m32(f32_at(h, 0xc)).store();
                set(h, 0xc, x);
                let z = ten
                    .mul_m32(f32_at(h, 0x54))
                    .add_m32(f32_at(h, 0x14))
                    .store();
                set(h, 0x14, z);
            }
            return;
        }
        if !ignored {
            let start = id;
            let mut walked = false;
            for _ in 0..4 {
                let i = i32::from(triangle(h).cast_signed()).cast_unsigned();
                ID_WALK_STEP.set(h, 0);
                if call(h, ID_WALK, &[chr, p, i]) == 0 {
                    walked = true;
                    break;
                }
                if call(h, GET_IGNORE_ID, &[]) != 0 && flags(h) & 0x200 != 0 {
                    walked = true;
                    break;
                }
                if triangle(h) != u16::MAX {
                    call(h, ID_SLIDE_EDGE, &[chr.wrapping_add(0x3c), p]);
                }
                h.write(chr.wrapping_add(CHR_TRIANGLE), start);
            }
            if !walked {
                return;
            }
        }
        let c = triangle(h);
        let table = ID_TABLE.get(h);
        let i = i32::from(c.cast_signed()).cast_unsigned();
        if c != u16::MAX && table != 0 {
            let w = h.read::<u32>(table.wrapping_add(i.wrapping_mul(16)).wrapping_add(0xc));
            set_word(h, chr, w);
        } else {
            set_word(h, chr, 0xfffe_0000);
        }
        call(h, ID_GET_NORMAL, &[chr.wrapping_add(0x7c), i]);
        let i = i32::from(triangle(h).cast_signed()).cast_unsigned();
        let f = h.rebase(ID_GET_HEIGHT);
        let height = Reg::constant(fcall(h, f, &[p, i]));
        let height = (height / s).store();
        set(h, 0x16c, height);
        if GRAPHIC_MAP_ID.get(h) == 0x12a && h.read::<u16>(chr) == 1 {
            let lower = (Reg::load(height) - Reg::constant(ID_ONE.get(h))).store();
            set(h, 0x16c, lower);
        }
        let x = (Reg::load(h.read(p)) / s).store();
        set(h, 0xc, x);
        let z = s.divr_m32(h.read(p + 8)).store();
        set(h, 0x14, z);
    }

    fn set_word(h: &mut impl Mem, chr: u32, w: u32) {
        h.write(chr.wrapping_add(CHR_TRIANGLE_WORD), w);
    }

    /// `Sg_CanSpeedUp`'s callees: `MagicFile_GetMagicId`,
    /// `graphicLevelGetNameHash`, `getScenerioFlag` (`u16`), `MsBattleCheck`,
    /// `graphicIsVideoPlaying`, `movie_have_camera`, `getUCStatus`,
    /// `getInterValefor` (`u8`), `Atel_WindowsBusy`, `SpdCtrl_IsSpeedingUp`.
    pub const MAGIC_ID: u32 = 0x009d_a3b0;
    pub const LEVEL_NAME_HASH: u32 = 0x0064_1da0;
    pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
    pub const BATTLE_CHECK: u32 = 0x0078_0c90;
    pub const VIDEO_PLAYING: u32 = 0x0064_1cf0;
    pub const MOVIE_HAVE_CAMERA: u32 = 0x0076_efc0;
    pub const GET_UC_STATUS: u32 = 0x0082_2460;
    pub const GET_INTER_VALEFOR: u32 = 0x0078_7340;
    pub const WINDOWS_BUSY: u32 = 0x0086_c8b0;
    pub const IS_SPEEDING_UP: u32 = 0x006f_7330;
    /// Its globals: the frame wait by time (`g_WaitFramesByTime + 4`), the
    /// map number, the movie fade, three words that each block it.
    pub const WAIT_BY_TIME: Global<u32> = Global::new(0x0112_a00c);
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
    pub const MOVIE_FADING: Global<u32> = Global::new(0x0134_0824);
    pub const SPEED_UP_BLOCKS: [u32; 3] = [0x012f_20a4, 0x012f_20a8, 0x012f_20ac];

    /// `Sg_CanSpeedUp()` (`0x0081fd30`, name ours; the world update
    /// `0x00821e80` asks it before its time-rate step): 1 while
    /// `SpdCtrl_IsSpeedingUp`; else 0 while a video plays without the
    /// movie's camera (nor a timed frame wait), in some magic effects
    /// (`0x14a`, `0x14c`, `0x17d` on map 0xd0; `0x132`, `0xa9`, `0x1b5`,
    /// `0x1b6`, `0xcd`), at listed story points of some maps and levels
    /// (scenario flag, battle, level name hash, the script's windows,
    /// Valefor's flags), while the movie fades, in eight levels during
    /// `getUCStatus`, or while one of [`SPEED_UP_BLOCKS`] is set; 1
    /// otherwise. Every callee runs first, in the original's order.
    #[allow(clippy::too_many_lines)] // one decision, as the original's
    pub fn can_speed_up(h: &mut impl ffx_bridge::Host) -> u32 {
        fn call(h: &mut impl ffx_bridge::Host, f: u32) -> u32 {
            let f = h.rebase(f);
            h.call(f, &[])
        }
        let magic = call(h, MAGIC_ID);
        let hash = call(h, LEVEL_NAME_HASH);
        let [lo, hi, _, _] = call(h, GET_SCENERIO_FLAG).to_le_bytes();
        let flag = u16::from_le_bytes([lo, hi]);
        let battle = call(h, BATTLE_CHECK);
        let video = call(h, VIDEO_PLAYING);
        let movie_camera = call(h, MOVIE_HAVE_CAMERA) != 0 || WAIT_BY_TIME.get(h) != 0;
        let uc = call(h, GET_UC_STATUS);
        call(h, GET_INTER_VALEFOR);
        let busy = call(h, WINDOWS_BUSY);
        if call(h, IS_SPEEDING_UP) != 0 {
            return 1;
        }
        if video != 0 && !movie_camera {
            return 0;
        }
        let map = CURRENT_MAP.get(h);
        if matches!(magic, 0x14a | 0x14c | 0x17d) && map == 0xd0 {
            return 0;
        }
        if matches!(magic, 0x132 | 0xa9 | 0x1b5 | 0x1b6 | 0xcd) {
            return 0;
        }
        if map == 0xa4 {
            if hash == 0x1cd6 && flag == 0x5be {
                return 0;
            }
        } else {
            if matches!(map, 0x5f | 0x78 | 0x7f) && battle == 0 && flag == 0x2de {
                return 0;
            }
            if map == 0x3b && hash == 0x56d && battle == 0 && flag == 0x309 {
                return 0;
            }
        }
        if MOVIE_FADING.get(h) != 0 {
            return 0;
        }
        // `None`: on to the level checks; `Some(blocked)`: decided here
        let story = match hash {
            0x24f9 => {
                let valefor = call(h, GET_INTER_VALEFOR);
                if valefor & 4 != 0 && call(h, GET_INTER_VALEFOR) & 0x80 == 0 {
                    return 0;
                }
                None
            }
            0x1b4f if busy == 2 => Some(battle == 0),
            0x7b4 if flag == 0xc3f => Some(battle == 0),
            _ => None,
        };
        let blocked = match story {
            Some(blocked) => blocked,
            None if battle == 0 && hash == 0x1c99 => true,
            None if hash == 0x1ca => flag == 0xb68,
            None => hash == 0x6e1 && battle != 0 && magic == u32::MAX,
        };
        if blocked {
            return 0;
        }
        if uc != 0
            && matches!(
                hash,
                0x1b84 | 0x1f7 | 0x1899 | 0x1d21 | 0xc04 | 0x1e48 | 0xbae | 0x1c45
            )
        {
            return 0;
        }
        if SPEED_UP_BLOCKS
            .iter()
            .any(|&g| Global::<u32>::new(g).get(h) != 0)
        {
            return 0;
        }
        1
    }

    /// `Sg_MapInit`'s callees and data: `yiReset`, `yiMapSetup(objects,
    /// textures)`, `Sg_PcRead(path)`, `Sg_IdFixupAndSet(id)`, `dbgPrintf`,
    /// the C runtime's `sprintf` (through its import slot); the debug line
    /// (`g_DebugLine`, the shade map's directory), the debug map list's
    /// flag, the chosen map's directory and name (pointers).
    pub const YI_RESET: u32 = 0x0090_9e60;
    pub const YI_MAP_SETUP: u32 = 0x0090_9680;
    pub const PC_READ: u32 = 0x0083_ae30;
    pub const ID_FIXUP_AND_SET: u32 = 0x0084_4bc0;
    pub const SPRINTF_SLOT: Global<u32> = Global::new(0x00b0_c3dc);
    pub const DEBUG_LINE: u32 = 0x023c_4000;
    pub const MAP_SHADE: Global<u8> = Global::new(0x0130_34b9);
    pub const MAP_DIRS: u32 = 0x0130_3580;
    pub const MAP_DIR: Global<u32> = Global::new(0x0130_3640);
    pub const MAP_NAME: Global<u32> = Global::new(0x0130_3644);
    /// The formats: `".../master/%s/%s/bin"`, `".../sugi/data/%s/%s/map"`,
    /// `".../btlmaster/%s/%s/bin"`, `"shd=%s\ndir=%s\n"`,
    /// `"%s/mapobjdata.vpb"`, `"%s/maptexdata.txb"`, `"%s/ffxmap.id"`.
    pub const FMT_MASTER: u32 = 0x00b5_a9a8;
    pub const FMT_SUGI: u32 = 0x00b5_a984;
    pub const FMT_BATTLE: u32 = 0x00b5_a9d8;
    pub const FMT_SHADE_LOG: u32 = 0x00b5_a9c8;
    pub const FMT_OBJECTS: u32 = 0x00b5_a9fc;
    pub const FMT_TEXTURES: u32 = 0x00b5_aa10;
    pub const FMT_IDS: u32 = 0x00b5_aa24;
    /// `Sg_MapInit`'s three 0x40-byte path buffers, as its frame holds them
    /// (`work`: the textures path, the map directory, the objects path).
    pub const MAP_INIT_WORK: u32 = 0xc0;

    /// `Sg_MapInit()` (`0x00844d10`, from `WMainProc` and `Sg_Test`):
    /// `yiReset`, the debug line cleared, the selected map's directory and
    /// name into [`MAP_DIR`] and [`MAP_NAME`]; the map directory from the
    /// selection: 0 `master`, 2 `btlmaster`, 1 `sugi` (with
    /// [`MAP_SHADE`] the `sugi` path goes into the debug line, the map is
    /// `master/<name[..4]>/<name[..6]>`, and `dbgPrintf` logs both; any
    /// other selection leaves the buffer as it was). Then `yiMapSetup` with
    /// the objects and textures paths, and `Sg_IdFixupAndSet` with the id
    /// file `Sg_PcRead` returns (from the debug line's directory first when
    /// set). `work` holds the three paths ([`MAP_INIT_WORK`] bytes).
    pub fn map_init(h: &mut impl ffx_bridge::Host, work: u32) {
        fn call(h: &mut impl ffx_bridge::Host, f: u32, args: &[u32]) -> u32 {
            let f = h.rebase(f);
            h.call(f, args)
        }
        let (textures, dir, objects) = (work, work + 0x40, work + 0x80);
        call(h, YI_RESET, &[]);
        let line = h.rebase(DEBUG_LINE);
        h.write(line, 0u8);
        let select = MAP_DIR_SELECT.get(h);
        let base = select.wrapping_mul(0x40);
        let map_dir = h.rebase(MAP_DIRS).wrapping_add(base);
        let map_name = h.rebase(MAP_NAMES.sym).wrapping_add(base);
        MAP_DIR.set(h, map_dir);
        MAP_NAME.set(h, map_name);
        let sprintf = SPRINTF_SLOT.get(h);
        let format = match map_get_dir_select(h) {
            0 => Some(FMT_MASTER),
            2 => Some(FMT_BATTLE),
            1 if MAP_SHADE.get(h) == 0 => Some(FMT_SUGI),
            1 => {
                let f = h.rebase(FMT_SUGI);
                h.call(sprintf, &[line, f, map_dir, map_name]);
                // the name twice, cut to 4 and to 6 characters
                let name = MAP_NAME.get(h);
                for (to, cut) in [(objects, 4), (textures, 6)] {
                    for k in 0..0x1_0000 {
                        let c: u8 = h.read(name.wrapping_add(k));
                        h.write(to + k, c);
                        if c == 0 {
                            break;
                        }
                    }
                    h.write(to + cut, 0u8);
                }
                let f = h.rebase(FMT_MASTER);
                h.call(sprintf, &[dir, f, objects, textures]);
                let f = h.rebase(FMT_SHADE_LOG);
                call(h, DBG_PRINTF, &[f, line, dir]);
                None
            }
            _ => None,
        };
        if let Some(format) = format {
            let f = h.rebase(format);
            h.call(sprintf, &[dir, f, map_dir, map_name]);
        }
        let f = h.rebase(FMT_OBJECTS);
        h.call(sprintf, &[objects, f, dir]);
        let f = h.rebase(FMT_TEXTURES);
        h.call(sprintf, &[textures, f, dir]);
        call(h, YI_MAP_SETUP, &[objects, textures]);
        let ids = h.rebase(FMT_IDS);
        let mut id = 0;
        if h.read::<u8>(line) != 0 {
            h.call(sprintf, &[objects, ids, line]);
            id = call(h, PC_READ, &[objects]);
        }
        if id == 0 {
            h.call(sprintf, &[objects, ids, dir]);
            id = call(h, PC_READ, &[objects]);
        }
        call(h, ID_FIXUP_AND_SET, &[id]);
    }

    /// `Sg_EffectReadPoll`'s state: the battle effect read's and the event
    /// effect read's frame counters (`i32`, -1 idle;
    /// `g_BtlEffectReadClear`, `g_BtlEventEffectClear`) and whether each
    /// file was taken preloaded (`g_BtlEffectPreloaded` and the word
    /// after it).
    pub const EFFECT_READ_FRAMES: Global<i32> = Global::new(0x00c4_21e8);
    pub const EVENT_READ_FRAMES: Global<i32> = Global::new(0x00c4_21e4);
    pub const EFFECT_PRELOADED: Global<u32> = Global::new(0x012f_b8b8);
    pub const EVENT_PRELOADED: Global<u32> = Global::new(0x012f_b8bc);
    /// Its callees: `graphicIsMagicFileAsyncLoadAvailable`,
    /// `GetU32_01940ab4`, the PRX checks `0x006793d0` (state 3,
    /// `Compare_006f9b70`) and `0x006793b0` (`PhyreFIOS_LoadPrxIdle`'s
    /// thunk: > 0 an error, 2 out of kernel memory), `MsEffectReadError`,
    /// `dbgPrintf`, `IsLoadTexture`, `MsEffectReadComplete`,
    /// `0x006420d0` (the magic file loaded), `MsEventEffectReadComplete`,
    /// `MsEffectReadStatInit`.
    pub const ASYNC_LOAD_AVAILABLE: u32 = 0x0064_1c60;
    pub const GETU32_01940AB4: u32 = 0x009d_a2c0;
    pub const PRX_READY: u32 = 0x0067_93d0;
    pub const PRX_ERROR: u32 = 0x0067_93b0;
    pub const EFFECT_READ_ERROR: u32 = 0x0078_81b0;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const IS_LOAD_TEXTURE: u32 = 0x009d_a280;
    pub const EFFECT_READ_COMPLETE: u32 = 0x0078_8110;
    pub const MAGIC_FILE_LOADED: u32 = 0x0064_20d0;
    pub const EVENT_EFFECT_READ_COMPLETE: u32 = 0x0078_8d10;
    pub const EFFECT_READ_STAT_INIT: u32 = 0x0078_81c0;
    /// `"MagicFile: No kernel memory.\n"`.
    pub const NO_KERNEL_MEMORY: u32 = 0x00b5_9084;

    /// `Sg_EffectReadPoll()` (`0x00822250`, name ours; from
    /// `0x00821e80`): one frame of the battle effect read and the event
    /// effect read. Each running read (frame counter >= 0) counts a frame;
    /// without the async magic file load the read completes at once.
    /// Otherwise it waits for the PRX (state 3), reports a PRX error (the
    /// effect read only), and without `IsLoadTexture` resets the read
    /// state; with it, a preloaded file (async 1) completes first, and the
    /// read ends once the magic file is loaded, completing then unless it
    /// already did. An ended read's counter goes to -1, its preloaded flag
    /// to 0. Returns `eax` as the original leaves it: the event counter
    /// when that read is idle, else its last call's result.
    pub fn effect_read_poll(h: &mut impl ffx_bridge::Host) -> u32 {
        fn call(h: &mut impl ffx_bridge::Host, f: u32) -> u32 {
            let f = h.rebase(f);
            h.call(f, &[])
        }
        let available = call(h, ASYNC_LOAD_AVAILABLE);
        let frames = EFFECT_READ_FRAMES.get(h);
        if frames >= 0 {
            EFFECT_READ_FRAMES.set(h, frames.wrapping_add(1));
            let ended = if available == 0 || call(h, GETU32_01940AB4) != 0 {
                call(h, EFFECT_READ_COMPLETE);
                true
            } else if call(h, PRX_READY) == 0 {
                false
            } else {
                let error = call(h, PRX_ERROR).cast_signed();
                if error > 0 {
                    call(h, EFFECT_READ_ERROR);
                    if error == 2 {
                        let (f, text) = (h.rebase(DBG_PRINTF), h.rebase(NO_KERNEL_MEMORY));
                        h.call(f, &[text]);
                    }
                }
                if call(h, IS_LOAD_TEXTURE) == 0 {
                    call(h, EFFECT_READ_STAT_INIT);
                    true
                } else {
                    if available == 1 && EFFECT_PRELOADED.get(h) == 0 {
                        call(h, EFFECT_READ_COMPLETE);
                        EFFECT_PRELOADED.set(h, available);
                    }
                    if call(h, MAGIC_FILE_LOADED) == 0 {
                        false
                    } else {
                        if EFFECT_PRELOADED.get(h) == 0 {
                            call(h, EFFECT_READ_COMPLETE);
                        }
                        true
                    }
                }
            };
            if ended {
                EFFECT_PRELOADED.set(h, 0);
                EFFECT_READ_FRAMES.set(h, -1);
            }
        }
        let frames = EVENT_READ_FRAMES.get(h);
        if frames < 0 {
            return frames.cast_unsigned();
        }
        let frames = frames.wrapping_add(1);
        EVENT_READ_FRAMES.set(h, frames);
        let last = if available == 0 {
            call(h, EVENT_EFFECT_READ_COMPLETE)
        } else {
            let r = call(h, PRX_READY);
            if r == 0 {
                return r;
            }
            if call(h, IS_LOAD_TEXTURE) == 0 {
                call(h, EFFECT_READ_STAT_INIT)
            } else {
                if available == 1 && EVENT_PRELOADED.get(h) == 0 {
                    call(h, EVENT_EFFECT_READ_COMPLETE);
                    EVENT_PRELOADED.set(h, available);
                }
                let r = call(h, MAGIC_FILE_LOADED);
                if r == 0 {
                    return r;
                }
                if EVENT_PRELOADED.get(h) == 0 {
                    call(h, EVENT_EFFECT_READ_COMPLETE)
                } else {
                    r
                }
            }
        };
        EVENT_READ_FRAMES.set(h, -1);
        EVENT_PRELOADED.set(h, 0);
        last
    }

    /// The free camera's distance now (`f32`), easing toward
    /// [`FREE_CAM_DIST`].
    pub const FREE_CAM_DIST_NOW: Global<f32> = Global::new(0x0130_1bb4);
    /// The free camera's offset from the player (`f32` x, y, z; y is
    /// [`FREE_CAM_HEIGHT`]).
    pub const FREE_CAM_OFFSET: u32 = 0x00c4_dee0;
    /// `Sg_CameraSetPlayer`'s constants: the easing 0.125 and the height
    /// 10 (`f64`), the yaw bound pi/2 as `f64` (compared) and `f32`
    /// (stored), and their negatives.
    pub const EASE: Global<f64> = Global::new(0x00b9_2200);
    pub const PLAYER_HEIGHT: Global<f64> = Global::new(0x00b9_22b0);
    pub const YAW_MAX: Global<f64> = Global::new(0x00b5_82e8);
    pub const YAW_MAX_F32: Global<f32> = Global::new(0x00b5_99c0);
    pub const YAW_MIN: Global<f64> = Global::new(0x00b5_a490);
    pub const YAW_MIN_F32: Global<f32> = Global::new(0x00b5_a488);
    /// `Ch_GetFieldMode(chr)` (`0x0082ab90`): called for the player, its
    /// result unused.
    pub const CH_GET_FIELD_MODE: u32 = 0x0082_ab90;

    /// `Sg_CameraSetPlayer()` (`0x0083f2e0`, from `Sg_MainLoop`): the free
    /// camera behind the player. The distance eases an eighth of the way
    /// toward [`FREE_CAM_DIST`]; the reference point is the player's
    /// position (`Sg_GetPlayerPos`; 0 without a player) 10 down, plus
    /// [`FREE_CAM_OFFSET`]; the yaw is held to +-pi/2 and the distance to
    /// at least 1 (a NaN passes both); the position is the reference plus
    /// `-dist * (sin p cos y, sin y, cos p cos y)` with the CRT's `sin` and
    /// `cos` (`trig`, `f32` bits in and out; yaw = [`FREE_CAM_YAW`], p =
    /// [`FREE_CAM_PITCH`]). Ends in `Sg_CameraPutRects`, whose `eax` it
    /// returns.
    #[allow(clippy::similar_names)] // the sines and cosines
    pub fn camera_set_player(
        h: &mut impl ffx_bridge::Host,
        trig: &dyn Fn(ffx_battle::pos::bridge::Trig, u32) -> u32,
    ) -> u32 {
        use crate::camera::bridge::{CAMERA_POS, CAMERA_REF};
        use ffx_battle::pos::bridge::Trig;
        use ffx_bridge::x87::{self, Reg};
        let now = Reg::load(FREE_CAM_DIST_NOW.get(h));
        let step = Reg::load(FREE_CAM_DIST.get(h));
        let eased = now - (now - step) * Reg::constant(EASE.get(h));
        FREE_CAM_DIST_NOW.set(h, eased.store());
        let r = h.rebase(CAMERA_REF.sym);
        if get_player_pos(h, r) != 0 {
            CAMERA_REF.set(h, [0.0; 3]);
        }
        let player = get_player(h);
        if player != 0 {
            let f = h.rebase(CH_GET_FIELD_MODE);
            h.call(f, &[player]);
        }
        let [x, y, z] = CAMERA_REF.get(h);
        let y = (Reg::load(y) - Reg::constant(PLAYER_HEIGHT.get(h))).store();
        let offset: [f32; 3] = h.read(h.rebase(FREE_CAM_OFFSET));
        let reference = [
            Reg::load(x).add_m32(offset[0]).store(),
            Reg::load(y).add_m32(offset[1]).store(),
            Reg::load(z).add_m32(offset[2]).store(),
        ];
        CAMERA_REF.set(h, reference);
        let yaw = FREE_CAM_YAW.get(h);
        if x87::load(yaw.to_bits()) > YAW_MAX.get(h) {
            FREE_CAM_YAW.set(h, YAW_MAX_F32.get(h));
        }
        let mut yaw = FREE_CAM_YAW.get(h);
        if x87::load(yaw.to_bits()) < YAW_MIN.get(h) {
            FREE_CAM_YAW.set(h, YAW_MIN_F32.get(h));
            yaw = FREE_CAM_YAW.get(h);
        }
        if x87::load(FREE_CAM_DIST_NOW.get(h).to_bits()) < 1.0 {
            FREE_CAM_DIST_NOW.set(h, 1.0);
        }
        let f = |t: Trig, v: f32| f32::from_bits(trig(t, v.to_bits()));
        let sin_yaw = x87::round_trip(f(Trig::Sin, yaw));
        let cos_yaw = x87::round_trip(f(Trig::Cos, FREE_CAM_YAW.get(h)));
        let pitch = FREE_CAM_PITCH.get(h);
        let sin_pitch = f(Trig::Sin, pitch);
        let x = Reg::load(sin_pitch).mul_m32(cos_yaw).store();
        let cos_pitch = f(Trig::Cos, pitch);
        let z = Reg::load(cos_pitch).mul_m32(cos_yaw).store();
        let back = (-Reg::load(FREE_CAM_DIST_NOW.get(h))).store();
        let x = Reg::load(back).mul_m32(x).store();
        let y = Reg::load(back).mul_m32(sin_yaw).store();
        let z = Reg::load(back).mul_m32(z).store();
        let [rx, ry, rz] = CAMERA_REF.get(h);
        CAMERA_POS.set(
            h,
            [
                Reg::load(rx).add_m32(x).store(),
                Reg::load(ry).add_m32(y).store(),
                Reg::load(rz).add_m32(z).store(),
            ],
        );
        crate::camera::bridge::sg_camera_put_rects(h)
    }

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

    /// [`id_get_height`]'s local for the normal (unmapped, below
    /// [`crate::scratch::SCRATCH`]).
    const HEIGHT_FRAME: u32 = 0xffff_e000;

    /// `Sg_IdGetHeight(p, i)` (`0x0083e040`): the height of id triangle
    /// `i`'s plane under the point `p` (`f32` x, z): with the triangle's
    /// normal `n` ([`id_get_normal`]) and first vertex `v`, `d = (n.y v.y +
    /// n.x v.x) + n.z v.z` and `t = n.x p.x + n.z p.z` (each stored as
    /// `f32`), the height `(d - t) / n.y`. 0 without id data or for `i`
    /// -1. Returned in ST0.
    pub fn id_get_height(mem: &mut impl Mem, p: u32, i: u32) -> f32 {
        use ffx_bridge::x87;
        let table = ID_TABLE.get(mem);
        if table == 0 || i == u32::MAX {
            return 0.0;
        }
        // the normal's local: below the scratch range, which the normal's
        // own locals (Sg_CalcTriangleNormal) take
        let n: [u32; 3] = {
            let mut f = ffx_bridge::frame::Frame::new(mem, HEIGHT_FRAME, 16);
            id_get_normal(&mut f, HEIGHT_FRAME, i);
            f.read(HEIGHT_FRAME)
        };
        let [nx, ny, nz] = n.map(x87::load);
        let e = table.wrapping_add(i << 4);
        let verts = ID_VERTICES.get(mem);
        let first = i32::from(mem.read::<i16>(e));
        let vp = verts.wrapping_add((first << 3).cast_unsigned());
        let v = [0u32, 2, 4].map(|k| {
            let c = i32::from(mem.read::<i16>(vp.wrapping_add(k)));
            x87::load(x87::store(f64::from(c)))
        });
        let ld = |b: u32| x87::load(b);
        let d = x87::add(
            x87::add(x87::mul(ny, v[1]), x87::mul(nx, v[0])),
            x87::mul(nz, v[2]),
        );
        let d = ld(x87::store(d));
        let px = ld(mem.read(p));
        let pz = ld(mem.read(p.wrapping_add(8)));
        let t = ld(x87::store(x87::add(x87::mul(nx, px), x87::mul(nz, pz))));
        f32::from_bits(x87::store(x87::div(x87::sub(d, t), ny)))
    }

    /// `TkGetControlerHist(pad, a, b)` and `TkGetControlerAnalogHist(pad,
    /// a, axis, b)`, called as the game's.
    pub const TK_GET_CONTROLER_HIST: u32 = 0x0088_8d70;
    pub const TK_GET_CONTROLER_ANALOG_HIST: u32 = 0x0088_8c20;
    /// The pad state `Sg_ControlMain` keeps: the stick bytes `0x023c44a4..a7`
    /// (left y, left x, right y, right x as read by axis 3..0), last
    /// frame's held buttons, and the debug L+R request.
    pub const PAD_STICKS: u32 = 0x023c_44a4;
    pub const PAD_LAST: Global<u32> = Global::new(0x023c_44b0);
    pub const PAD_DEBUG_REQUEST: Global<u32> = Global::new(0x023c_44b4);

    /// `Sg_ControlMain()` (`0x0082d070`): the frame's pad. Held buttons
    /// (`TkGetControlerHist(0, 0, 0)`, 16 bits) and the four stick axes
    /// (`TkGetControlerAnalogHist(0, 0, axis, 0)` into `0x023c44a7` down to
    /// `0x023c44a4`); pressed = newly held since last frame; then the free
    /// camera (`Sg_ControlCamera`). In a debug build without the L/R button
    /// mode, buttons step the frames per step ([`EXEC_FRAMES`]: 8+4
    /// pressing 4 up, 4+8 pressing 8 down; below 0 it becomes 0 and
    /// [`EXEC_STEP`] is set), 4 alone requests `0x023c44b4`, button 1
    /// goes to [`EXEC_SPECIAL`]. Returns the last `eax`: the camera's, the
    /// L/R mode, or the frames byte.
    pub fn control_main(h: &mut impl ffx_bridge::Host) -> u32 {
        let hist = h.rebase(TK_GET_CONTROLER_HIST);
        let held = h.call(hist, &[0, 0, 0]) & 0xffff;
        PAD_HELD.set(h, held);
        let analog = h.rebase(TK_GET_CONTROLER_ANALOG_HIST);
        let sticks = h.rebase(PAD_STICKS);
        for axis in 0..4u32 {
            let v = crate::lo8(h.call(analog, &[0, 0, axis, 0]));
            h.write(sticks.wrapping_add(3 - axis), v);
        }
        let held = PAD_HELD.get(h);
        let pressed = (PAD_LAST.get(h) ^ held) & held;
        PAD_PRESSED.set(h, pressed);
        PAD_LAST.set(h, held);
        let r = control_camera(h);
        EXEC_STEP.set(h, 0);
        PAD_DEBUG_REQUEST.set(h, 0);
        if IS_DEBUG.get(h) == 0 {
            return r;
        }
        let lr = get_lr_button_mode(h);
        if lr != 0 {
            return lr.cast_unsigned();
        }
        let b = PAD_HELD.get(h);
        EXEC_SPECIAL.set(h, crate::lo8(b & 1));
        let p = PAD_PRESSED.get(h);
        let mut level = EXEC_FRAMES.get(h);
        if b & 8 != 0 && b & 4 != 0 && p & 4 != 0 {
            level = level.wrapping_add(1);
            EXEC_FRAMES.set(h, level);
        }
        if b & 4 != 0 {
            if b & 8 == 0 {
                PAD_DEBUG_REQUEST.set(h, 1);
            } else if p & 8 != 0 {
                level = level.wrapping_sub(1);
                EXEC_FRAMES.set(h, level);
            }
        }
        if level.cast_signed() < 0 {
            EXEC_STEP.set(h, 1);
            EXEC_FRAMES.set(h, 0);
        }
        u32::from(level)
    }

    /// `Sg_ChrPushApart(a, b, reach, dist)` (`0x0083ecd0`): two characters
    /// closer than `reach` (`dist` apart) are pushed apart in x and z by
    /// the overlap `reach - dist`, along the normalised direction from `b`
    /// to `a`. The shares come from the characters' weights (`+0x500`): `a`
    /// moves `wa / (wa + wb)` of the overlap and `b` the rest. A negative
    /// weight does not move (the other takes all); both non-negative use
    /// them as they are (both 0 meaning 1 each); both negative weigh the
    /// characters by their distances from their anchors
    /// (`Sg_VectorGetLengthXZ(+0x1c, +0xc)`, scaled by the larger; equal
    /// distances end it, returning 0), and then `a` takes the
    /// remainder rather than the share. A share below 0 ends it (0). Each
    /// moved character's step (`+0x4c`, `+0x54`) gets the push, and `+0x194`
    /// bit 5 when the step is not zero. Returns bit 5 of either
    /// character's `+0x194`.
    /// The characters the push-apart pass checks (`0x0083d3b0` fills it:
    /// 40 pointers) and their count.
    pub const PUSH_CHRS: u32 = 0x0130_1b08;
    pub const PUSH_CHR_COUNT: Global<i32> = Global::new(0x0130_1ba8);

    /// `Sg_ChrSortByZ(n)` (`0x0083eb30`): the first `n` of [`PUSH_CHRS`]
    /// sorted by their z (`+0x14`) with a selection sort: for each place,
    /// the last of the rest whose z is at most the smallest so far (a NaN
    /// never is) swaps in.
    pub fn chr_sort_by_z(mem: &mut impl Mem, n: i32) {
        let list = mem.rebase(PUSH_CHRS);
        let at = |i: i32| list.wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let z = |mem: &mut _, i: i32| {
            let chr: u32 = Mem::read(mem, at(i));
            f64::from(Mem::read::<f32>(mem, chr.wrapping_add(0x14)))
        };
        for i in 0..n.max(0) {
            let mut min = i;
            for j in i..n {
                if z(mem, j) <= z(mem, min) {
                    min = j;
                }
            }
            let a: u32 = mem.read(at(min));
            let b: u32 = mem.read(at(i));
            mem.write(at(i), a);
            mem.write(at(min), b);
        }
    }

    #[allow(clippy::float_cmp)]
    pub fn chr_push_apart(mem: &mut impl Mem, a: u32, b: u32, reach: f32, dist: f32) -> u32 {
        use crate::scratch::{SCRATCH, Scratch};
        use ffx_bridge::x87;
        let ld = |x: f32| x87::load(x.to_bits());
        let st = |x: f64| f32::from_bits(x87::store(x));
        let ra: f32 = mem.read(a.wrapping_add(0x500));
        let rb: f32 = mem.read(b.wrapping_add(0x500));
        let (mut wa, mut wb) = (1.0f32, 1.0f32);
        let mut both_negative = false;
        if ld(rb) < 0.0 {
            wa = 0.0;
            if ld(ra) < 0.0 {
                both_negative = true;
                let la = crate::vec::bridge::vector_get_length_xz(
                    mem,
                    a.wrapping_add(0x1c),
                    a.wrapping_add(0xc),
                );
                let lb = crate::vec::bridge::vector_get_length_xz(
                    mem,
                    b.wrapping_add(0x1c),
                    b.wrapping_add(0xc),
                );
                if ld(lb) == ld(la) {
                    return 0;
                }
                let mx = if ld(lb) > ld(la) { lb } else { la };
                wa = st(x87::div(ld(la), ld(mx)));
                wb = st(x87::div(ld(lb), ld(mx)));
            }
        } else if ld(ra) < 0.0 {
            wb = 0.0;
        } else {
            wa = x87::round_trip(ra);
            wb = x87::round_trip(rb);
            if wa == 0.0 && wb == 0.0 {
                wa = 1.0;
                wb = 1.0;
            }
        }
        let d = st(x87::sub(ld(reach), ld(dist)));
        let pa: [f32; 3] = mem.read(a.wrapping_add(0xc));
        let pb: [f32; 3] = mem.read(b.wrapping_add(0xc));
        let dx = st(x87::sub(ld(pa[0]), ld(pb[0])));
        let dz = st(x87::sub(ld(pa[2]), ld(pb[2])));
        let (dx, dz) = {
            let mut s = Scratch::new(mem);
            s.write(SCRATCH, [dx, 0.0, dz]);
            ffx_ps2::vu0::bridge::normalize(&mut s, SCRATCH, SCRATCH);
            let v: [f32; 3] = s.read(SCRATCH);
            (v[0], v[2])
        };
        let t = st(x87::div(x87::mul(ld(wa), ld(d)), x87::add(ld(wa), ld(wb))));
        if ld(t) < 0.0 {
            return 0;
        }
        let u = st(x87::sub(ld(d), ld(t)));
        let (sa, sb) = if both_negative { (t, u) } else { (u, t) };
        let push = |mem: &mut _, c: u32, s: f32, sign: f64| {
            if ld(s) == 0.0 {
                return;
            }
            let x = Mem::read::<f32>(mem, c.wrapping_add(0x4c));
            let x = st(x87::add(ld(x), sign * x87::mul(ld(dx), ld(s))));
            Mem::write(mem, c.wrapping_add(0x4c), x);
            let z = Mem::read::<f32>(mem, c.wrapping_add(0x54));
            let z = st(x87::add(ld(z), sign * x87::mul(ld(dz), ld(s))));
            Mem::write(mem, c.wrapping_add(0x54), z);
            if x != 0.0 || z != 0.0 {
                let f: u32 = Mem::read(mem, c.wrapping_add(0x194));
                Mem::write(mem, c.wrapping_add(0x194), f | 0x20);
            }
        };
        push(mem, a, sa, 1.0);
        push(mem, b, sb, -1.0);
        let fa: u32 = mem.read(a.wrapping_add(0x194));
        let fb: u32 = mem.read(b.wrapping_add(0x194));
        ((fb | fa) >> 5) & 1
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

    /// `Movie_IsBusy_0076ed10()`.
    pub const MOVIE_IS_BUSY: u32 = 0x0076_ed10;
    /// `TOMenuTryOpen_008be840(request)`.
    pub const TO_MENU_TRY_OPEN: u32 = 0x008b_e840;
    /// `Sg_MenuSetReserve(request)`.
    pub const SG_MENU_SET_RESERVE: u32 = 0x0082_1750;
    /// `TkMenuStart(mode)`.
    pub const TK_MENU_START: u32 = 0x008a_b030;
    /// `g_PauseOn`.
    pub const PAUSE_ON: Global<u32> = Global::new(0x0134_0828);
    /// [`MENU_RESERVE`]'s request word (-1: none).
    pub const MENU_REQUEST: Global<u32> = Global::new(0x012f_bc38);
    /// [`MENU_RESERVE`]'s second word: the open's result
    /// (`Native_MenuOpen_Result` reads it).
    pub const MENU_RESULT: Global<u32> = Global::new(0x012f_bc3c);
    /// The menu background capture's state (`g_MenuCaptureState`, name
    /// ours): the renderer (`0x00668d68`, `0x0066adcf`) copies the frame
    /// while it is 1 and sets 2; `Sg_MainCheckMenu` keeps the request and
    /// answers 0 while it is 1.
    pub const MENU_CAPTURE_STATE: Global<u32> = Global::new(0x0134_0844);

    /// `Sg_MainCheckMenu()` (`0x00820750`, `Sg_MainLoop`'s menu check):
    /// with [`MOVIE_FADING`] or [`PAUSE_ON`] set, `TkMenuStart(request)`
    /// and 0. With no request (`MENU_RESERVE[0]` = -1), the newly pressed
    /// pad bit 0x10 reserves request 0 (`Sg_MenuSetReserve(0)`). With one,
    /// `MENU_RESERVE[1]` gets 1 while no movie runs (`Movie_IsBusy` 0),
    /// else `TOMenuTryOpen(request)`, and a 0 there makes the answer 1;
    /// the request is cleared (-1) unless [`MENU_CAPTURE_STATE`] is 1.
    /// Returns 0 whenever [`MENU_CAPTURE_STATE`] is 1 at the end.
    pub fn main_check_menu(h: &mut impl ffx_bridge::Host) -> u32 {
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        if MOVIE_FADING.get(h) != 0 || PAUSE_ON.get(h) != 0 {
            let request = MENU_REQUEST.get(h);
            call(h, TK_MENU_START, &[request]);
            return 0;
        }
        let mut answer = 0;
        if MENU_REQUEST.get(h) == u32::MAX {
            if lo8(PAD_PRESSED.get(h)) & 0x10 != 0 {
                call(h, SG_MENU_SET_RESERVE, &[0]);
            }
        } else {
            let result = if call(h, MOVIE_IS_BUSY, &[]) == 0 {
                1
            } else {
                let request = MENU_REQUEST.get(h);
                let r = call(h, TO_MENU_TRY_OPEN, &[request]);
                if r == 0 {
                    answer = 1;
                }
                r
            };
            MENU_RESULT.set(h, result);
            if MENU_CAPTURE_STATE.get(h) == 1 {
                return 0;
            }
            MENU_REQUEST.set(h, u32::MAX);
        }
        if MENU_CAPTURE_STATE.get(h) == 1 {
            return 0;
        }
        answer
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

    /// The menu list state of `0x0076f470` (`g_MenuList*`, names ours):
    /// the open hook (a pointer to a function pointer), the lists (a
    /// record pointer and an `i16` count each, 8 bytes), the record
    /// base, the busy byte, the value slots, the text table and its base,
    /// the text out buffer, the preset pairs, the list count, the target
    /// record, the current list, the last record, a mirror word, the
    /// current-record flag and the three pad words (`u16`).
    pub const ML_HOOK: Global<u32> = Global::new(0x0112_a014);
    pub const ML_LISTS: Global<u32> = Global::new(0x0112_a01c);
    pub const ML_TEXTS: Global<u32> = Global::new(0x0112_a024);
    pub const ML_PAIRS: Global<u32> = Global::new(0x0112_a028);
    pub const ML_OUT: Global<u32> = Global::new(0x0112_a02c);
    pub const ML_BUSY: Global<u8> = Global::new(0x0112_a038);
    pub const ML_BASE: Global<u32> = Global::new(0x0112_a03c);
    pub const ML_TEXT_BASE: Global<u32> = Global::new(0x0112_a040);
    pub const ML_IS_TARGET: Global<u32> = Global::new(0x0112_a078);
    pub const ML_COUNT: Global<i32> = Global::new(0x0112_a080);
    pub const ML_TARGET: Global<u32> = Global::new(0x0112_a084);
    pub const ML_CURRENT: Global<u32> = Global::new(0x0112_a088);
    pub const ML_TRG: Global<u16> = Global::new(0x0112_a08c);
    pub const ML_PAD: Global<u16> = Global::new(0x0112_a090);
    pub const ML_REP: Global<u16> = Global::new(0x0112_a094);
    pub const ML_LAST: Global<u32> = Global::new(0x0112_a098);
    pub const ML_MIRROR: Global<u32> = Global::new(0x0112_a0a0);
    pub const ML_SLOTS: Global<u32> = Global::new(0x0112_a0a4);
    /// `TODebugGetPadPrintFlag()`, `TOMenuGetControlPad()`, `...Trg()`,
    /// `...Rep()`, `MenuCursorBlocked()`, `MsgExpandCodes(out, text, arg,
    /// &jump, 0)`, `MenuCursorStep(&again)`, `ResetRecords()`.
    pub const PAD_PRINT_FLAG: u32 = 0x008a_bd20;
    pub const CONTROL_PAD: u32 = 0x008b_e410;
    pub const CONTROL_PAD_TRG: u32 = 0x008b_e4b0;
    pub const CONTROL_PAD_REP: u32 = 0x008b_e470;
    pub const CURSOR_BLOCKED: u32 = 0x0077_5530;
    pub const EXPAND_CODES: u32 = 0x0077_6940;
    pub const CURSOR_STEP: u32 = 0x0077_6110;
    pub const RESET_RECORDS: u32 = 0x0077_4740;
    /// [`menu_list_step`]'s work: `MsgExpandCodes`' jump word, then
    /// `MenuCursorStep`'s answer.
    pub const MENU_STEP_WORK: u32 = 8;

    /// A list's first record and its count.
    fn ml_list(h: &impl Mem, lists: u32, i: u32) -> (u32, i32) {
        let at = lists.wrapping_add(i.wrapping_mul(8));
        let first = h.read::<u32>(at).wrapping_add(ML_BASE.get(h));
        (first, i32::from(h.read::<i16>(at.wrapping_add(4))))
    }

    /// A value slot by an `i16` index.
    fn ml_slot(h: &impl Mem, index: i16) -> u32 {
        let k = i32::from(index).cast_unsigned();
        ML_SLOTS.get(h).wrapping_add(k.wrapping_mul(4))
    }

    /// The pad words cleared.
    fn ml_clear_pad(h: &mut impl Mem) {
        ML_PAD.set(h, 0);
        ML_TRG.set(h, 0);
        ML_REP.set(h, 0);
    }

    /// One value record's pad step (record `+0x104..`): flags (bit 0 on,
    /// bit 1 wraps, bit 2 counts up), the axis `+0x105`, the pad word
    /// `+0x106` (0, 2 held, 1 triggered), the bounds `u16 +0x108/+0x10a`,
    /// the slot `i16 +0x10c`.
    fn ml_value_step(h: &mut impl Mem, rec: u32) {
        let at = |o: u32| rec.wrapping_add(o);
        let flags: u8 = h.read(at(0x104));
        if flags & 1 == 0 {
            return;
        }
        let pad = match h.read::<i8>(at(0x106)) {
            0 | 2 => ML_PAD.get(h),
            1 => ML_TRG.get(h),
            _ => return,
        };
        if pad == 0 {
            return;
        }
        let slot = ml_slot(h, h.read(at(0x10c)));
        let mut value: i32 = h.read(slot);
        let step = if flags & 4 != 0 { 1 } else { -1 };
        let (up, down) = match h.read::<i8>(at(0x105)) {
            0 => (0x1000, 0x4000),
            1 => (0x8000, 0x2000),
            2 => (4, 8),
            3 => (1, 8),
            _ => (0, 0),
        };
        if pad & up != 0 {
            value = value.wrapping_add(step);
        }
        if pad & down != 0 {
            value = value.wrapping_sub(step);
        }
        let lo = i32::from(h.read::<u16>(at(0x108)));
        let hi = i32::from(h.read::<u16>(at(0x10a)));
        let value = if flags & 2 == 0 {
            value.max(lo).min(hi)
        } else if value < lo {
            hi
        } else if value > hi {
            lo
        } else {
            value
        };
        h.write(slot, value);
    }

    /// `0x0076f470()` (`menu_list_step`, name ours; no direct caller):
    /// one step of a menu's lists. The open hook (`*[0x0112a014]`) runs,
    /// the pad words are read (`Rep` keeps its bits 4..7 from `Trg`);
    /// then, while `MenuCursorStep` answers 1: a record of the current
    /// list with byte `+0x110` 2 and `u16 +0x116` set clears the pad
    /// words; `MenuCursorBlocked`; unless busy, each record of the
    /// current list (0x138 bytes from its pointer + base) notes whether
    /// it is the target, a text record (kind 0) expands its text
    /// (`MsgExpandCodes` with the slot values of `+0x18`/`+0x24`, -1 for
    /// none) and on a jump moves the current list to the one holding the
    /// target (its presets `+0x1c`/`+0x1e` into the slots,
    /// `ResetRecords` when the list changed, the pad words cleared) and
    /// starts over; a kind-6 record of a newly shown list resets
    /// `+0x2a`/`+0x1a`. Then every record's value steps with the pad.
    /// Returns `MenuCursorStep`'s `eax`.
    #[allow(clippy::too_many_lines)] // one loop with a restart
    pub fn menu_list_step(h: &mut impl ffx_bridge::Host, work: u32) -> u32 {
        let hook = ML_HOOK.get(h);
        if hook != 0 {
            let f: u32 = h.read(hook);
            h.call(f, &[]);
        }
        let call = |h: &mut _, f: u32| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, &[])
        };
        call(h, PAD_PRINT_FLAG);
        let pad = call(h, CONTROL_PAD);
        ML_PAD.set(h, crate::lo16(pad));
        let trg = call(h, CONTROL_PAD_TRG);
        ML_TRG.set(h, crate::lo16(trg));
        let rep = crate::lo16(call(h, CONTROL_PAD_REP)) & 0xff0f;
        let held = crate::lo16(h.read::<u32>(h.rebase(ML_TRG.sym)) & 0xf0);
        ML_REP.set(h, rep | held);
        let (jump, again) = (work, work.wrapping_add(4));
        'restart: loop {
            ML_IS_TARGET.set(h, 0);
            let lists = ML_LISTS.get(h);
            if lists != 0 {
                let (mut rec, n) = ml_list(h, lists, ML_CURRENT.get(h));
                for _ in 0..n.max(0) {
                    let kind: i8 = h.read(rec.wrapping_add(0x110));
                    if kind == 2 && h.read::<u16>(rec.wrapping_add(0x116)) != 0 {
                        ml_clear_pad(h);
                    }
                    rec = rec.wrapping_add(0x138);
                }
            }
            call(h, CURSOR_BLOCKED);
            let lists = ML_LISTS.get(h);
            if ML_BUSY.get(h) == 0 && lists != 0 {
                let current = ML_CURRENT.get(h);
                let (first, n) = ml_list(h, lists, current);
                let shown = first != ML_LAST.get(h);
                let mut target = ML_TARGET.get(h);
                let mirror = if target == 0 {
                    first
                } else {
                    current.wrapping_sub(ML_BASE.get(h))
                };
                ML_MIRROR.set(h, mirror);
                ML_LAST.set(h, first);
                if n > 0 {
                    let mut rec = first;
                    for _ in 0..n {
                        let value = |h: &mut _, o: u32| -> u32 {
                            let index: i16 = Mem::read(h, rec.wrapping_add(o));
                            if index < 0 {
                                0
                            } else {
                                Mem::read(h, ml_slot(h, index))
                            }
                        };
                        let (text, arg) = (value(h, 0x18), value(h, 0x24));
                        ML_IS_TARGET.set(h, u32::from(rec == target));
                        match h.read::<u8>(rec) {
                            0 => {
                                let k = i32::from(h.read::<i16>(rec.wrapping_add(0x16)));
                                let k = k.cast_unsigned().wrapping_add(text);
                                let texts = ML_TEXTS.get(h).wrapping_add(k.wrapping_mul(4));
                                let text: u32 = h.read(texts);
                                let text = text.wrapping_add(ML_TEXT_BASE.get(h));
                                let (f, out) = (h.rebase(EXPAND_CODES), ML_OUT.get(h));
                                h.call(f, &[out, text, arg, jump, 0]);
                                if h.read::<u32>(jump) != 0 {
                                    ml_jump(h, current);
                                    continue 'restart;
                                }
                                target = ML_TARGET.get(h);
                            }
                            6 if shown => {
                                h.write(rec.wrapping_add(0x2a), 0u8);
                                h.write(rec.wrapping_add(0x1a), u16::MAX);
                            }
                            _ => {}
                        }
                        rec = rec.wrapping_add(0x138);
                    }
                    let mut rec = ML_LAST.get(h);
                    for _ in 0..n {
                        ml_value_step(h, rec);
                        rec = rec.wrapping_add(0x138);
                    }
                }
            }
            let f = h.rebase(CURSOR_STEP);
            let eax = h.call(f, &[again]);
            if h.read::<u32>(again) != 1 {
                return eax;
            }
        }
    }

    /// The jump of [`menu_list_step`]: the first list (of the
    /// `0x0112a080`) holding the target record becomes current (mirror
    /// `i - base`), its presets (`u16 +0x1c` pairs of slot index and
    /// value from `pairs + 8 * i16 +0x1e`) go into the slots,
    /// `ResetRecords` runs when it is not `was`, and the pad words clear.
    /// Without one nothing changes.
    fn ml_jump(h: &mut impl ffx_bridge::Host, was: u32) {
        let (target, lists) = (ML_TARGET.get(h), ML_LISTS.get(h));
        let mut i = 0u32;
        while i.cast_signed() < ML_COUNT.get(h) {
            let (mut rec, n) = ml_list(h, lists, i);
            for _ in 0..n.max(0) {
                if rec == target {
                    ML_CURRENT.set(h, i);
                    let base = ML_BASE.get(h);
                    ML_MIRROR.set(h, i.wrapping_sub(base));
                    let presets = rec.wrapping_add(0x1c);
                    if h.read::<u16>(presets) != 0 {
                        let k = i32::from(h.read::<i16>(rec.wrapping_add(0x1e)));
                        let mut pair = ML_PAIRS
                            .get(h)
                            .wrapping_add(k.cast_unsigned().wrapping_mul(8));
                        let slots = ML_SLOTS.get(h);
                        let mut done = 0u32;
                        while done < u32::from(h.read::<u16>(presets)) {
                            let (slot, value): (u32, u32) =
                                (h.read(pair), h.read(pair.wrapping_add(4)));
                            h.write(slots.wrapping_add(slot.wrapping_mul(4)), value);
                            done += 1;
                            pair = pair.wrapping_add(8);
                        }
                    }
                    if was != i {
                        let f = h.rebase(RESET_RECORDS);
                        h.call(f, &[]);
                    }
                    ml_clear_pad(h);
                    return;
                }
                rec = rec.wrapping_add(0x138);
            }
            i += 1;
        }
    }

    /// `Sg_LoadVramResource()`, `user_malloc(size)`, `user_free(p)`,
    /// `Sg_InitFilter()`, `Sg_DbInitAll()` (debug), `Sg_Dmy()`,
    /// `0x00839ca0(1)`, `Ch_FunctionHistoryInit()`,
    /// `Ch_TextureSpecInitAll()`, `Ch_DrawSetShadowMode(mode)`,
    /// `Ch_MemMoveSetTrigger()`, `dbgPrintf(format, ..)`,
    /// `Sg_MapSetName(name, which)`, `Sg_MapInit()`.
    pub const LOAD_VRAM_RESOURCE: u32 = 0x0083_fc50;
    pub const SG_USER_MALLOC: u32 = 0x0068_7020;
    pub const SG_USER_FREE: u32 = 0x0068_7010;
    pub const INIT_FILTER: u32 = 0x0082_cd50;
    pub const DB_INIT_ALL: u32 = 0x0084_1440;
    pub const SG_DMY: u32 = 0x0084_67a0;
    pub const INIT_0083_9CA0: u32 = 0x0083_9ca0;
    pub const FUNCTION_HISTORY_INIT: u32 = 0x0082_6a40;
    pub const TEXTURE_SPEC_INIT_ALL: u32 = 0x0083_d140;
    pub const DRAW_SET_SHADOW_MODE: u32 = 0x0083_0140;
    pub const MEM_MOVE_SET_TRIGGER: u32 = 0x0082_7250;
    pub const SG_DBG_PRINTF: u32 = 0x0062_f500;
    pub const MAP_SET_NAME: u32 = 0x0084_4fb0;
    pub const SG_MAP_INIT: u32 = 0x0084_4d10;
    /// The emulated GS register sink (`g_GsRegisterSink`).
    pub const GS_REGISTER_SINK: Global<u32> = Global::new(0x0230_56e8);
    /// The 1 MB block `Sg_InitAll` allocates (`0x00c498dc`).
    pub const INIT_BLOCK: Global<u32> = Global::new(0x00c4_98dc);
    /// `Sg_Test`'s 64 KB block (`0x013036b8`) and the frame counter it
    /// prints (`g_SgFrame`).
    pub const TEST_BLOCK: Global<u32> = Global::new(0x0130_36b8);
    pub const SG_FRAME: Global<u32> = Global::new(0x023c_bbf0);
    /// `Sg_Test`'s `"%d:SG:sync end\n"` and its two map names
    /// (`"sugimoto"`, `"kaisou"`).
    pub const FMT_TEST_FRAME: u32 = 0x00b5_aac4;
    pub const TEST_MAP_NAMES: [u32; 2] = [0x00b5_9078, 0x00b5_aad4];

    /// `Sg_InitAll()` (`0x0083b5f0`): `Sg_LoadVramResource`, the GS sink
    /// written 0x83 twice, a 1 MB block into `0x00c498dc`,
    /// `Sg_InitFilter`, `Sg_DbInitAll`, `Sg_Dmy`, `0x00839ca0(1)`, the
    /// character function history and texture specs, shadow mode 0.
    /// Returns the last call's `eax`.
    pub fn init_all(h: &mut impl ffx_bridge::Host) -> u32 {
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        call(h, LOAD_VRAM_RESOURCE, &[]);
        GS_REGISTER_SINK.set(h, 0x83);
        GS_REGISTER_SINK.set(h, 0x83);
        let block = call(h, SG_USER_MALLOC, &[0x10_0000]);
        INIT_BLOCK.set(h, block);
        for f in [INIT_FILTER, DB_INIT_ALL, SG_DMY] {
            call(h, f, &[]);
        }
        call(h, INIT_0083_9CA0, &[1]);
        call(h, FUNCTION_HISTORY_INIT, &[]);
        call(h, TEXTURE_SPEC_INIT_ALL, &[]);
        call(h, DRAW_SET_SHADOW_MODE, &[0])
    }

    /// `Sg_Test(n)` (`0x008467b0`, the script native 0x401e; jump table
    /// `0x00846830`): 0 names the two test maps (`Sg_MapSetName`) and
    /// runs `Sg_MapInit`, 1 prints the frame (`dbgPrintf`), 2 allocates a
    /// 64 KB block into `0x013036b8`, 3 frees it and sets the character
    /// memory-move trigger; anything else nothing. Returns `n`.
    pub fn sg_test(h: &mut impl ffx_bridge::Host, n: u32) -> u32 {
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        match n {
            0 => {
                for (which, name) in (0u32..).zip(TEST_MAP_NAMES) {
                    let name = h.rebase(name);
                    call(h, MAP_SET_NAME, &[name, which]);
                }
                call(h, SG_MAP_INIT, &[]);
            }
            1 => {
                let (fmt, frame) = (h.rebase(FMT_TEST_FRAME), SG_FRAME.get(h));
                call(h, SG_DBG_PRINTF, &[fmt, frame]);
            }
            2 => {
                let block = call(h, SG_USER_MALLOC, &[0x1_0000]);
                TEST_BLOCK.set(h, block);
            }
            3 => {
                let block = TEST_BLOCK.get(h);
                call(h, SG_USER_FREE, &[block]);
                call(h, MEM_MOVE_SET_TRIGGER, &[]);
            }
            _ => {}
        }
        n
    }

    /// `0x00821e80`'s state (`Sg_FrameSync`, name ours): the frame
    /// remainder and the voice frame remainder (`f32`), the voice hold
    /// count, the sync frame index, the sync clock's start, accumulated
    /// pause and its pending add (64-bit microseconds), the sync on flag,
    /// its last, now, step time and rate (`f32`), the camera restart
    /// flag, the v-blank counters, the pause and fade words, the debug
    /// byte.
    pub const FS_REMAINDER: Global<u32> = Global::new(0x012f_b7f8);
    pub const FS_VOICE_REMAINDER: Global<u32> = Global::new(0x012f_b7fc);
    pub const FS_VOICE_HOLD: Global<u32> = Global::new(0x012f_b808);
    pub const FS_SYNC_INDEX: Global<u32> = Global::new(0x012f_b814);
    pub const FS_START: Global<u64> = Global::new(0x012f_b818);
    pub const FS_PAUSED: Global<u64> = Global::new(0x012f_b7e8);
    pub const FS_PAUSE_ADD: Global<u64> = Global::new(0x00cc_c868);
    pub const FS_SYNC_ON: Global<u32> = Global::new(0x012f_b8a0);
    pub const FS_SYNC_LAST: Global<u32> = Global::new(0x012f_b8a4);
    pub const FS_SYNC_NOW: Global<u32> = Global::new(0x012f_b8a8);
    pub const FS_SYNC_STEP: Global<u32> = Global::new(0x012f_b8ac);
    pub const FS_SYNC_RATE: Global<u32> = Global::new(0x012f_b8b0);
    pub const FS_RESTART: Global<u32> = Global::new(0x00c4_94d4);
    pub const FS_VCOUNT: Global<u32> = Global::new(0x012f_b7f0);
    pub const FS_VCOUNT2: Global<u32> = Global::new(0x012f_b7f4);
    pub const FS_MOVIE_FADING: Global<u32> = Global::new(0x0134_0824);
    pub const FS_PAUSE_ON: Global<u32> = Global::new(0x0134_0828);
    pub const FS_PAUSE_2: Global<u32> = Global::new(0x0134_082c);
    pub const FS_IS_DEBUG: Global<u8> = Global::new(0x0133_c910);
    /// `setFramSync()`, `Sg_SyncCutScene()`,
    /// `graphicGetTimeInMicroseconds()` (64-bit), `Sg_CanSpeedUp()`,
    /// `SaveDataIsScreenClosed()`, `TkScanControler()`, `Gu_VSyncMain()`
    /// (debug), `Sg_EffectReadPoll(dt)`.
    pub const SET_FRAM_SYNC: u32 = 0x0082_2720;
    pub const SYNC_CUT_SCENE: u32 = 0x0082_1d60;
    pub const TIME_US: u32 = 0x0064_1420;
    pub const CAN_SPEED_UP: u32 = 0x0081_fd30;
    pub const SCREEN_CLOSED: u32 = 0x0064_8100;
    pub const SCAN_CONTROLER: u32 = 0x0088_93e0;
    pub const VSYNC_MAIN: u32 = 0x0084_a460;
    pub const EFFECT_READ_POLL: u32 = 0x0082_2250;
    /// `.rdata`: 0.3 (`f32`, and as `f64`), 60, 1000, 0.1, 30 (`f64`), 2
    /// (`f32`).
    const FS_K03_F32: u32 = 0x00b3_eb44;
    const FS_K03: u32 = 0x00b4_8308;
    const FS_K60: u32 = 0x00b4_a180;
    const FS_K1000: u32 = 0x00b0_dba0;
    const FS_K01: u32 = 0x00b9_21f0;
    const FS_K30: u32 = 0x00b9_22c8;
    const FS_K2_F32: u32 = 0x00b9_2268;

    /// The sync clock in seconds (`f32`): microseconds since the start
    /// less the paused time, `u64` to float (one rounding), to `f32`,
    /// over 1000, to `f32`.
    fn fs_seconds(h: &impl Mem, now: u64) -> u32 {
        let d = now
            .wrapping_sub(FS_PAUSED.get(h))
            .wrapping_sub(FS_START.get(h));
        #[allow(clippy::cast_precision_loss)] // the original's u64 to float
        let us = ffx_bridge::x87::load(ffx_bridge::x87::store(d as f64));
        ffx_bridge::x87::store(ffx_bridge::x87::div(us, h.read::<f64>(h.rebase(FS_K1000))))
    }

    /// `0x00821e80(dt)` (`Sg_FrameSync`, name ours; from `Sg_MainLoop`
    /// each frame, `dt` the frame's seconds as `f32`). `dt` is capped at
    /// 0.3; the frames due are (remainder + `dt`) x 60. Unless the voice
    /// hold runs (`0x012fb808`): the cut-scene sync (`Sg_SyncCutScene`,
    /// index 0); while it is on, the sync clock (`time` gives
    /// `graphicGetTimeInMicroseconds`) either restarts (`0x00c494d4`) or
    /// takes the pending pause and is read: behind the step time it
    /// steps (hold and index up, the sync again) while the rate is below
    /// how far it is ahead and the sync stays on; ahead by 0.1 s or more
    /// it spins until the clock catches up. Off, fewer than 2 frames due
    /// or no speed-up (`Sg_CanSpeedUp`) holds 1 with both remainders 0;
    /// else the whole frames go to the hold (`ftol` of the voice frames
    /// x 30) and the fractions stay. Then the v-blank counters (+2 when
    /// no movie fade, no pause and the save screen is closed), the pad
    /// scan, `Gu_VSyncMain` in debug builds, and `Sg_EffectReadPoll(dt)`,
    /// whose `eax` it returns.
    #[allow(clippy::too_many_lines)] // one frame, in the original's order
    pub fn frame_sync<H: ffx_bridge::Host>(
        h: &mut H,
        dt: u32,
        time: &mut impl FnMut(&mut H) -> u64,
    ) -> u32 {
        let call = |h: &mut H, f: u32, args: &[u32]| {
            let f = h.rebase(f);
            h.call(f, args)
        };
        let k = |h: &H, at: u32| h.read::<f64>(h.rebase(at));
        let remainder = ffx_bridge::x87::load(FS_REMAINDER.get(h));
        let mut due = ffx_bridge::x87::store(ffx_bridge::x87::mul(
            ffx_bridge::x87::add(remainder, ffx_bridge::x87::load(dt)),
            k(h, FS_K60),
        ));
        call(h, SET_FRAM_SYNC, &[]);
        let cap = ffx_bridge::x87::load(h.read(h.rebase(FS_K03_F32)));
        let mut dt = dt;
        if ffx_bridge::x87::load(dt) > cap {
            dt = ffx_bridge::x87::store(cap);
            let remainder = ffx_bridge::x87::load(FS_REMAINDER.get(h));
            due = ffx_bridge::x87::store(ffx_bridge::x87::mul(
                ffx_bridge::x87::add(remainder, k(h, FS_K03)),
                k(h, FS_K60),
            ));
        }
        if FS_VOICE_HOLD.get(h) == 0 {
            FS_SYNC_INDEX.set(h, 0);
            call(h, SYNC_CUT_SCENE, &[]);
            let through =
                |h: &H, g: Global<u32>| ffx_bridge::x87::store(ffx_bridge::x87::load(g.get(h)));
            if FS_SYNC_ON.get(h) == 1 {
                let restart = FS_RESTART.get(h) == 1;
                FS_VOICE_HOLD.set(h, 1);
                if restart {
                    let now = time(h);
                    let step = through(h, FS_SYNC_STEP);
                    FS_SYNC_LAST.set(h, step);
                    FS_START.set(h, now);
                    FS_RESTART.set(h, 0);
                    FS_PAUSE_ADD.set(h, 0);
                    FS_PAUSED.set(h, 0);
                } else {
                    let now = time(h);
                    let paused = FS_PAUSED.get(h).wrapping_add(FS_PAUSE_ADD.get(h));
                    FS_PAUSED.set(h, paused);
                    FS_PAUSE_ADD.set(h, 0);
                    FS_SYNC_NOW.set(h, fs_seconds(h, now));
                    let f = |h: &H, g: Global<u32>| ffx_bridge::x87::load(g.get(h));
                    let (now, last) = (f(h, FS_SYNC_NOW), f(h, FS_SYNC_LAST));
                    if last > now {
                        // ahead: spin while 0.1 s or more ahead
                        let t = time(h);
                        FS_SYNC_NOW.set(h, fs_seconds(h, t));
                        let ahead = |h: &H| {
                            // not below 0.1, or unordered
                            ffx_bridge::x87::sub(f(h, FS_SYNC_LAST), f(h, FS_SYNC_NOW))
                                .partial_cmp(&k(h, FS_K01))
                                != Some(std::cmp::Ordering::Less)
                        };
                        if ahead(h) {
                            loop {
                                let t = time(h);
                                FS_SYNC_NOW.set(h, fs_seconds(h, t));
                                if !ahead(h) {
                                    break;
                                }
                            }
                        }
                    } else {
                        // behind: step while the rate is below the lag
                        let behind = |h: &H| {
                            f(h, FS_SYNC_RATE)
                                < ffx_bridge::x87::sub(f(h, FS_SYNC_NOW), f(h, FS_SYNC_LAST))
                        };
                        if behind(h) {
                            while FS_SYNC_ON.get(h) == 1 {
                                let step = through(h, FS_SYNC_STEP);
                                FS_VOICE_HOLD.set(h, FS_VOICE_HOLD.get(h).wrapping_add(1));
                                FS_SYNC_INDEX.set(h, FS_SYNC_INDEX.get(h).wrapping_add(1));
                                FS_SYNC_LAST.set(h, step);
                                call(h, SYNC_CUT_SCENE, &[]);
                                if !behind(h) {
                                    break;
                                }
                            }
                        }
                    }
                    let step = through(h, FS_SYNC_STEP);
                    FS_SYNC_LAST.set(h, step);
                }
            } else {
                let two = ffx_bridge::x87::load(h.read(h.rebase(FS_K2_F32)));
                let due_f = ffx_bridge::x87::load(due);
                // 2 frames or more due, or unordered
                let speed = due_f.partial_cmp(&two) != Some(std::cmp::Ordering::Less)
                    && call(h, CAN_SPEED_UP, &[]) != 0;
                if speed {
                    let whole = ffx_bridge::x87::ftol(h, due_f);
                    let whole_f = ffx_bridge::x87::load(ffx_bridge::x87::store(f64::from(
                        whole.cast_signed(),
                    )));
                    let frac =
                        ffx_bridge::x87::div(ffx_bridge::x87::sub(due_f, whole_f), k(h, FS_K60));
                    FS_REMAINDER.set(h, ffx_bridge::x87::store(frac));
                    let voice = ffx_bridge::x87::load(FS_VOICE_REMAINDER.get(h));
                    let thirty = k(h, FS_K30);
                    let frames =
                        ffx_bridge::x87::load(ffx_bridge::x87::store(ffx_bridge::x87::mul(
                            ffx_bridge::x87::add(voice, ffx_bridge::x87::load(dt)),
                            thirty,
                        )));
                    let hold = ffx_bridge::x87::ftol(h, frames);
                    FS_VOICE_HOLD.set(h, hold);
                    let hold_f = ffx_bridge::x87::load(ffx_bridge::x87::store(f64::from(
                        hold.cast_signed(),
                    )));
                    let frac = ffx_bridge::x87::div(ffx_bridge::x87::sub(frames, hold_f), thirty);
                    FS_VOICE_REMAINDER.set(h, ffx_bridge::x87::store(frac));
                } else {
                    FS_VOICE_HOLD.set(h, 1);
                    FS_REMAINDER.set(h, 0);
                    FS_VOICE_REMAINDER.set(h, 0);
                }
            }
        }
        if FS_MOVIE_FADING.get(h) == 0
            && FS_PAUSE_ON.get(h) == 0
            && FS_PAUSE_2.get(h) == 0
            && call(h, SCREEN_CLOSED, &[]) != 0
        {
            FS_VCOUNT.set(h, FS_VCOUNT.get(h).wrapping_add(2));
            FS_VCOUNT2.set(h, FS_VCOUNT2.get(h).wrapping_add(2));
        }
        call(h, SCAN_CONTROLER, &[]);
        if FS_IS_DEBUG.get(h) != 0 {
            call(h, VSYNC_MAIN, &[]);
        }
        let dt = ffx_bridge::x87::store(ffx_bridge::x87::load(dt));
        call(h, EFFECT_READ_POLL, &[dt])
    }

    /// The help menu's three file numbers (`0x00c416bc`: the menu SPS2,
    /// `0x00c416c0`: the sbin, `0x00c416d8`), its module number
    /// (`0x00c416c4`), the sbin's parse (`0x00c416cc`) and three table
    /// pointers (`0x00c416d0`, `0x00c416d4`, `0x00c416dc`).
    pub const SA_MENU_SPS2: Global<u32> = Global::new(0x00c4_16bc);
    pub const SA_MENU_SBIN: Global<u32> = Global::new(0x00c4_16c0);
    pub const SA_MENU_MODULE: Global<u32> = Global::new(0x00c4_16c4);
    pub const SA_MENU_PARSED: Global<u32> = Global::new(0x00c4_16cc);
    pub const SA_MENU_TABLES: [(u32, u32); 3] = [
        (0x00c4_16d0, 0x00c4_0f30),
        (0x00c4_16d4, 0x00c4_16e0),
        (0x00c4_16dc, 0x00c4_1720),
    ];
    pub const SA_MENU_THIRD: Global<u32> = Global::new(0x00c4_16d8);
    /// The file numbers by menu kind 1..=4, else.
    pub const SA_MENU_FILES: [[u32; 3]; 5] =
        [[4, 5, 6], [7, 8, 9], [10, 11, 12], [13, 14, 0], [1, 2, 3]];
    /// `g_CdLib + 0x14` (a table's size), `+ 0`, `+ 8`, `+ 0x10`.
    pub const CDLIB_SIZE: Global<u32> = Global::new(0x0231_0c54);
    pub const CDLIB_READ: Global<u32> = Global::new(0x0231_0c40);
    pub const CDLIB_SYNC: Global<u32> = Global::new(0x0231_0c48);
    pub const CDLIB_TABLE: Global<u32> = Global::new(0x0231_0c50);
    /// `g_CdChannelKeepNames` (`cd_read.m_mode`), `g_FFXHddMode`.
    pub const CD_READ_MODE: Global<u32> = Global::new(0x0231_0c58);
    pub const FFX_HDD_MODE: Global<i8> = Global::new(0x0133_c913);
    /// The sbin while it is parsed, the parse (again), the event FTC.
    pub const SA_MENU_SBIN_BUF: Global<u32> = Global::new(0x0112_a044);
    pub const SA_MENU_PARSED_2: Global<u32> = Global::new(0x0112_a4a8);
    pub const SA_MENU_FTC: Global<u32> = Global::new(0x0112_a010);
    /// Zeroed at the start: `0x0112a07c` (`g_EmptyName`), `0x0112a018`,
    /// `0x0112a054` words, and after the task: the SPS2 block `0x0112a034`,
    /// the three HDD-copy tables `0x0112a048`, `0x0112a04c`, `0x0112a050`,
    /// `0x02310c20` (the first one's size) and the `system.cnf` copy
    /// `0x0112a064` and its size `0x0112a068`.
    pub const SA_MENU_CLEARED: [u32; 3] = [0x0112_a07c, 0x0112_a018, 0x0112_a054];
    pub const SA_MENU_FLAG_39: Global<u8> = Global::new(0x0112_a039);
    pub const SA_MENU_SPS2_BLOCK: Global<u32> = Global::new(0x0112_a034);
    pub const SA_MENU_HDD_TABLES: [Global<u32>; 3] = [
        Global::new(0x0112_a050),
        Global::new(0x0112_a048),
        Global::new(0x0112_a04c),
    ];
    pub const SA_MENU_HDD_SIZE: Global<u32> = Global::new(0x0231_0c20);
    pub const SA_MENU_CNF: Global<u32> = Global::new(0x0112_a064);
    pub const SA_MENU_CNF_SIZE: Global<u32> = Global::new(0x0112_a068);
    /// The HDD copy's state words set in kind 4: `0x02310c1c` -1,
    /// `0x02310c18` and `0x02310c14` 0.
    pub const SA_MENU_HDD_STATE: [(u32, u32); 3] =
        [(0x0231_0c1c, u32::MAX), (0x0231_0c18, 0), (0x0231_0c14, 0)];
    /// The 0x42000-byte work block.
    pub const SA_MENU_WORK: Global<u32> = Global::new(0x0112_a030);
    /// `"cd_read.m_mode=%d\n"`, `"ffx_hddmode=%d\n"`, `"sa_menu_start\n"`,
    /// `"sa_menu.file_module_no=%d\n"`, `"sbin size=%d\n"`,
    /// `"sps2 size=%d\n"`, the dashes, `"dvd_hdd_copy_system_cnf_buff=\n"`,
    /// `"%s"`; the event FTC's name.
    pub const STR_CD_READ_MODE: u32 = 0x00b5_340c;
    pub const STR_HDD_MODE: u32 = 0x00b5_2164;
    pub const STR_MENU_START: u32 = 0x00b5_3420;
    pub const STR_MODULE_NO: u32 = 0x00b5_3430;
    pub const STR_SBIN_SIZE: u32 = 0x00b5_344c;
    pub const STR_SPS2_SIZE: u32 = 0x00b5_349c;
    pub const STR_DASHES: u32 = 0x00b5_345c;
    pub const STR_CNF: u32 = 0x00b5_347c;
    pub const STR_S: u32 = 0x00b0_e160;
    pub const STR_EVENT_FTC: u32 = 0x00b4_04a0;
    /// `TOMenuClearControlPadBuff()`, `dbgPrintf`,
    /// `MsGetSaveConfigEnglish()`, `HelpMem_Get(reset, size)`,
    /// `0x00775ed0(sbin)` (its parse), `HelpMem_Free(size)`,
    /// `user_free_thunk(p)`, `TKMallocAndReadEventFTC(name)`,
    /// `TOFTCTransfer(ftc)`, `sa_task_buff_size_get(n)`,
    /// `sa_task_init2(n, buffer, 0)`, the C runtime's `memset` (here
    /// through `Mem`), `TKMallocAndReadSPS2(kind, page)`,
    /// `SetFileTables()`, `disk_open_check_set(flag)`,
    /// `hdd_OpenFfxPartition()`, `PatchMenuRecords()`, the two tails
    /// `FindRecordAt()` and `FindRecordFlagged()`; the task's handler.
    pub const CLEAR_PAD_BUFF: u32 = 0x008b_e3d0;
    pub const SA_DBG_PRINTF: u32 = 0x0062_f500;
    pub const GET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;
    pub const HELP_MEM_GET: u32 = 0x0077_4c00;
    pub const SBIN_PARSE: u32 = 0x0077_5ed0;
    pub const HELP_MEM_FREE: u32 = 0x0077_4aa0;
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;
    pub const READ_EVENT_FTC: u32 = 0x0088_c860;
    pub const FTC_TRANSFER: u32 = 0x008a_c130;
    pub const SA_TASK_BUFF_SIZE_GET: u32 = 0x0077_a180;
    pub const SA_TASK_INIT2: u32 = 0x0077_a1d0;
    pub const READ_SPS2: u32 = 0x0088_ce80;
    pub const SET_FILE_TABLES: u32 = 0x0077_6f00;
    pub const DISK_OPEN_CHECK_SET: u32 = 0x0076_d6b0;
    pub const HDD_OPEN_PARTITION: u32 = 0x0077_9000;
    pub const PATCH_MENU_RECORDS: u32 = 0x0077_67d0;
    pub const FIND_RECORD_AT: u32 = 0x0077_5ce0;
    pub const FIND_RECORD_FLAGGED: u32 = 0x0077_5de0;
    pub const SA_MENU_HANDLER: u32 = 0x0077_4c80;

    /// `0x00770430(kind)` (`sa_menu_start`, name ours from its
    /// `dbgPrintf`; from `TkMenuCtrlHelp`): starts the help menu of kind
    /// `kind & 0xff` (1..=4, others as 0).
    ///
    /// The pad buffer cleared, the pad words ([`ML_TRG`], [`ML_PAD`],
    /// [`ML_REP`]) 0, three reports; the file numbers by kind
    /// ([`SA_MENU_FILES`], jump table `0x00770910`) and the three table
    /// pointers; [`SA_MENU_CLEARED`], [`ML_BUSY`] and `0x0112a039` 0. The
    /// module 3 with the English save setting, else 0x30 (reported, made
    /// the CD library's table). The sbin read whole into help memory
    /// (`HelpMem_Get(0, size)`), parsed by `0x00775ed0` (both parse words)
    /// and freed. The event FTC: with none loaded `user_free_thunk(0)`
    /// first; read and, when there is one, transferred. A task
    /// (`sa_task_init2(1, HelpMem_Get(1, sa_task_buff_size_get(1)), 0)`,
    /// [`ML_HOOK`]) gets the handler `0x00774c80` through its `+8` with
    /// `(handler, 1, 0)`. [`ML_SLOTS`] (0x400 bytes) and [`ML_OUT`]
    /// (0x4000) from help memory, zeroed. Kind 4 (the HDD copy) reads
    /// tables 0xf, 0x10, 0x11 and, under table 1, `system.cnf` (table 0xd,
    /// with a 0 after it) and reports it; the others read table 0 (the
    /// SPS2) to [`SA_MENU_SPS2_BLOCK`]. Then a block for the menu SPS2's
    /// size (stored in [`ML_BASE`], then overwritten: help memory taken
    /// for nothing), `TKMallocAndReadSPS2(kind, 1)` and `(kind, 0)` to
    /// [`ML_BASE`] and [`ML_TEXT_BASE`], 0x42000 bytes of work,
    /// `SetFileTables`, for kind 4 the partition opened, `PatchMenuRecords`
    /// and the tail: `FindRecordAt` when `kind & 0x8000`, else
    /// `FindRecordFlagged`, whose `eax` it returns.
    #[allow(clippy::too_many_lines)] // one straight sequence of loads
    pub fn sa_menu_start(h: &mut impl ffx_bridge::Host, kind: u32) -> u32 {
        let call = |h: &mut _, sym: u32, args: &[u32]| -> u32 {
            let f = ffx_bridge::Mem::rebase(h, sym);
            ffx_bridge::Host::call(h, f, args)
        };
        let ptr = |h: &mut _, g: Global<u32>, args: &[u32]| -> u32 {
            let f = g.get(h);
            ffx_bridge::Host::call(h, f, args)
        };
        call(h, CLEAR_PAD_BUFF, &[]);
        let menu = kind & 0xff;
        ML_TRG.set(h, 0);
        ML_PAD.set(h, 0);
        ML_REP.set(h, 0);
        let (fmt, mode) = (h.rebase(STR_CD_READ_MODE), CD_READ_MODE.get(h));
        call(h, SA_DBG_PRINTF, &[fmt, mode]);
        let (fmt, hdd) = (h.rebase(STR_HDD_MODE), FFX_HDD_MODE.get(h));
        call(h, SA_DBG_PRINTF, &[fmt, i32::from(hdd).cast_unsigned()]);
        let fmt = h.rebase(STR_MENU_START);
        call(h, SA_DBG_PRINTF, &[fmt]);
        let files = match menu {
            1..=4 => SA_MENU_FILES[menu as usize - 1],
            _ => SA_MENU_FILES[4],
        };
        SA_MENU_SPS2.set(h, files[0]);
        SA_MENU_SBIN.set(h, files[1]);
        SA_MENU_THIRD.set(h, files[2]);
        for (at, table) in SA_MENU_TABLES {
            let table = h.rebase(table);
            Global::<u32>::new(at).set(h, table);
        }
        for at in SA_MENU_CLEARED[..2].iter().copied() {
            Global::<u32>::new(at).set(h, 0);
        }
        SA_MENU_FLAG_39.set(h, 0);
        ML_BUSY.set(h, 0);
        Global::<u32>::new(SA_MENU_CLEARED[2]).set(h, 0);
        let module = if call(h, GET_SAVE_CONFIG_ENGLISH, &[]) == 0 {
            0x30
        } else {
            3
        };
        SA_MENU_MODULE.set(h, module);
        let fmt = h.rebase(STR_MODULE_NO);
        call(h, SA_DBG_PRINTF, &[fmt, module]);
        let module = SA_MENU_MODULE.get(h);
        ptr(h, CDLIB_TABLE, &[module]);
        // the sbin
        let sbin = SA_MENU_SBIN.get(h);
        let size = ptr(h, CDLIB_SIZE, &[sbin]);
        let fmt = h.rebase(STR_SBIN_SIZE);
        call(h, SA_DBG_PRINTF, &[fmt, size]);
        let buf = call(h, HELP_MEM_GET, &[0, size]);
        SA_MENU_SBIN_BUF.set(h, buf);
        let sbin = SA_MENU_SBIN.get(h);
        ptr(h, CDLIB_READ, &[sbin, buf, 0, 0]);
        ptr(h, CDLIB_SYNC, &[0]);
        let buf = SA_MENU_SBIN_BUF.get(h);
        let parsed = call(h, SBIN_PARSE, &[buf]);
        SA_MENU_PARSED_2.set(h, parsed);
        SA_MENU_PARSED.set(h, parsed);
        call(h, HELP_MEM_FREE, &[size]);
        let none = SA_MENU_FTC.get(h) == 0;
        SA_MENU_SBIN_BUF.set(h, 0);
        if none {
            call(h, USER_FREE_THUNK, &[0]);
            SA_MENU_FTC.set(h, 0);
        }
        let name = h.rebase(STR_EVENT_FTC);
        let ftc = call(h, READ_EVENT_FTC, &[name]);
        SA_MENU_FTC.set(h, ftc);
        if ftc != 0 {
            call(h, FTC_TRANSFER, &[ftc]);
        }
        // the task and its handler
        let n = call(h, SA_TASK_BUFF_SIZE_GET, &[1]);
        let buf = call(h, HELP_MEM_GET, &[1, n]);
        let task = call(h, SA_TASK_INIT2, &[1, buf, 0]);
        ML_HOOK.set(h, task);
        let (f, handler) = (
            h.read::<u32>(task.wrapping_add(8)),
            h.rebase(SA_MENU_HANDLER),
        );
        h.call(f, &[handler, 1, 0]);
        for (g, len) in [(ML_SLOTS, 0x400u32), (ML_OUT, 0x4000)] {
            let block = call(h, HELP_MEM_GET, &[1, len]);
            g.set(h, block);
            h.write_bytes(block, &vec![0u8; len as usize]);
        }
        SA_MENU_SPS2_BLOCK.set(h, 0);
        for g in SA_MENU_HDD_TABLES {
            g.set(h, 0);
        }
        SA_MENU_HDD_SIZE.set(h, 0);
        SA_MENU_CNF.set(h, 0);
        SA_MENU_CNF_SIZE.set(h, 0);
        if menu == 4 {
            for (at, v) in SA_MENU_HDD_STATE {
                Global::<u32>::new(at).set(h, v);
            }
            for (table, g) in (0xf..).zip(SA_MENU_HDD_TABLES) {
                let size = ptr(h, CDLIB_SIZE, &[table]);
                if table == 0xf {
                    SA_MENU_HDD_SIZE.set(h, size);
                }
                let block = call(h, HELP_MEM_GET, &[1, size]);
                g.set(h, block);
                ptr(h, CDLIB_READ, &[table, block, 0, 0]);
                ptr(h, CDLIB_SYNC, &[0]);
            }
            ptr(h, CDLIB_TABLE, &[1]);
            let size = ptr(h, CDLIB_SIZE, &[0xd]);
            let cnf = call(h, HELP_MEM_GET, &[1, size.wrapping_add(4)]);
            SA_MENU_CNF.set(h, cnf);
            SA_MENU_CNF_SIZE.set(h, size);
            h.write(cnf, 0u8);
            let cnf = SA_MENU_CNF.get(h);
            ptr(h, CDLIB_READ, &[0xd, cnf, 0, 0]);
            ptr(h, CDLIB_SYNC, &[0]);
            let (cnf, size) = (SA_MENU_CNF.get(h), SA_MENU_CNF_SIZE.get(h));
            h.write(size.wrapping_add(cnf), 0u8);
            let dashes = h.rebase(STR_DASHES);
            call(h, SA_DBG_PRINTF, &[dashes]);
            let fmt = h.rebase(STR_CNF);
            call(h, SA_DBG_PRINTF, &[fmt]);
            let (fmt, cnf) = (h.rebase(STR_S), SA_MENU_CNF.get(h));
            call(h, SA_DBG_PRINTF, &[fmt, cnf]);
            call(h, SA_DBG_PRINTF, &[dashes]);
            let module = SA_MENU_MODULE.get(h);
            ptr(h, CDLIB_TABLE, &[module]);
        } else {
            let size = ptr(h, CDLIB_SIZE, &[0]);
            let fmt = h.rebase(STR_SPS2_SIZE);
            call(h, SA_DBG_PRINTF, &[fmt, size]);
            let block = call(h, HELP_MEM_GET, &[1, size]);
            SA_MENU_SPS2_BLOCK.set(h, block);
            ptr(h, CDLIB_READ, &[0, block, 0, 0]);
            ptr(h, CDLIB_SYNC, &[0]);
        }
        let sps2 = SA_MENU_SPS2.get(h);
        let size = ptr(h, CDLIB_SIZE, &[sps2]);
        let fmt = h.rebase(STR_SPS2_SIZE);
        call(h, SA_DBG_PRINTF, &[fmt, size]);
        let block = call(h, HELP_MEM_GET, &[1, size]);
        ML_BASE.set(h, block);
        let base = call(h, READ_SPS2, &[menu, 1]);
        ML_BASE.set(h, base);
        let texts = call(h, READ_SPS2, &[menu, 0]);
        ML_TEXT_BASE.set(h, texts);
        let work = call(h, HELP_MEM_GET, &[1, 0x42000]);
        SA_MENU_WORK.set(h, work);
        call(h, SET_FILE_TABLES, &[]);
        if menu == 4 {
            call(h, DISK_OPEN_CHECK_SET, &[1]);
            call(h, HDD_OPEN_PARTITION, &[]);
        }
        call(h, PATCH_MENU_RECORDS, &[]);
        let tail = if kind & 0x8000 != 0 {
            FIND_RECORD_AT
        } else {
            FIND_RECORD_FLAGGED
        };
        call(h, tail, &[])
    }

    // ---- chara-main-map round 1

    /// `Cache_Dispose(kind, id, check)` (`0x00840250`), called through the
    /// game.
    pub const CACHE_DISPOSE: u32 = 0x0084_0250;

    /// `Sg_CacheDisposeChr(id)` (`0x00840e60`; callers `ChN_DataDispose`,
    /// `Ch_BlkDisposeHandle`, `Native_4021`): the character's four motion
    /// groups (kind 2, keys `id | n << 16`, n 0..3) and its model data
    /// (kind 1, key `id`) out of the cache, checked. EAX is the last
    /// `Cache_Dispose`'s.
    pub fn cache_dispose_chr(h: &mut impl ffx_bridge::Host, id: u32) -> u32 {
        let f = h.rebase(CACHE_DISPOSE);
        for n in 0..4u32 {
            h.call(f, &[2, id | n << 16, 1]);
        }
        h.call(f, &[1, id, 1])
    }

    /// `Sg_EventjumpDispose`'s callees, all through the game: `dbgPrintf`,
    /// `Sg_MemSetFast(v)` (`0x0083c520`), `Ch_DisposeAll` (`0x008268e0`),
    /// `Ch_BlkDisposeAll` (`0x008259c0`), `Ch_DisposeAllMGRP`
    /// (`0x00836a80`), `Sg_PLightDispose` (its thunk `0x00831a30`), and
    /// the two log lines (`0x00b59e60`, `0x00b59e78`).
    pub const MEM_SET_FAST: u32 = 0x0083_c520;
    pub const CH_DISPOSE_ALL: u32 = 0x0082_68e0;
    pub const CH_BLK_DISPOSE_ALL: u32 = 0x0082_59c0;
    pub const CH_DISPOSE_ALL_MGRP: u32 = 0x0083_6a80;
    pub const PLIGHT_DISPOSE: u32 = 0x0083_1a30;
    pub const EVENTJUMP_DISPOSE_LOGS: [u32; 2] = [0x00b5_9e60, 0x00b5_9e78];

    /// `Sg_EventjumpDispose()` (`0x0083b420`; callers `AtelEventRemove`,
    /// `TkInitScene`): logs, `Sg_MemSetFast(0)`, then every character,
    /// model block and event motion group goes, and the point lights;
    /// logs again. EAX is the last `dbgPrintf`'s.
    pub fn eventjump_dispose(h: &mut impl ffx_bridge::Host) -> u32 {
        let f = h.rebase(SG_DBG_PRINTF);
        let log = h.rebase(EVENTJUMP_DISPOSE_LOGS[0]);
        h.call(f, &[log]);
        let f = h.rebase(MEM_SET_FAST);
        h.call(f, &[0]);
        for sym in [
            CH_DISPOSE_ALL,
            CH_BLK_DISPOSE_ALL,
            CH_DISPOSE_ALL_MGRP,
            PLIGHT_DISPOSE,
        ] {
            let f = h.rebase(sym);
            h.call(f, &[]);
        }
        let (f, log) = (h.rebase(SG_DBG_PRINTF), h.rebase(EVENTJUMP_DISPOSE_LOGS[1]));
        h.call(f, &[log])
    }

    /// The callees of [`eventjump_cache_chrs`], all through the game:
    /// `AtelGetBindpList(0)` (`0x0086ae20`), `Cache_ClearCheck`
    /// (`0x00840120`), `Cache_ToTop(kind, id)` (`0x00840cf0`),
    /// `Ch_RegMotionToTop` (`0x00837780`), `Cache_DisposeCheck`
    /// (`0x00840440`), `0x00840ab0`, `Ch_RegMotionUse` (`0x00837790`),
    /// `Ch_DataRead(id)` (`0x00829ef0`), `Ch_MgrpRead(id, 0)` (its thunk
    /// `0x00836a40`) and `Ch_ReadSync` (`0x0082a5e0`).
    pub const GET_BINDP_LIST: u32 = 0x0086_ae20;
    pub const CACHE_CLEAR_CHECK: u32 = 0x0084_0120;
    pub const CACHE_TO_TOP: u32 = 0x0084_0cf0;
    pub const REG_MOTION_TO_TOP: u32 = 0x0083_7780;
    pub const CACHE_DISPOSE_CHECK: u32 = 0x0084_0440;
    pub const CACHE_EMPTY: u32 = 0x0084_0ab0;
    pub const REG_MOTION_USE: u32 = 0x0083_7790;
    pub const CH_DATA_READ: u32 = 0x0082_9ef0;
    pub const CH_MGRP_READ: u32 = 0x0083_6a40;
    pub const CH_READ_SYNC: u32 = 0x0082_a5e0;

    /// `Sg_EventjumpCacheChrs_0083b670()` (name ours; caller
    /// `Sg_EventjumpInit`): the characters of the event's bind list
    /// (`AtelGetBindpList(0)`: `u16` count, then ids, walked from the last)
    /// stay in the cache: after `Cache_ClearCheck` their model data and
    /// motion groups go to the top (`Cache_ToTop(1, id)`, `(2, id)`), the
    /// resident motions too, the rest is dropped (`Cache_DisposeCheck`,
    /// `0x00840ab0`), then each is read again (`Ch_DataRead(id)`,
    /// `Ch_MgrpRead(id, 0)`), and the reads are waited for
    /// (`Ch_ReadSync(0)`, its `eax`).
    pub fn eventjump_cache_chrs(h: &mut impl ffx_bridge::Host) -> u32 {
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        let list = call(h, GET_BINDP_LIST, &[0]);
        call(h, CACHE_CLEAR_CHECK, &[]);
        let id = |h: &_, i: u32| u32::from(Mem::read::<u16>(h, list.wrapping_add(2 * i)));
        let mut i = u32::from(h.read::<u16>(list));
        while i != 0 {
            let v = id(h, i);
            call(h, CACHE_TO_TOP, &[1, v]);
            let v = id(h, i);
            call(h, CACHE_TO_TOP, &[2, v]);
            i -= 1;
        }
        for sym in [
            REG_MOTION_TO_TOP,
            CACHE_DISPOSE_CHECK,
            CACHE_EMPTY,
            REG_MOTION_USE,
        ] {
            call(h, sym, &[]);
        }
        let mut i = u32::from(h.read::<u16>(list));
        while i != 0 {
            let v = id(h, i);
            call(h, CH_DATA_READ, &[v]);
            let v = id(h, i);
            call(h, CH_MGRP_READ, &[v, 0]);
            i -= 1;
        }
        call(h, CH_READ_SYNC, &[0])
    }
    // ---- chara-main-map round 2

    /// The callees of [`chr_move_push_apart`], through the game:
    /// `Sg_IdChrMove(chr)`, `Sg_ChrSortByZ(n)`, `Sg_VectorGetLengthXZ(a,
    /// b)` (`ST0`), `Sg_ChrPushApart(a, b, reach, dist)` and
    /// `Atel_RequestOwnerTrigger_008764a0(chr, kind)`.
    pub const ID_CHR_MOVE: u32 = 0x0083_e5f0;
    pub const CHR_SORT_BY_Z: u32 = 0x0083_eb30;
    pub const VECTOR_GET_LENGTH_XZ: u32 = 0x0083_e3e0;
    pub const CHR_PUSH_APART: u32 = 0x0083_ecd0;
    pub const REQUEST_OWNER_TRIGGER: u32 = 0x0087_64a0;
    /// Its constants, `f64` 0.5 and 2.
    pub const PUSH_HALF: Global<f64> = Global::new(0x00b9_2220);
    pub const PUSH_TWO: Global<f64> = Global::new(0x00b9_2270);

    /// A character the pass takes: active (`+2`) and not hidden (`+0x180`
    /// clear, or `+0x181` set).
    fn push_visible(h: &impl Mem, c: u32) -> bool {
        h.read::<u8>(c.wrapping_add(2)) != 0
            && (h.read::<u8>(c.wrapping_add(0x180)) == 0
                || h.read::<u8>(c.wrapping_add(0x181)) != 0)
    }

    /// `Sg_ChrMovePushApart_0083d3b0()` (name ours; caller `Ch_CalcMain`):
    /// each frame's moves of the field characters, then keeping them
    /// apart. Over the characters the pass takes ([`push_visible`]; the
    /// array and count are read again only after one it takes):
    ///
    /// 1. Their "pushed" flag (`+0x194` bit 14) is cleared.
    /// 2. Up to two rounds: those without bit 8 are listed in
    ///    [`PUSH_CHRS`]; each moves by its step (`+0x4c`: with bit 7 all
    ///    three, else `Sg_IdChrMove` and the step's y), the step is zeroed,
    ///    its reach (`+0x4e8`) is `(+0x64 + +0x5c) * 0.5 * +0x4e4` and its
    ///    height above its floor (`+0x520`) `y - +0x51c`, or `+0x6a8` when
    ///    that is 0. The list sorted by z, each pair closer than their
    ///    reaches in the z and x boxes (2 reaches each way; past the first
    ///    in z ends that one's pairs), and for two climbing characters (bit
    ///    30) with overlapping heights (`+0x520` within the other's
    ///    `+0x698`; NaNs pass), whose reaches' sum (`f32`) is above their
    ///    distance (`Sg_VectorGetLengthXZ`; NaN too) is pushed apart
    ///    (`Sg_ChrPushApart`); a pusher (bit 9, `+0x154` not 0) marks the
    ///    other pushed. The count is read again after every call. A second
    ///    round runs only when something moved.
    /// 3. A newly pushed character (bit 14 without bit 15) gets its
    ///    owner's trigger 2 (`Atel_RequestOwnerTrigger`), and bit 15 takes
    ///    bit 14.
    ///
    /// No caller reads `eax`.
    #[allow(clippy::too_many_lines, clippy::float_cmp)]
    pub fn chr_move_push_apart<H: ffx_bridge::Host>(
        h: &mut H,
        fcall: &mut impl FnMut(&mut H, u32, &[u32]) -> f64,
    ) {
        use crate::seq::bridge::{CHR_ARRAY, CHR_COUNT};
        use ffx_bridge::x87::Reg;
        let call = |h: &mut H, f: u32, args: &[u32]| {
            let f = h.rebase(f);
            h.call(f, args)
        };
        let ld = |h: &H, a: u32| Reg::load(h.read::<f32>(a));
        let flags = |h: &H, c: u32| h.read::<u32>(c.wrapping_add(0x194));
        let end = |count: u32, array: u32| array.wrapping_add(count.wrapping_mul(0x880));
        let list = h.rebase(PUSH_CHRS);
        let slot = |i: i32| list.wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let (mut count, mut array) = (CHR_COUNT.get(h), CHR_ARRAY.get(h));
        let mut c = array;
        while c != end(count, array) {
            if push_visible(h, c) {
                let f = flags(h, c);
                h.write(c.wrapping_add(0x194), f & !0x4000);
                (array, count) = (CHR_ARRAY.get(h), CHR_COUNT.get(h));
            }
            c = c.wrapping_add(0x880);
        }
        let mut round = 0;
        loop {
            let mut n = 0i32;
            PUSH_CHR_COUNT.set(h, 0);
            let mut c = array;
            while c != end(count, array) {
                if push_visible(h, c) {
                    if flags(h, c) & 0x100 == 0 {
                        h.write(slot(n), c);
                        n += 1;
                        PUSH_CHR_COUNT.set(h, n);
                    }
                    let at = |o: u32| c.wrapping_add(o);
                    if h.read::<u8>(at(0x194)) & 0x80 != 0 {
                        for k in 0..3 {
                            let v = ld(h, at(0x4c + 4 * k)).add_m32(h.read(at(0xc + 4 * k)));
                            h.write(at(0xc + 4 * k), v.store());
                        }
                    } else {
                        call(h, ID_CHR_MOVE, &[c]);
                        let v = ld(h, at(0x50)).add_m32(h.read(at(0x10)));
                        h.write(at(0x10), v.store());
                        n = PUSH_CHR_COUNT.get(h);
                    }
                    for o in [0x54u32, 0x50, 0x4c] {
                        h.write(at(o), 0f32);
                    }
                    let r = ld(h, at(0x64)).add_m32(h.read(at(0x5c)));
                    let r = (r * Reg::constant(PUSH_HALF.get(h))).mul_m32(h.read(at(0x4e4)));
                    h.write(at(0x4e8), r.store());
                    let floor: f32 = h.read(at(0x51c));
                    let height = if floor == 0.0 {
                        ld(h, at(0x6a8)).store()
                    } else {
                        ld(h, at(0x10)).sub_m32(floor).store()
                    };
                    h.write(at(0x520), height);
                    (array, count) = (CHR_ARRAY.get(h), CHR_COUNT.get(h));
                }
                c = c.wrapping_add(0x880);
            }
            let mut moved = 0u32;
            call(h, CHR_SORT_BY_Z, &[n.cast_unsigned()]);
            if n > 0 {
                let two = Reg::constant(PUSH_TWO.get(h));
                let mut i = 0i32;
                loop {
                    let a: u32 = h.read(slot(i));
                    let mut j = i + 1;
                    while j < n {
                        let b: u32 = h.read(slot(j));
                        let ra = ld(h, a.wrapping_add(0x4e8)) * two;
                        let rb = two.mul_m32(h.read(b.wrapping_add(0x4e8)));
                        // past a's box in z: no later one reaches it either
                        let az = ld(h, a.wrapping_add(0x14)) + ra;
                        let bz = ld(h, b.wrapping_add(0x14)) - rb;
                        if bz.value() > az.value() {
                            break;
                        }
                        j += 1;
                        let (ax, bx) = (ld(h, a.wrapping_add(0xc)), ld(h, b.wrapping_add(0xc)));
                        if (bx - rb).value() > (ax + ra).value()
                            || (bx + rb).value() < (ax - ra).value()
                        {
                            continue;
                        }
                        if flags(h, a) & 0x4000_0000 != 0 && flags(h, b) & 0x4000_0000 != 0 {
                            let (a520, b698) =
                                (ld(h, a.wrapping_add(0x520)), ld(h, b.wrapping_add(0x698)));
                            let (a698, b520) =
                                (ld(h, a.wrapping_add(0x698)), ld(h, b.wrapping_add(0x520)));
                            if b698.value() < a520.value() || b520.value() > a698.value() {
                                continue;
                            }
                        }
                        let reach = ld(h, a.wrapping_add(0x4e8))
                            .add_m32(h.read(b.wrapping_add(0x4e8)))
                            .store();
                        let (pa, pb) = (a.wrapping_add(0xc), b.wrapping_add(0xc));
                        let f = h.rebase(VECTOR_GET_LENGTH_XZ);
                        let dist = Reg::constant(fcall(h, f, &[pa, pb])).store();
                        // on when reach > dist, or either is NaN
                        if reach <= dist {
                            continue;
                        }
                        moved = moved.wrapping_add(call(
                            h,
                            CHR_PUSH_APART,
                            &[a, b, reach.to_bits(), dist.to_bits()],
                        ));
                        for (p, q) in [(a, b), (b, a)] {
                            if flags(h, p) & 0x200 != 0
                                && h.read::<f32>(p.wrapping_add(0x154)) != 0.0
                            {
                                let f = flags(h, q);
                                h.write(q.wrapping_add(0x194), f | 0x4000);
                            }
                        }
                        n = PUSH_CHR_COUNT.get(h);
                    }
                    i += 1;
                    if i >= n {
                        break;
                    }
                }
                array = CHR_ARRAY.get(h);
            }
            round += 1;
            if moved == 0 {
                break;
            }
            count = CHR_COUNT.get(h);
            if round >= 2 {
                break;
            }
        }
        count = CHR_COUNT.get(h);
        let mut c = array;
        while c != end(count, array) {
            if push_visible(h, c) {
                let f = flags(h, c);
                if f & 0x4000 != 0 && f & 0x8000 == 0 {
                    call(h, REQUEST_OWNER_TRIGGER, &[c, 2]);
                }
                let f = flags(h, c);
                h.write(c.wrapping_add(0x194), (((f << 1) ^ f) & 0x8000) ^ f);
                (array, count) = (CHR_ARRAY.get(h), CHR_COUNT.get(h));
            }
            c = c.wrapping_add(0x880);
        }
    }

    /// The callees of [`eventjump_init`] not named above, through the game.
    pub const GET_SCREEN_FADE_COLOR: u32 = 0x0064_1390;
    pub const SET_SCREEN_FADE_COLOR: u32 = 0x0064_3900;
    pub const SET_SCREEN_FADE_LAST_REASON: u32 = 0x0064_3940;
    pub const LOCK_SCREEN_FADE_COLOR: u32 = 0x0064_1e20;
    /// The resets and switches it calls in order, each with its one word
    /// (`None`: no argument), up to `Sg_SetMapAlphaFast(1)` and
    /// `Ch_DrawSetShadowSemi(0)` (`SetByte_012fb8fd`).
    pub const EVENTJUMP_INIT_FIRST: [(u32, Option<u32>); 12] = [
        (0x0083_c520, Some(0)), // Sg_MemSetFast
        (0x0082_cc70, None),    // Sg_FilterReset
        (0x0082_cce0, None),    // Sg_FlashReset
        (0x0082_ccf0, None),    // Sg_FocusReset
        (0x0082_bcd0, None),    // Sg_AccInitAll
        (0x0082_0440, Some(1)), // Sg_EnableCalcMap
        (0x0082_0400, Some(1)), // Sg_EnableDrawMap
        (0x0082_0460, Some(1)), // Sg_EnableDrawMenu
        (0x0082_03e0, Some(1)), // Sg_EnableCalcChr
        (0x0082_03c0, Some(1)), // Sg_EnableDrawChr
        (0x0082_1b00, Some(1)), // Sg_SetMapAlphaFast
        (0x0082_1b20, Some(0)), // SetByte_012fb8fd (Ch_DrawSetShadowSemi)
    ];
    pub const PLAYER_SET_DIR_FIX: u32 = 0x0082_d990;
    pub const CH_SYS_DRAW_SET_SITUATION: u32 = 0x0083_0cd0;
    pub const SET_SUIMEN: u32 = 0x0083_6640;
    pub const SET_WATER_LIMIT: u32 = 0x0083_6660;
    pub const LIGHT_SET: u32 = 0x0083_6520;
    /// The four default lights (`Light_Set(i, r, g, b, pitch, yaw)`).
    pub const EVENTJUMP_LIGHTS: [[u32; 6]; 4] = [
        [0, 0x53, 0x53, 0x53, 0x82, 0x4c],
        [1, 0x37, 0x37, 0x37, 0x136, 0xe],
        [2, 0x12, 0x12, 0xd, 0x82, 0x11c],
        [3, 0x50, 0x5a, 0x69, 0, 0],
    ];
    /// The calls after the lights, in order, with their words.
    pub const EVENTJUMP_INIT_LAST: [(u32, &[u32]); 9] = [
        (0x0082_dab0, &[1]),          // Sg_SetCameraFollow
        (0x0082_4e10, &[]),           // Sg_GlobalInitWind
        (0x0083_0150, &[1]),          // Sg_SetMovieGomiAvoid
        (0x0083_0100, &[0, 0, 0]),    // Ch_DrawSetShadowColor
        (0x0083_0120, &[0xe1, 0x2d]), // Ch_DrawSetShadowDir
        (0x0083_e8d0, &[]),           // Sg_IdEventInitAll
        (0x0082_d8d0, &[]),           // Sg_PlayerInitDefault
        (0x0082_1ae0, &[1]),          // Sg_SetKeepFps
        (0x0083_b670, &[]),           // Sg_EventjumpCacheChrs_0083b670
    ];
    pub const CH_ALLOCATE_INIT_ALL: u32 = 0x0082_5670;
    /// The water's default height and limit, `f32` 1e6.
    pub const NO_WATER: Global<f32> = Global::new(0x00b2_be08);
    /// `"SG:EventjumpInit %d-----\n"` and `"SG:done.-----\n"`.
    pub const EVENTJUMP_INIT_LOGS: [u32; 2] = [0x00b5_9e94, 0x00b5_9eb0];

    /// `Sg_EventjumpInit(n)` (`0x0083b460`; callers `AtelActorSetUp2`,
    /// `AtelCharSetUp`): a new event's scene state. Logs `n`; the screen
    /// fade keeps its colour (`graphicGetScreenFadeColor` into `scratch`,
    /// no alpha; the original's blue lands in its own argument slot, which
    /// neither caller reads) at full alpha, reason 2, locked; the filters,
    /// flash, focus and accumulators reset, the map, menu and characters
    /// calculated and drawn, the map opaque, no shadow semi-transparency,
    /// no player direction fix (0, 0.0), situation 0, no water (1e6 both),
    /// the default lights, camera follow, wind, movie filter, black
    /// shadows from (0xe1, 0x2d), the event ids and player defaults, frame
    /// keeping, the bind list's characters cached, and `n` characters (at
    /// least 10) allocated. Logs again: that `dbgPrintf`'s `eax`.
    pub fn eventjump_init(h: &mut impl ffx_bridge::Host, n: u32, scratch: u32) -> u32 {
        use ffx_bridge::x87::Reg;
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        let log = h.rebase(EVENTJUMP_INIT_LOGS[0]);
        call(h, SG_DBG_PRINTF, &[log, n]);
        let (r, g, b) = (scratch, scratch + 4, scratch + 8);
        call(h, GET_SCREEN_FADE_COLOR, &[r, g, b, 0]);
        // fld / fstp dword: the colour as the x87 copies it
        let rgb = [r, g, b].map(|p| Reg::load(h.read::<f32>(p)).store().to_bits());
        call(
            h,
            SET_SCREEN_FADE_COLOR,
            &[rgb[0], rgb[1], rgb[2], 1f32.to_bits()],
        );
        call(h, SET_SCREEN_FADE_LAST_REASON, &[2]);
        call(h, LOCK_SCREEN_FADE_COLOR, &[]);
        let n = n.cast_signed().max(10).cast_unsigned();
        for (f, arg) in EVENTJUMP_INIT_FIRST {
            match arg {
                Some(v) => call(h, f, &[v]),
                None => call(h, f, &[]),
            };
        }
        call(h, PLAYER_SET_DIR_FIX, &[0, 0f32.to_bits()]);
        call(h, CH_SYS_DRAW_SET_SITUATION, &[0]);
        let water = Reg::load(NO_WATER.get(h)).store().to_bits();
        call(h, SET_SUIMEN, &[water]);
        let water = Reg::load(NO_WATER.get(h)).store().to_bits();
        call(h, SET_WATER_LIMIT, &[water]);
        for light in EVENTJUMP_LIGHTS {
            call(h, LIGHT_SET, &light);
        }
        for (f, args) in EVENTJUMP_INIT_LAST {
            call(h, f, args);
        }
        call(h, CH_ALLOCATE_INIT_ALL, &[n]);
        let log = h.rebase(EVENTJUMP_INIT_LOGS[1]);
        call(h, SG_DBG_PRINTF, &[log])
    }

    /// `HandleSaveDataScreen`'s callees, through the game.
    pub mod save_screen {
        pub const IS_SCREEN_CLOSED: u32 = 0x0064_8100;
        pub const GET_FADE_LAST_REASON: u32 = 0x0064_13e0;
        pub const IS_LOADING_ALBHED: u32 = 0x0064_8070;
        pub const IS_FADE_COLOR_LOCKED: u32 = 0x0064_1cb0;
        pub const UNLOCK_FADE_COLOR: u32 = 0x0064_48d0;
        pub const SET_FADE_COLOR: u32 = 0x0064_3900;
        pub const SET_FADE_ALPHA_MODE: u32 = 0x0064_38e0;
        pub const SET_FADE_LAST_REASON: u32 = 0x0064_3940;
        pub const LOCK_FADE_COLOR: u32 = 0x0064_1e20;
        pub const IS_BUSY: u32 = 0x0064_8040;
        pub const INPUT_CLEAR: u32 = 0x0063_0c30;
        pub const GET_SCREEN_STATE: u32 = 0x0064_7f70;
        pub const TK_GET_CONTROLER_HIST: u32 = 0x0088_8d70;
        pub const IS_SAVING_FINISHED: u32 = 0x0064_80f0;
        pub const DEBUG_LOAD_ALL: u32 = 0x0064_92c0;
        pub const MEMSET_THUNK: u32 = 0x0094_964c;
        pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
        pub const SET_SAVE_DATA_INFO: u32 = 0x0064_86c0;
        pub const UPDATE_ICON: u32 = 0x0064_8920;
        pub const TO_SAVE: u32 = 0x0064_87a0;
        pub const SET_STATE: u32 = 0x0064_8710;
        pub const CLOSE_SAVE_LOAD_SCREEN: u32 = 0x008b_3db0;
        pub const SET_SCREEN_STATE: u32 = 0x0064_86e0;
        pub const IS_SAVE: u32 = 0x0064_80d0;
        pub const PRE_SAVE: u32 = 0x008b_50b0;
        pub const IS_SAVE_ENABLED: u32 = 0x0064_80e0;
        pub const TO_LOAD: u32 = 0x0064_8760;
        pub const IS_LOADING_FINISHED: u32 = 0x0064_8090;
        pub const GET_DIALOG_STATE: u32 = 0x0064_7ee0;
        pub const IS_LOADING_SUCCESSFUL: u32 = 0x0064_80a0;
        pub const IS_OPERATION_CANCELED: u32 = 0x0064_80c0;
        pub const IS_ALBHED_LOADED: u32 = 0x0082_2540;
        pub const MARK_ALBHED_LOADED: u32 = 0x0064_7e40;
        pub const IS_DELETING_FINISHED: u32 = 0x0064_8060;
        pub const RESET_TO_IDLE: u32 = 0x0064_8530;
        pub const CLEAR_FORCE_PLACE_ID: u32 = 0x0086_38b0;
        pub const LOADED_CHOICE: u32 = 0x0082_2480;
        pub const JUMP_TO_LOADED_SCENE: u32 = 0x008b_4ec0;
        /// `"Description: "`, `"%s%d"`, `"Save Data "` and the title
        /// (`0x00b590c8`) of the debug saves.
        pub const DESCRIPTION: u32 = 0x00b5_90a4;
        pub const FORMAT: u32 = 0x00b5_90b4;
        pub const SAVE_DATA: u32 = 0x00b5_90bc;
        pub const TITLE: u32 = 0x00b5_90c8;
    }
    /// `i32`: the debug save-everything slot (-1 off; a pressed button on
    /// the save screen starts it at 0, 200 slots).
    pub const SAVE_DEBUG_SLOT: Global<i32> = Global::new(0x00c4_94cc);
    /// `u32`: set, a finished delete closes the screen instead of saving
    /// again (cleared then; nothing sets it).
    pub const SAVE_CLOSE_AFTER_DELETE: Global<u32> = Global::new(0x012f_b7c8);
    /// `u32` `g_SaveLoadedPending`: 1 after a successful load.
    pub const SAVE_LOADED_PENDING: Global<u32> = Global::new(0x012f_b8c0);
    /// Bytes of scratch [`handle_save_data_screen`] needs: the debug
    /// description (`[ebp-0x44]`).
    pub const SAVE_SCREEN_SCRATCH: u32 = 0x40;

    /// `HandleSaveDataScreen()` (`0x0081ff80`; caller `Sg_MainLoop`): the
    /// save/load screen's frame while it is open. Without a fade reason
    /// (and no Al Bhed load) the fade becomes black, opaque, reason 1,
    /// locked; input is cleared while saving is busy. By screen state:
    /// - 1: a button (`TkGetControlerHist(0, 0, 0)` negative) starts the
    ///   debug save of every slot; that runs a slot per frame once the last
    ///   save finished (`debugSavedataLoadAll(slot)`: 0 saves it as
    ///   `"Description: <slot>"`, else data state 2) up to 200, then
    ///   closes (state 3). Otherwise a save (`TkMenuPreSave`; not enabled:
    ///   state 3) or a load starts, state 2. Input is cleared.
    /// - 2: a finished load with no dialog closes the screen when it failed
    ///   or was cancelled, else marks the load pending (when the Al Bhed
    ///   check gives -1); a finished save closes it; either way state 3. A
    ///   finished delete saves again, or with [`SAVE_CLOSE_AFTER_DELETE`]
    ///   (cleared) goes to state 3.
    /// - 3: back to idle, state 4, no forced place.
    /// - 4: with a load pending, the choice (`0x00822480`) 1 reopens the
    ///   screen (state 1), 2 jumps to the loaded scene (state 0); without,
    ///   state 0.
    ///
    /// No caller reads `eax`.
    #[allow(clippy::too_many_lines)]
    pub fn handle_save_data_screen(h: &mut impl ffx_bridge::Host, scratch: u32) {
        use save_screen as s;
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        if call(h, s::IS_SCREEN_CLOSED, &[]) != 0 {
            return;
        }
        if call(h, s::GET_FADE_LAST_REASON, &[]) == 0 && call(h, s::IS_LOADING_ALBHED, &[]) == 0 {
            if call(h, s::IS_FADE_COLOR_LOCKED, &[]) != 0 {
                call(h, s::UNLOCK_FADE_COLOR, &[]);
            }
            call(h, s::SET_FADE_COLOR, &[0, 0, 0, 1f32.to_bits()]);
            call(h, s::SET_FADE_ALPHA_MODE, &[0]);
            call(h, s::SET_FADE_LAST_REASON, &[1]);
            call(h, s::LOCK_FADE_COLOR, &[]);
        }
        if call(h, s::IS_BUSY, &[]) != 0 {
            call(h, s::INPUT_CLEAR, &[]);
        }
        if call(h, s::GET_SCREEN_STATE, &[]) == 1 {
            if lo8(call(h, s::TK_GET_CONTROLER_HIST, &[0, 0, 0])).cast_signed() < 0 {
                SAVE_DEBUG_SLOT.set(h, 0);
                call(h, s::INPUT_CLEAR, &[]);
                return;
            }
            let slot = SAVE_DEBUG_SLOT.get(h);
            if slot < 0 {
                if call(h, s::IS_SAVE, &[]) == 0 {
                    call(h, s::TO_LOAD, &[]);
                } else {
                    call(h, s::PRE_SAVE, &[]);
                    call(h, s::TO_SAVE, &[]);
                    if call(h, s::IS_SAVE_ENABLED, &[]) == 0 {
                        call(h, s::SET_SCREEN_STATE, &[3]);
                        call(h, s::INPUT_CLEAR, &[]);
                        return;
                    }
                }
                call(h, s::SET_SCREEN_STATE, &[2]);
            } else if slot == 0 || call(h, s::IS_SAVING_FINISHED, &[]) != 0 {
                let slot = SAVE_DEBUG_SLOT.get(h).cast_unsigned();
                if call(h, s::DEBUG_LOAD_ALL, &[slot]) == 0 {
                    call(h, s::MEMSET_THUNK, &[scratch, 0, 0x40]);
                    let sprintf: u32 = h.read(h.rebase(s::SPRINTF_SLOT));
                    let slot = SAVE_DEBUG_SLOT.get(h).cast_unsigned();
                    let (fmt, desc) = (h.rebase(s::FORMAT), h.rebase(s::DESCRIPTION));
                    ffx_bridge::Host::call(h, sprintf, &[scratch, fmt, desc, slot]);
                    let (title, name) = (h.rebase(s::TITLE), h.rebase(s::SAVE_DATA));
                    call(h, s::SET_SAVE_DATA_INFO, &[title, name, scratch]);
                    call(h, s::UPDATE_ICON, &[0]);
                    call(h, s::TO_SAVE, &[]);
                } else {
                    call(h, s::SET_STATE, &[2]);
                }
                let slot = SAVE_DEBUG_SLOT.get(h).wrapping_add(1);
                SAVE_DEBUG_SLOT.set(h, slot);
                if slot >= 200 {
                    SAVE_DEBUG_SLOT.set(h, -1);
                    call(h, s::CLOSE_SAVE_LOAD_SCREEN, &[]);
                    call(h, s::SET_SCREEN_STATE, &[3]);
                    call(h, s::INPUT_CLEAR, &[]);
                    return;
                }
            }
            call(h, s::INPUT_CLEAR, &[]);
            return;
        }
        let next = match call(h, s::GET_SCREEN_STATE, &[]) {
            2 => {
                if call(h, s::IS_LOADING_FINISHED, &[]) != 0
                    && call(h, s::GET_DIALOG_STATE, &[]) == 0
                {
                    if call(h, s::IS_LOADING_SUCCESSFUL, &[]) == 0
                        || call(h, s::IS_OPERATION_CANCELED, &[]) != 0
                    {
                        call(h, s::CLOSE_SAVE_LOAD_SCREEN, &[]);
                    } else if call(h, s::IS_ALBHED_LOADED, &[]) == u32::MAX {
                        SAVE_LOADED_PENDING.set(h, 1);
                        call(h, s::MARK_ALBHED_LOADED, &[]);
                    }
                    call(h, s::SET_SCREEN_STATE, &[3]);
                }
                if call(h, s::IS_SAVING_FINISHED, &[]) != 0
                    && call(h, s::GET_DIALOG_STATE, &[]) == 0
                {
                    call(h, s::CLOSE_SAVE_LOAD_SCREEN, &[]);
                    call(h, s::SET_SCREEN_STATE, &[3]);
                }
                if call(h, s::IS_DELETING_FINISHED, &[]) == 0
                    || call(h, s::GET_DIALOG_STATE, &[]) != 0
                {
                    return;
                }
                if SAVE_CLOSE_AFTER_DELETE.get(h) == 0 {
                    call(h, s::TO_SAVE, &[]);
                    return;
                }
                SAVE_CLOSE_AFTER_DELETE.set(h, 0);
                3
            }
            _ if call(h, s::GET_SCREEN_STATE, &[]) == 3 => {
                call(h, s::RESET_TO_IDLE, &[]);
                call(h, s::SET_SCREEN_STATE, &[4]);
                call(h, s::CLEAR_FORCE_PLACE_ID, &[]);
                return;
            }
            _ => {
                if call(h, s::GET_SCREEN_STATE, &[]) != 4 {
                    return;
                }
                if SAVE_LOADED_PENDING.get(h) != 0 {
                    match call(h, s::LOADED_CHOICE, &[]) {
                        1 => {
                            call(h, s::SET_SCREEN_STATE, &[1]);
                            SAVE_LOADED_PENDING.set(h, 0);
                        }
                        2 => {
                            call(h, s::SET_SCREEN_STATE, &[0]);
                            call(h, s::JUMP_TO_LOADED_SCENE, &[]);
                            SAVE_LOADED_PENDING.set(h, 0);
                        }
                        _ => {}
                    }
                    return;
                }
                0
            }
        };
        call(h, s::SET_SCREEN_STATE, &[next]);
    }
}
