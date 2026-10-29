//! M5 `main`: the game's frame, `Sg_MainLoop` (update and draw), its
//! caller `updateFFX`, and `FFXApplication::animate` around them (the PC
//! application's update, where the harness's lockstep holds the game).
//!
//! Every callee stays a call (most are ports already, reached through
//! their hooks). The order of calls and of the reads between them is the
//! original's: callees change the globals the loop tests next.

/// `Sg_MainLoop`'s globals (`u32` unless noted).
pub mod g {
    /// `g_PauseOn`.
    pub const PAUSE_ON: u32 = 0x0134_0828;
    /// The scene-change request the menu check takes.
    pub const SCENE_REQUEST: u32 = 0x0134_0824;
    /// Cleared when `Sg_ReturnConst1` answers (the debug hold).
    pub const DEBUG_HOLD: u32 = 0x0134_082c;
    /// The scene request at the last menu step.
    pub const SCENE_LAST: u32 = 0x0134_0830;
    /// The menu-fade state: 1 = the fade is locked.
    pub const MENU_FADE: u32 = 0x0134_0848;
    /// The reset request (1).
    pub const RESET_REQUEST: u32 = 0x0134_0844;
    /// `g_VoiceHold`: frames to repeat the update (above 1).
    pub const VOICE_HOLD: u32 = 0x012f_b808;
    /// The three "update repeated" flags.
    pub const REPEAT_A: u32 = 0x012f_b7d0;
    pub const REPEAT_B: u32 = 0x012f_b7d4;
    pub const REPEAT_C: u32 = 0x012f_b7d8;
    /// The dynamic geometry's swap state.
    pub const DYN_GEO_STATE: u32 = 0x012f_b7dc;
    /// The field and character draw switches (bytes).
    pub const FIELD_ON: u32 = 0x012f_bbb1;
    pub const FIELD_DRAW: u32 = 0x012f_bbb9;
    pub const CHARA_ON: u32 = 0x012f_bbb2;
    pub const CHARA_DRAW: u32 = 0x012f_bbba;
    pub const FIELD_FIRST: u32 = 0x012f_bbb6;
    pub const MOVIE_ON: u32 = 0x012f_b8fb;
    pub const MOVIE_LAST: u32 = 0x012f_b8fc;
    pub const MOVIE_HOLD: u32 = 0x012f_b8fd;
    pub const CHARA_CALC: u32 = 0x012f_b8ff;
    pub const PART_MATRIX: u32 = 0x012f_b8f9;
    pub const CAMERA_IDLE: u32 = 0x012f_bbb0;
    pub const SCREEN_LAST: u32 = 0x012f_bbb5;
    pub const EFFECT_ON: u32 = 0x012f_bbb8;
    pub const PARTICLE_ON: u32 = 0x012f_bbb7;
    pub const MESSAGE_ON: u32 = 0x012f_bbb4;
    /// `g_SgDebugOn` (byte).
    pub const DEBUG_ON: u32 = 0x0133_c910;
    /// The display flags (bit 24: the distortion pass).
    pub const DISP_FLAGS: u32 = 0x018d_ed08;
    /// The interface `ExportValidateInterface` checks.
    pub const INTERFACE: u32 = 0x012f_bc38;
    /// The current map (for `isEternalCalmScene`).
    pub const MAP: u32 = 0x012f_bc40;
    /// The scenario flag now and at the last frame.
    pub const SCENARIO: u32 = 0x00c4_949c;
    pub const SCENARIO_LAST: u32 = 0x00c4_94a0;
    /// The airship byte (`0x0112d6c9`) and its achievement state.
    pub const AIRSHIP: u32 = 0x0112_d6c9;
    pub const AIRSHIP_DONE: u32 = 0x00c4_94a4;
    /// A block freed when it stays the same two frames.
    pub const FREE_NEXT: u32 = 0x012e_9624;
    pub const FREE_SEEN: u32 = 0x012e_9628;
    /// The effect pair to destroy (state, hold).
    pub const VFX_DESTROY: u32 = 0x012e_9644;
    pub const VFX_HOLD: u32 = 0x012e_9640;
    /// The player-name reset request.
    pub const NAME_RESET: u32 = 0x0113_3350;
    /// The main menu's water.
    pub const MENU_WATER: u32 = 0x025d_5ebc;
    /// The Eternal Calm copyright's state and frame count.
    pub const CALM_STATE: u32 = 0x0132_7108;
    pub const CALM_FRAMES: u32 = 0x012f_b7c0;
    /// The draw buffer index.
    pub const DRAW_BUFFER: u32 = 0x023c_bbf0;
    /// The double buffers `sceGsSwapDBuffDc` swaps.
    pub const DBUFF: u32 = 0x012f_b900;
    /// The scenario number printed last (`u16`).
    pub const SCENARIO_PRINTED: u32 = 0x012f_b7c4;
    /// `1.0`-ish scale for the field (`f32`, `.rdata`) and the
    /// characters' step (`f32`, `.rdata`).
    pub const FIELD_SCALE: u32 = 0x00b9_2318;
    pub const CHARA_STEP: u32 = 0x00b5_9158;
    /// The inverse matrices (`0x0230fca0` camera, `0x0230fc60` screen).
    pub const INV_CAMERA: u32 = 0x0230_fca0;
    pub const INV_SCREEN: u32 = 0x0230_fc60;
    /// Profiler marker names.
    pub const MARK_A: u32 = 0x00b5_9104;
    pub const MARK_B: u32 = 0x00b7_c984;
    pub const MARK_C: u32 = 0x00b7_c988;
    pub const MARK_D: u32 = 0x00b7_c99c;
    pub const MARK_E: u32 = 0x00b5_9110;
    /// `"scenario %d\n"`-style format for the printed scenario.
    pub const FMT_SCENARIO: u32 = 0x00b5_911c;
    /// `MSVCR110!__iob_func` and `fprintf` (cdecl) import slots.
    pub const IOB_FUNC_IAT: u32 = 0x00b0_c350;
    pub const FPRINTF_IAT: u32 = 0x00b0_c38c;
}

/// `Sg_MainLoop`'s callees (cdecl unless noted).
pub mod f {
    pub const SPD_SCALE: u32 = 0x006f_7340;
    pub const USE_EXTENDED_ALPHA: u32 = 0x0071_e460;
    pub const SCENE_CLEAR_LIST: u32 = 0x0063_dac0;
    pub const PS2_LOADING_CALLBACK: u32 = 0x0064_27f0;
    pub const MODEL_MEM_USAGE: u32 = 0x0072_c1a0;
    pub const VIDEO_UPDATE: u32 = 0x0064_5df0;
    pub const FREE_CAMERA_TYPE: u32 = 0x0064_11e0;
    pub const GET_DISP_MODE: u32 = 0x0082_0550;
    pub const SET_DISTORTION_SKIPPED: u32 = 0x0064_2f80;
    pub const TK_UPDATE_SCAN: u32 = 0x0088_9a80;
    pub const FRAME_SYNC: u32 = 0x0082_1e80;
    pub const TK_SCAN: u32 = 0x0088_93e0;
    pub const VFX_UPDATE_DELETED: u32 = 0x0064_5a40;
    pub const CHR_INSTANTIATE: u32 = 0x0063_9db0;
    pub const CHR_DELAY_RELEASE: u32 = 0x0064_42d0;
    pub const STUB_639380: u32 = 0x0063_9380;
    pub const SAVE_UPDATE: u32 = 0x0064_8480;
    pub const SAVE_TAKE_LOADED: u32 = 0x0064_8010;
    pub const GET_U32_01325AC4: u32 = 0x0064_7c90;
    pub const ALBHED_DIC: u32 = 0x0086_e820;
    pub const SAVE_SCREEN: u32 = 0x0081_ff80;
    pub const RETURN_CONST1: u32 = 0x0082_02d0;
    pub const CONTROLER_HIST: u32 = 0x0088_8d70;
    pub const RESET_NAME: u32 = 0x0078_73a0;
    pub const CLEAR_SPECIAL: u32 = 0x0064_4dd0;
    pub const CHECK_MENU: u32 = 0x0082_0750;
    pub const SAVE_SCREEN_CLOSED: u32 = 0x0064_8100;
    pub const SCENE_LOADING: u32 = 0x0081_fcd0;
    pub const DYN_GEO_SWAP: u32 = 0x0063_ff70;
    pub const ASSETS_LOADED: u32 = 0x0064_1ba0;
    pub const FADE_LAST_REASON: u32 = 0x0064_13e0;
    pub const GET_FADE_COLOR: u32 = 0x0064_1390;
    pub const FADE_LOCKED: u32 = 0x0064_1cb0;
    pub const UNLOCK_FADE: u32 = 0x0064_48d0;
    pub const SET_FADE_COLOR: u32 = 0x0064_3900;
    pub const SET_FADE_ALPHA_MODE: u32 = 0x0064_38e0;
    pub const LOCK_FADE: u32 = 0x0064_1e20;
    pub const SET_FADE_LAST_REASON: u32 = 0x0064_3940;
    pub const PROFILER_MARKER: u32 = 0x00a4_4840;
    pub const CONTROL_MAIN: u32 = 0x0082_d070;
    pub const VALIDATE_INTERFACE: u32 = 0x0082_02e0;
    pub const MENU_RESET: u32 = 0x0064_2320;
    pub const SCENARIO_FLAG: u32 = 0x0086_c470;
    pub const ACHIEVEMENT: u32 = 0x0082_2380;
    pub const CALC_RATE: u32 = 0x0082_06c0;
    pub const INIT_FONT: u32 = 0x0082_1aa0;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const VFX_DESTROY: u32 = 0x0064_5280;
    pub const CLEAR_FLAGS24: u32 = 0x0083_2db0;
    pub const EXEC_FRAMES: u32 = 0x0082_d7e0;
    pub const CONTROL_PLAYER: u32 = 0x0082_d180;
    pub const BATTLE_PROCESS: u32 = 0x0079_0c10;
    pub const EVENT_JUMP: u32 = 0x0088_d3d0;
    pub const MESSAGE_WINDOW: u32 = 0x008a_b910;
    pub const COUNTERS_FLUSH: u32 = 0x0082_1a40;
    pub const OP_LOOP_BEFORE: u32 = 0x007f_f770;
    pub const EFFECT_PROCESS: u32 = 0x0078_7e00;
    pub const CAMERA_PROCESS: u32 = 0x007b_e110;
    pub const FIELD_PROC: u32 = 0x0091_a8c0;
    pub const CALC_MAIN: u32 = 0x0083_2e10;
    pub const CAMERA_SET_PLAYER: u32 = 0x0083_f2e0;
    pub const CALC_MIDDLE: u32 = 0x0083_3390;
    pub const YI_ANIM: u32 = 0x0090_8320;
    pub const MOVIE_CAMERA: u32 = 0x0076_edc0;
    pub const VIDEO_PLAY: u32 = 0x0071_85e0;
    pub const TEXTURE_VIDEO: u32 = 0x0064_42c0;
    pub const SET_DISP_DRAW_ENV: u32 = 0x0082_17d0;
    pub const ENV_INIT_MOVIE: u32 = 0x0082_caa0;
    pub const ENV_INIT: u32 = 0x0082_c8d0;
    pub const SET_CHADIST: u32 = 0x0091_a250;
    pub const MFIFO_RESET: u32 = 0x0084_32e0;
    pub const CAMERA_PUT_RECTS: u32 = 0x0083_f130;
    pub const SCREEN_PROCESS: u32 = 0x007b_c110;
    pub const SCREEN_DATA: u32 = 0x007b_bd00;
    pub const GET_PART_MATRIX: u32 = 0x0076_1c40;
    pub const COPY_CAMERA: u32 = 0x007c_0070;
    pub const COPY_SCREEN: u32 = 0x007b_ba50;
    pub const SET_PART_MATRIX: u32 = 0x0076_2550;
    pub const INVERSE: u32 = 0x006e_d460;
    pub const VIEW_PROJECTION: u32 = 0x0071_1910;
    pub const YI_SCALE: u32 = 0x0090_a950;
    pub const YI_VIEW: u32 = 0x0090_a980;
    pub const YI_BATTLE_VIEW: u32 = 0x0090_a140;
    pub const CH_CAMERA_MATRIX: u32 = 0x0083_5eb0;
    pub const CH_SCREEN_MATRIX: u32 = 0x0083_5e90;
    pub const DRAW_ENV: u32 = 0x0082_c7b0;
    pub const ACC_FRAME_MAP: u32 = 0x0082_bc50;
    pub const ACC_DRAW_BG: u32 = 0x0082_be80;
    pub const YI_DISP: u32 = 0x0090_8480;
    pub const YI_DISP2: u32 = 0x0090_84a0;
    pub const ACC_FRAME_CHR: u32 = 0x0082_be20;
    pub const SPLIT_SCREEN: u32 = 0x0064_1cc0;
    pub const CH_DRAW: u32 = 0x0082_ed50;
    pub const MFIFO_END: u32 = 0x0084_3260;
    pub const MFIFO_SPAD_SYNC: u32 = 0x0084_32d0;
    pub const MFIFO_SYNC: u32 = 0x0084_3360;
    pub const DREAM_EFFECT: u32 = 0x0082_23a0;
    pub const MENU_WATER: u32 = 0x0063_e920;
    pub const OP_LOOP_AFTER: u32 = 0x007f_f680;
    pub const BATTLE_DRAW: u32 = 0x0078_0d00;
    pub const TK_DRAW_NO_TRANS: u32 = 0x0088_d4f0;
    pub const YN_DRAW_SCENE: u32 = 0x0091_a240;
    pub const IS_PREREND: u32 = 0x0084_3210;
    pub const CH_DRAW_SHADOW: u32 = 0x0083_0180;
    pub const GS1_TRANSFER: u32 = 0x007e_b580;
    pub const SET_VFX_TYPE: u32 = 0x0064_5670;
    pub const FIELD_PARTICLE: u32 = 0x0090_83e0;
    pub const PART_LOOP: u32 = 0x0076_2240;
    pub const OT_DRAW: u32 = 0x007e_c470;
    pub const SAVE_MANAGER_UPDATE: u32 = 0x0076_ef80;
    pub const DRAW_FILTER: u32 = 0x0082_c0c0;
    pub const ACC_SET_CURRENT: u32 = 0x0082_ce90;
    pub const SCR_EFFECT: u32 = 0x0090_8550;
    pub const TK_088D500: u32 = 0x0088_d500;
    pub const DRAW_MESSAGE_WINDOW: u32 = 0x008a_bd30;
    pub const SET_PAD_CTRL: u32 = 0x008a_b580;
    pub const TK_DRAW_DEBUG: u32 = 0x0088_d4e0;
    pub const NUM_VDISP: u32 = 0x007e_6560;
    pub const MAIN_DEBUG: u32 = 0x0082_07f0;
    pub const OP_LOOP_END: u32 = 0x007f_f670;
    pub const IS_ETERNAL_CALM: u32 = 0x0067_ab50;
    pub const CALM_COPYRIGHT: u32 = 0x0090_4a70;
    pub const MENU_UPDATE: u32 = 0x0064_23b0;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const COUNTERS_SNAPSHOT: u32 = 0x0082_1a70;
    pub const TK_SYNC_V: u32 = 0x0088_7f00;
    pub const SWAP_DBUFF: u32 = 0x0062_f870;
    pub const SET_TK_ANTI_XYB: u32 = 0x0083_b110;
    pub const MOVIE_LOAD_IMAGE: u32 = 0x0076_f020;
    pub const ENV_MOVIE_MESSAGE: u32 = 0x008a_f590;
    pub const MAGIC_FILE_UPDATE: u32 = 0x009d_ac60;
    pub const CLUSTER_RELATIONSHIP: u32 = 0x0063_d9c0;
    pub const CLOTH_WAIT: u32 = 0x00a4_4b30;
    pub const GET_SCENARIO: u32 = 0x0078_7360;
    pub const CLOTH_STATS: u32 = 0x0063_ddc0;
    pub const SOFTWARE_RESET: u32 = 0x0087_38d0;
}

/// `Sg_MainLoop`'s frame below `ebp`: the field scale (three `f32` at
/// `-0x14`), the camera matrix (`-0x54`), the screen matrix (`-0x94`),
/// the fade colour (`-0xa4..-0x98`).
pub const MAIN_LOOP_FRAME: u32 = 0xa4;

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the fade colour's r, g, b, a
pub mod bridge {
    use super::{f, g};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn call(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }
    fn r32(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }
    fn w32(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }
    fn on(h: &impl Host, sym: u32) -> bool {
        h.read::<u8>(h.rebase(sym)) != 0
    }
    fn flush(h: &mut impl Host, k: u32) {
        call(h, f::COUNTERS_FLUSH, &[k]);
    }
    fn marker(h: &mut impl Host, name: u32) {
        if r32(h, g::REPEAT_C) == 0 {
            let name = h.rebase(name);
            call(h, f::PROFILER_MARKER, &[name]);
        }
    }
    /// The field draws (both switches on).
    fn field(h: &impl Host) -> bool {
        on(h, g::FIELD_ON) && on(h, g::FIELD_DRAW)
    }
    /// Whether the distortion pass stays on: the field draws, bit 24 of
    /// the display flags, and display mode 0 (asked last).
    fn distortion_kept(h: &mut impl Host) -> bool {
        field(h) && r32(h, g::DISP_FLAGS) & 0x0100_0000 != 0 && call(h, f::GET_DISP_MODE, &[]) == 0
    }
    /// The three "update repeated" flags off, the hold done, the scene
    /// request saved, and the deleted effects released.
    fn end_repeat(h: &mut impl Host) {
        let req = r32(h, g::SCENE_REQUEST);
        for at in [g::REPEAT_A, g::REPEAT_C, g::REPEAT_B, g::VOICE_HOLD] {
            w32(h, at, 0);
        }
        w32(h, g::SCENE_LAST, req);
        call(h, f::VFX_UPDATE_DELETED, &[]);
    }

    /// `Sg_MainLoop(dt)` (`0x00820ae0`, cdecl; `dt` an `f32`): one frame
    /// of the game. `dt` is scaled by the speed control (and stored back
    /// in its argument slot, `slot`); with the free camera in mode 2 only
    /// the distortion switch is kept. Otherwise: the controllers, the
    /// frame sync (or the pause's scan), the voice hold's repeats, the
    /// loaders and save screen; then, unless a scene change, the pause,
    /// the debug hold or the save screen stops it (`stop`), the fades,
    /// the world update (`Sg_ControlMain`, the menus, achievements, the
    /// per-frame battle/event/field/character steps), the draw (every
    /// screen of `MsScreenProcess`, the effects, the filter, the debug
    /// side), the Eternal Calm copyright, and the frame's end (the GS
    /// swap, the movie messages, the cloth wait, the scenario print).
    /// `ebp` is the original's frame ([`super::MAIN_LOOP_FRAME`] bytes
    /// below it).
    #[allow(clippy::too_many_lines)] // one frame, in the original's order
    pub fn main_loop(h: &mut impl Host, ebp: u32, dt: u32, slot: u32) {
        let scale_at = ebp - 0x14;
        let camera = ebp - 0x54;
        let screen = ebp - 0x94;
        let fade = ebp - 0xa4;
        let dt = h.call_sym_st0(f::SPD_SCALE, Conv::Cdecl, &[x87::store(x87::load(dt))]);
        let dt = dt.to_bits();
        h.write(slot, dt);
        // the original pops ST0 after this call too, on an empty stack
        call(h, f::USE_EXTENDED_ALPHA, &[0]);
        call(h, f::SCENE_CLEAR_LIST, &[]);
        call(h, f::PS2_LOADING_CALLBACK, &[]);
        let s = x87::store(x87::load(r32(h, g::FIELD_SCALE)));
        for k in 0..3 {
            h.write(scale_at + 4 * k, s);
        }
        call(h, f::MODEL_MEM_USAGE, &[]);
        let wide = x87::load(dt).to_bits();
        #[allow(clippy::cast_possible_truncation)] // the double's two words
        call(h, f::VIDEO_UPDATE, &[wide as u32, (wide >> 32) as u32]);
        if call(h, f::FREE_CAMERA_TYPE, &[]) == 2 {
            if !distortion_kept(h) {
                call(h, f::SET_DISTORTION_SKIPPED, &[1]);
            }
            return;
        }
        call(h, f::TK_UPDATE_SCAN, &[]);
        if r32(h, g::PAUSE_ON) == 0 {
            call(h, f::FRAME_SYNC, &[x87::store(x87::load(dt))]);
        } else {
            call(h, f::TK_SCAN, &[]);
        }
        let hold = r32(h, g::VOICE_HOLD);
        if hold.cast_signed() > 1 {
            for at in [g::REPEAT_A, g::REPEAT_C, g::REPEAT_B] {
                w32(h, at, 1);
            }
            w32(h, g::VOICE_HOLD, hold - 1);
        } else {
            end_repeat(h);
        }
        call(h, f::CHR_INSTANTIATE, &[]);
        call(h, f::CHR_DELAY_RELEASE, &[x87::store(x87::load(dt))]);
        call(h, f::STUB_639380, &[]);
        call(h, f::SAVE_UPDATE, &[]);
        if call(h, f::SAVE_TAKE_LOADED, &[]) == 1 || call(h, f::SAVE_TAKE_LOADED, &[]) == 1 {
            let dic = call(h, f::GET_U32_01325AC4, &[]);
            call(h, f::ALBHED_DIC, &[1, dic]);
        }
        call(h, f::SAVE_SCREEN, &[]);
        if call(h, f::RETURN_CONST1, &[]) != 0 {
            w32(h, g::DEBUG_HOLD, 0);
        }
        let hist = call(h, f::CONTROLER_HIST, &[0, 0, 0]) & 0xffff;
        if hist & 0x8000 != 0 && hist & 0x100 != 0 && hist & 0x20 != 0 {
            call(h, f::RESET_NAME, &[]);
        }
        let mut req = r32(h, g::SCENE_REQUEST);
        if req != 0 {
            call(h, f::CLEAR_SPECIAL, &[]);
            req = r32(h, g::SCENE_REQUEST);
        }
        let mut pause = 0;
        if req != 0 || r32(h, g::PAUSE_ON) != 0 {
            call(h, f::CHECK_MENU, &[]);
            pause = r32(h, g::PAUSE_ON);
            req = r32(h, g::SCENE_REQUEST);
        }
        if req != 0 || pause != 0 || r32(h, g::DEBUG_HOLD) != 0 {
            stop(h, pause);
            return;
        }
        if call(h, f::SAVE_SCREEN_CLOSED, &[]) == 0 || call(h, f::SCENE_LOADING, &[]) != 0 {
            let pause = r32(h, g::PAUSE_ON);
            stop(h, pause);
            return;
        }
        if r32(h, g::DYN_GEO_STATE) == 0 {
            call(h, f::DYN_GEO_SWAP, &[1]);
            w32(h, g::DYN_GEO_STATE, 1);
        }
        fades(h, fade);
        marker(h, g::MARK_A);
        call(h, f::CONTROL_MAIN, &[]);
        if r32(h, g::NAME_RESET) != 0 {
            call(h, f::RESET_NAME, &[]);
            w32(h, g::NAME_RESET, 0);
        }
        let iface = r32(h, g::INTERFACE);
        if call(h, f::VALIDATE_INTERFACE, &[iface]) != 0 {
            if call(h, f::CHECK_MENU, &[]) != 0 {
                frame_end(h);
                return;
            }
        } else {
            if call(h, f::CHECK_MENU, &[]) != 0 && r32(h, g::MENU_FADE) != 1 {
                call(h, f::LOCK_FADE, &[]);
                if !distortion_kept(h) {
                    call(h, f::SET_DISTORTION_SKIPPED, &[1]);
                }
                frame_end(h);
                return;
            }
            w32(h, g::MENU_FADE, 0);
            call(h, f::UNLOCK_FADE, &[]);
        }
        if r32(h, g::RESET_REQUEST) == 1 {
            end_repeat(h);
        }
        if r32(h, g::REPEAT_A) == 0 {
            call(h, f::MENU_RESET, &[]);
        }
        achievements(h);
        call(h, f::CALC_RATE, &[]);
        call(h, f::INIT_FONT, &[]);
        call(h, f::CLEAR_SPECIAL, &[]);
        let p = r32(h, g::FREE_NEXT);
        if p != 0 {
            if r32(h, g::FREE_SEEN) == p {
                call(h, f::USER_FREE, &[p]);
                w32(h, g::FREE_NEXT, 0);
                w32(h, g::FREE_SEEN, 0);
            } else {
                w32(h, g::FREE_SEEN, p);
            }
        }
        if r32(h, g::VFX_DESTROY) == 1 && r32(h, g::VFX_HOLD) == 0 {
            call(h, f::VFX_DESTROY, &[1, 0x2c2]);
            call(h, f::VFX_DESTROY, &[2, 0x2c2]);
            w32(h, g::VFX_DESTROY, 0);
        }
        w32(h, g::VFX_HOLD, 0);
        call(h, f::CLEAR_FLAGS24, &[]);
        marker(h, g::MARK_B);
        update(h);
        marker(h, g::MARK_C);
        let movie = h.read::<u8>(h.rebase(g::MOVIE_ON));
        let at = h.rebase(g::MOVIE_LAST);
        h.write(at, movie);
        call(h, f::MOVIE_CAMERA, &[]);
        flush(h, 7);
        call(h, f::VIDEO_PLAY, &[]);
        call(h, f::TEXTURE_VIDEO, &[]);
        let buffer = r32(h, g::DRAW_BUFFER);
        call(h, f::SET_DISP_DRAW_ENV, &[buffer]);
        let mut drawn = false;
        if on(h, g::MOVIE_ON) {
            call(h, f::ENV_INIT_MOVIE, &[]);
            drawn = on(h, g::MOVIE_ON);
        }
        if !drawn {
            draw(h, scale_at, camera, screen);
        }
        call(h, f::MFIFO_SYNC, &[]);
        flush(h, 8);
        call(h, f::SAVE_MANAGER_UPDATE, &[]);
        flush(h, 7);
        call(h, f::DRAW_FILTER, &[]);
        call(h, f::ACC_SET_CURRENT, &[]);
        if on(h, g::MOVIE_ON) {
            call(h, f::SET_DISTORTION_SKIPPED, &[1]);
        } else {
            if !on(h, g::FIELD_ON) {
                call(h, f::SET_DISTORTION_SKIPPED, &[1]);
            } else if on(h, g::FIELD_DRAW) {
                call(h, f::SCR_EFFECT, &[]);
            }
            call(h, f::TK_088D500, &[]);
            if on(h, g::MESSAGE_ON) {
                if r32(h, g::REPEAT_A) == 0 {
                    call(h, f::DRAW_MESSAGE_WINDOW, &[]);
                } else {
                    call(h, f::SET_PAD_CTRL, &[1]);
                }
            }
            call(h, f::TK_DRAW_DEBUG, &[]);
            flush(h, 3);
            if on(h, g::EFFECT_ON) && on(h, g::DEBUG_ON) {
                call(h, f::NUM_VDISP, &[]);
            }
            flush(h, 0xb);
            call(h, f::MAIN_DEBUG, &[]);
            flush(h, 1);
            call(h, f::OP_LOOP_END, &[]);
            flush(h, 0xb);
        }
        let map = r32(h, g::MAP);
        if call(h, f::IS_ETERNAL_CALM, &[map]) != 0 && r32(h, g::CALM_STATE) == 1 {
            let n = r32(h, g::CALM_FRAMES).wrapping_add(1);
            w32(h, g::CALM_FRAMES, n);
            call(h, f::CALM_COPYRIGHT, &[n]);
            if r32(h, g::CALM_FRAMES).cast_signed() > 0x130 {
                w32(h, g::CALM_STATE, 2);
            }
        }
        if r32(h, g::REPEAT_A) == 0 {
            call(h, f::MENU_UPDATE, &[]);
        }
        frame_end(h);
    }

    /// The fades at the end of a scene load: while the assets load with
    /// a fade (last reason 1 or 2) whose alpha is not 1.0, the colour is
    /// set again opaque and locked; once loaded with the colour locked
    /// and a reason, both are cleared. `fade` is the frame's colour
    /// (`ebp - 0xa4`).
    fn fades(h: &mut impl Host, fade: u32) {
        if call(h, f::ASSETS_LOADED, &[]) == 0
            && (call(h, f::FADE_LAST_REASON, &[]) == 1 || call(h, f::FADE_LAST_REASON, &[]) == 2)
        {
            let (r, g, b, a) = (fade + 0xc, fade + 8, fade + 4, fade);
            call(h, f::GET_FADE_COLOR, &[r, g, b, a]);
            let alpha = x87::load(h.read(a));
            // fld a; fld1; fucompp; test ah 0x44; jnp: skip when equal
            #[allow(clippy::float_cmp)] // the original's exact compare
            if alpha != 1.0 {
                if call(h, f::FADE_LOCKED, &[]) != 0 {
                    call(h, f::UNLOCK_FADE, &[]);
                }
                let rgb = [r, g, b].map(|at| x87::store(x87::load(h.read(at))));
                call(
                    h,
                    f::SET_FADE_COLOR,
                    &[rgb[0], rgb[1], rgb[2], 1.0f32.to_bits()],
                );
                call(h, f::SET_FADE_ALPHA_MODE, &[0]);
                call(h, f::LOCK_FADE, &[]);
            }
        }
        if call(h, f::ASSETS_LOADED, &[]) != 0
            && call(h, f::FADE_LOCKED, &[]) != 0
            && call(h, f::FADE_LAST_REASON, &[]).cast_signed() > 0
        {
            call(h, f::SET_FADE_LAST_REASON, &[0]);
            call(h, f::UNLOCK_FADE, &[]);
        }
    }

    /// The scenario achievements (flag 0xa4, 0x43d, 0x938 when it changes
    /// to them) and the airship's (0x1b, once, when its byte is 0x7f;
    /// reset from -1 to 0 while it is 1..=0x7e).
    fn achievements(h: &mut impl Host) {
        let flag = call(h, f::SCENARIO_FLAG, &[]);
        w32(h, g::SCENARIO, flag);
        if flag != r32(h, g::SCENARIO_LAST) {
            w32(h, g::SCENARIO_LAST, flag);
            let id = match flag {
                0xa4 => Some(3),
                0x43d => Some(5),
                0x938 => Some(6),
                _ => None,
            };
            if let Some(id) = id {
                call(h, f::ACHIEVEMENT, &[id]);
            }
        }
        let ship = h.read::<u8>(h.rebase(g::AIRSHIP));
        let done = r32(h, g::AIRSHIP_DONE);
        if ship.wrapping_sub(1) <= 0x7d && done == u32::MAX {
            w32(h, g::AIRSHIP_DONE, 0);
        } else if ship == 0x7f && done == 0 {
            w32(h, g::AIRSHIP_DONE, 1);
            call(h, f::ACHIEVEMENT, &[0x1b]);
        }
    }

    /// The world update: once per frame to execute (the player, battle,
    /// events, the message window, effects, camera, field and
    /// characters), or with none the effects (and camera) alone; then the
    /// characters' middle step and the field animation.
    fn update(h: &mut impl Host) {
        let mut i = 0i32;
        if call(h, f::EXEC_FRAMES, &[]).cast_signed() > 0 {
            loop {
                call(h, f::CONTROL_PLAYER, &[]);
                call(h, f::BATTLE_PROCESS, &[]);
                call(h, f::EVENT_JUMP, &[]);
                call(h, f::MESSAGE_WINDOW, &[]);
                flush(h, 5);
                if !on(h, g::MOVIE_ON) {
                    call(h, f::OP_LOOP_BEFORE, &[]);
                    call(h, f::EFFECT_PROCESS, &[0]);
                    flush(h, 0xb);
                }
                call(h, f::CAMERA_PROCESS, &[]);
                if !on(h, g::MOVIE_ON) {
                    call(h, f::FIELD_PROC, &[]);
                    flush(h, 5);
                }
                if on(h, g::CHARA_CALC) {
                    let step = x87::store(x87::load(r32(h, g::CHARA_STEP)));
                    call(h, f::CALC_MAIN, &[step]);
                }
                flush(h, 2);
                call(h, f::CAMERA_SET_PLAYER, &[]);
                i += 1;
                if i >= call(h, f::EXEC_FRAMES, &[]).cast_signed() {
                    break;
                }
            }
        }
        if call(h, f::EXEC_FRAMES, &[]) == 0 {
            call(h, f::EFFECT_PROCESS, &[0]);
            if on(h, g::CAMERA_IDLE) {
                call(h, f::CAMERA_SET_PLAYER, &[]);
                call(h, f::CAMERA_PROCESS, &[]);
            }
        }
        flush(h, 5);
        if on(h, g::CHARA_CALC) {
            call(h, f::CALC_MIDDLE, &[]);
            flush(h, 2);
        }
        if field(h) {
            call(h, f::YI_ANIM, &[]);
            flush(h, 4);
        }
    }

    /// The characters' draw (`Ch_DrawMain`) unless the split screen is on
    /// for a later screen, then counter 2.
    fn draw_characters(h: &mut impl Host, first: bool) {
        call(h, f::ACC_FRAME_CHR, &[]);
        if call(h, f::SPLIT_SCREEN, &[]) == 0 || first {
            call(h, f::CH_DRAW, &[]);
        }
        flush(h, 2);
    }

    /// One screen's draw: its matrices (from the effects' part matrix, or
    /// the battle camera's with their inverses), the field's and the
    /// characters' views, the draw environment of screen `n`, the map,
    /// the background and the field (`FIELD_FIRST` draws the field's
    /// second layer before the characters). Returns `first` cleared.
    fn draw_screen(
        h: &mut impl Host,
        n: u32,
        first: bool,
        scale_at: u32,
        camera: u32,
        screen: u32,
    ) {
        if on(h, g::PART_MATRIX) {
            call(h, f::GET_PART_MATRIX, &[camera, screen]);
        } else {
            call(h, f::COPY_CAMERA, &[camera]);
            call(h, f::COPY_SCREEN, &[screen]);
            call(h, f::SET_PART_MATRIX, &[camera, screen]);
            let inv = h.rebase(g::INV_CAMERA);
            call(h, f::INVERSE, &[inv, camera]);
            let inv = h.rebase(g::INV_SCREEN);
            call(h, f::INVERSE, &[inv, screen]);
            call(h, f::VIEW_PROJECTION, &[]);
        }
        call(h, f::YI_SCALE, &[scale_at]);
        call(h, f::YI_VIEW, &[screen]);
        call(h, f::YI_BATTLE_VIEW, &[camera]);
        call(h, f::CH_CAMERA_MATRIX, &[screen]);
        call(h, f::CH_SCREEN_MATRIX, &[camera]);
        call(h, f::DRAW_ENV, &[n]);
        flush(h, 3);
        call(h, f::ACC_FRAME_MAP, &[]);
        let mut field_on = on(h, g::FIELD_ON);
        if field_on && on(h, g::FIELD_DRAW) {
            call(h, f::ACC_DRAW_BG, &[]);
            call(h, f::YI_DISP, &[]);
            flush(h, 4);
            field_on = on(h, g::FIELD_ON);
        }
        let chara = |h: &mut _| on(h, g::CHARA_ON) && on(h, g::CHARA_DRAW);
        if on(h, g::FIELD_FIRST) {
            if field_on && on(h, g::FIELD_DRAW) {
                call(h, f::YI_DISP2, &[]);
                flush(h, 4);
            }
            if chara(h) {
                draw_characters(h, first);
            }
        } else {
            if chara(h) {
                draw_characters(h, first);
                field_on = on(h, g::FIELD_ON);
            }
            if field_on && on(h, g::FIELD_DRAW) {
                call(h, f::ACC_DRAW_BG, &[]);
                call(h, f::YI_DISP2, &[]);
                flush(h, 4);
            }
        }
    }

    /// The draw: every screen `MsScreenProcess` gives (two passes; the
    /// second draws the ones `MsGetScreenData` marks 0, the first the
    /// others, and the last screen number seen is kept), the MFIFO's end,
    /// the dream effect, the menu water, and per frame to execute the
    /// effects' draw.
    fn draw(h: &mut impl Host, scale_at: u32, camera: u32, screen: u32) {
        let mut first = true;
        call(h, f::ENV_INIT, &[]);
        call(h, f::SET_CHADIST, &[]);
        call(h, f::MFIFO_RESET, &[]);
        call(h, f::CAMERA_PUT_RECTS, &[]);
        for pass in 0..2u32 {
            let mut n = 0u32;
            let mut r = call(h, f::SCREEN_PROCESS, &[n]).cast_signed();
            while r <= 0 {
                if r == 0 {
                    let data = call(h, f::SCREEN_DATA, &[0, 0, 0, 0, 0]);
                    let draw = match data {
                        0 if pass == 0 => false,
                        0 => {
                            let at = h.rebase(g::SCREEN_LAST);
                            #[allow(clippy::cast_possible_truncation)] // `mov [..], bl`
                            h.write(at, n as u8);
                            true
                        }
                        1 => pass == 0,
                        _ => true,
                    };
                    if draw {
                        draw_screen(h, n, first, scale_at, camera, screen);
                        first = false;
                    }
                }
                n += 1;
                r = call(h, f::SCREEN_PROCESS, &[n]).cast_signed();
            }
        }
        let last = i32::from(h.read::<i8>(h.rebase(g::SCREEN_LAST))).cast_unsigned();
        call(h, f::SCREEN_PROCESS, &[last]);
        let last = i32::from(h.read::<i8>(h.rebase(g::SCREEN_LAST))).cast_unsigned();
        call(h, f::DRAW_ENV, &[last]);
        call(h, f::MFIFO_END, &[]);
        call(h, f::MFIFO_SPAD_SYNC, &[]);
        call(h, f::MFIFO_SYNC, &[]);
        flush(h, 8);
        marker(h, g::MARK_D);
        call(h, f::DREAM_EFFECT, &[]);
        if r32(h, g::MENU_WATER) != 0 {
            call(h, f::MENU_WATER, &[]);
        }
        if !(on(h, g::EFFECT_ON) && !on(h, g::MOVIE_LAST) && !on(h, g::MOVIE_HOLD)) {
            return;
        }
        let mut i = 0i32;
        loop {
            let n = match call(h, f::EXEC_FRAMES, &[]) {
                0 => 1,
                _ => call(h, f::EXEC_FRAMES, &[]),
            };
            if i >= n.cast_signed() {
                break;
            }
            call(h, f::OP_LOOP_AFTER, &[]);
            flush(h, 0xb);
            if i == 0 {
                call(h, f::BATTLE_DRAW, &[]);
                call(h, f::TK_DRAW_NO_TRANS, &[]);
                flush(h, 3);
                if field(h) {
                    call(h, f::YN_DRAW_SCENE, &[]);
                    if call(h, f::IS_PREREND, &[]) != 0 {
                        call(h, f::CH_DRAW_SHADOW, &[0]);
                    }
                    flush(h, 4);
                }
            }
            call(h, f::GS1_TRANSFER, &[]);
            if on(h, g::PARTICLE_ON) {
                call(h, f::SET_VFX_TYPE, &[1]);
                call(h, f::FIELD_PARTICLE, &[]);
                call(h, f::SET_VFX_TYPE, &[4]);
            }
            call(h, f::EFFECT_PROCESS, &[1]);
            call(h, f::PART_LOOP, &[]);
            flush(h, 0xb);
            call(h, f::MFIFO_SYNC, &[]);
            flush(h, 8);
            call(h, f::OT_DRAW, &[]);
            flush(h, 0xc);
            i += 1;
        }
    }

    /// The frame's end: the cache flush and path sync, counters, the
    /// vsync, the draw buffer swapped (`+1`), the anti-aliasing off, the
    /// movie messages, the magic files, the cluster check, the cloth
    /// wait, the scenario printed when it changed, the cloth stats.
    fn frame_end(h: &mut impl Host) {
        call(h, f::FLUSH_CACHE, &[0]);
        call(h, f::GS_SYNC_PATH, &[0, 0]);
        flush(h, 3);
        call(h, f::COUNTERS_SNAPSHOT, &[]);
        call(h, f::TK_SYNC_V, &[2]);
        let b = r32(h, g::DRAW_BUFFER).wrapping_add(1);
        w32(h, g::DRAW_BUFFER, b);
        let dbuff = h.rebase(g::DBUFF);
        call(h, f::SWAP_DBUFF, &[dbuff, b]);
        call(h, f::SET_TK_ANTI_XYB, &[0, 1, 0x80]);
        if call(h, f::MOVIE_LOAD_IMAGE, &[]) != 0 {
            call(h, f::ENV_MOVIE_MESSAGE, &[]);
        }
        call(h, f::MAGIC_FILE_UPDATE, &[]);
        call(h, f::CLUSTER_RELATIONSHIP, &[]);
        call(h, f::CLOTH_WAIT, &[]);
        marker(h, g::MARK_E);
        let printed = h.read::<u16>(h.rebase(g::SCENARIO_PRINTED));
        #[allow(clippy::cast_possible_truncation)] // `cmp ax, [..]`
        if call(h, f::GET_SCENARIO, &[]) as u16 != printed {
            #[allow(clippy::cast_possible_truncation)]
            let now = call(h, f::GET_SCENARIO, &[]) as u16;
            let at = h.rebase(g::SCENARIO_PRINTED);
            h.write(at, now);
            let iob: u32 = h.read(h.rebase(g::IOB_FUNC_IAT));
            let out = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x20);
            let fprintf: u32 = h.read(h.rebase(g::FPRINTF_IAT));
            let fmt = h.rebase(g::FMT_SCENARIO);
            h.call(fprintf, Conv::Cdecl, &[out, fmt, u32::from(now)]);
        }
        call(h, f::CLOTH_STATS, &[]);
    }

    /// A stopped frame (a scene change, the pause, the debug hold or the
    /// save screen): the dynamic geometry's swap undone in the pause, or
    /// its state cleared when the save screen is open; the fade locked,
    /// the distortion switch, and the software reset check.
    fn stop(h: &mut impl Host, pause: u32) {
        if r32(h, g::DYN_GEO_STATE) == 1 && pause != 0 {
            w32(h, g::DYN_GEO_STATE, 0);
            call(h, f::DYN_GEO_SWAP, &[0]);
        } else if call(h, f::SAVE_SCREEN_CLOSED, &[]) == 0 {
            w32(h, g::DYN_GEO_STATE, 0);
        }
        call(h, f::LOCK_FADE, &[]);
        if !distortion_kept(h) {
            call(h, f::SET_DISTORTION_SKIPPED, &[1]);
        }
        call(h, f::SOFTWARE_RESET, &[]);
    }

    /// `updateFFX`'s callees (cdecl).
    pub const ASYNC_CONTROLLER_ON: u32 = 0x0133_c930;
    pub const SET_INPUT_INFO: u32 = 0x0063_0de0;
    pub const DYN_GEO_CLEAR: u32 = 0x0063_fd80;
    pub const FILL_CLEAR_REQUEST: u32 = 0x0071_ecd0;
    pub const REMOVE_DELEGATIONS: u32 = 0x0063_dab0;
    pub const MAIN_LOOP: u32 = 0x0082_0ae0;
    pub const FILL_KICK: u32 = 0x0071_edb0;

    /// `updateFFX(dt)` (`0x00822840`, cdecl; `dt` an `f32`): the frame
    /// (`Sg_MainLoop`), after the input of the frame (with the async
    /// controller), the dynamic geometry cleared, the particle fill
    /// request cleared and the delegations removed; again while the voice
    /// hold repeats it. Then its argument slot (`slot`) is set to 0 and it
    /// tail-jumps to the particle fill's kick with that 0; its `eax`.
    pub fn update_ffx(h: &mut impl Host, dt: u32, slot: u32) -> u32 {
        loop {
            if r32(h, ASYNC_CONTROLLER_ON) != 0 {
                call(h, SET_INPUT_INFO, &[]);
            }
            call(h, DYN_GEO_CLEAR, &[]);
            call(h, FILL_CLEAR_REQUEST, &[]);
            call(h, REMOVE_DELEGATIONS, &[]);
            call(h, MAIN_LOOP, &[x87::store(x87::load(dt))]);
            if r32(h, g::VOICE_HOLD) == 0 {
                break;
            }
        }
        h.write(slot, 0u32);
        call(h, FILL_KICK, &[0])
    }

    /// `FFXApplication::animate`'s callees.
    pub const STEAM_READY: u32 = 0x00cc_9cf5;
    pub const RUN_CALLBACKS_IAT: u32 = 0x00b0_c8b8;
    pub const TROPHY_ERROR: u32 = 0x0064_8ff0;
    pub const IS_ETERNAL_CALM: u32 = 0x0067_ab50;
    pub const CALM_TEXTURE_DONE: u32 = 0x00c9_003c;
    pub const MENU_TEXTURES: u32 = 0x00cc_c820;
    pub const LOAD_SPECIAL_TEXTURE: u32 = 0x0067_d550;
    pub const UPDATE_MAP_YIQ: u32 = 0x0064_4990;
    pub const IS_ESC_MENU_ACTIVE: u32 = 0x0063_da80;
    pub const RESIZE_POST_TARGET: u32 = 0x0064_2ce0;
    pub const UPDATE_FFX: u32 = 0x0082_2840;
    pub const UPDATE_GLOBAL_TIME: u32 = 0x0042_fa00;
    pub const SCENE_ANIMATE: u32 = 0x0063_9300;
    pub const APPLICATION_ANIMATE: u32 = 0x0062_6060;

    /// `FFXApplication::animate(this)` (`0x0042f520`, thiscall, vtable
    /// slot 4): Steam's callbacks, then 0 when the trophies failed;
    /// the Eternal Calm menu texture once; with the Esc menu up
    /// (`+0x3ad`) the map's YIQ and the post target, and the flag again;
    /// without it `updateFFX(+0x24)`, the global time and the scene's
    /// animation; then the base class's `animate` (a tail jump), whose
    /// `eax` it returns.
    pub fn animate(h: &mut impl Host, this: u32) -> u32 {
        if on(h, STEAM_READY) {
            let f: u32 = h.read(h.rebase(RUN_CALLBACKS_IAT));
            h.call(f, Conv::Cdecl, &[]);
        }
        if call(h, TROPHY_ERROR, &[]) == 1 {
            return 0;
        }
        let map = r32(h, g::MAP);
        if call(h, IS_ETERNAL_CALM, &[map]) != 0 && r32(h, CALM_TEXTURE_DONE) == 0 {
            let textures = r32(h, MENU_TEXTURES);
            h.call_sym(
                LOAD_SPECIAL_TEXTURE,
                Conv::Thiscall(textures),
                &[0x10, 0, 0],
            );
            w32(h, CALM_TEXTURE_DONE, 1);
        }
        let mut menu = h.read::<u8>(this + 0x3ad) != 0;
        if menu {
            call(h, UPDATE_MAP_YIQ, &[]);
            if h.read::<u8>(this + 0x3ad) != 0 && call(h, IS_ESC_MENU_ACTIVE, &[]) == 0 {
                call(h, RESIZE_POST_TARGET, &[]);
            }
            menu = call(h, IS_ESC_MENU_ACTIVE, &[]) != 0;
            h.write(this + 0x3ad, u8::from(menu));
        }
        if !menu {
            let dt: u32 = h.read(this + 0x24);
            call(h, UPDATE_FFX, &[x87::store(x87::load(dt))]);
            h.call_sym(UPDATE_GLOBAL_TIME, Conv::Thiscall(this), &[]);
            call(h, SCENE_ANIMATE, &[]);
        }
        h.call_sym(APPLICATION_ANIMATE, Conv::Thiscall(this), &[])
    }

    /// `TkClearVsyncHandler` (cdecl).
    pub const CLEAR_VSYNC_HANDLER: u32 = 0x0088_8830;

    /// `0x00822750`: `jmp TkClearVsyncHandler` (`exitApplication`'s);
    /// its `eax`.
    pub fn clear_vsync_thunk(h: &mut impl Host) -> u32 {
        call(h, CLEAR_VSYNC_HANDLER, &[])
    }
}
