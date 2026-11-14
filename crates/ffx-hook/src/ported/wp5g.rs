//! M5 finish, WP5 part 6 (wp5g): functions in gaps of the dump that the
//! script natives jump to, the event loader queues, a battle CD read
//! calls back, the main menu's command window calls or the OT draw runs
//! (`ffx_script::wp5g`, `ffx_battle::read`, `ffx_menu::wp5g_menu`,
//! `ffx_vfx::wp5g_encount`; parity tests in
//! `crates/ffx-oracle/tests/wp5g.rs`). All run on the main thread (the
//! script VM, the CD completion, the menu, the battle draw); no shim keeps
//! state. The encounter draw steps' frame is a `StackFrame` (0x240 bytes).

use super::{StackFrame, call_cdecl, mem, ported};
use ffx_battle::read::bridge as read;
use ffx_menu::wp5g_menu::bridge as menu;
use ffx_script::wp5g::bridge as sc;
use ffx_vfx::wp5g_encount::bridge as enc4;

fn cdecl<M>(_: &mut M, f: u32, args: &[u32]) -> u32 {
    call_cdecl(f, args)
}

ported! {
    /// `MsBtlReadSceneDone()`: the battle scene read's completion;
    /// `MsBtlReadWeapon`'s `eax`.
    MsBtlReadSceneDone_00783dd0 @ 0x0078_3dd0 => extern "C" fn ms_btl_read_scene_done() -> u32 {
        read::ms_btl_read_scene_done(&mut mem(), &mut cdecl)
    }
    /// `MsGetSaveConfigSubTitle()`.
    MsGetSaveConfigSubTitle @ 0x0078_5210 => extern "C" fn ms_get_save_config_sub_title() -> u32 {
        read::ms_get_save_config_sub_title(&mem())
    }
    /// `0x007b2c00()`: `SndStopStream()`; 0.
    SndStopStreamRet0_007b2c00 @ 0x007b_2c00 => extern "C" fn stop_stream_0() -> u32 {
        sc::stop_stream_0(&mut mem(), &mut cdecl)
    }
    /// The event loader's "DONE" step; 1.
    AtelLoaderDone_00861480 @ 0x0086_1480 => extern "C" fn loader_done() -> u32 {
        sc::loader_done(&mut mem(), &mut cdecl)
    }
    /// The event loader's "DONE2" step; 1.
    AtelLoaderDone2_00861510 @ 0x0086_1510 => extern "C" fn loader_done2() -> u32 {
        sc::loader_done2(&mut mem(), &mut cdecl)
    }
    /// `0x0086bcc0()`: the save event's largest-word index.
    SaveEventMaxIndex_0086bcc0 @ 0x0086_bcc0 => extern "C" fn save_event_max_index() -> u32 {
        sc::save_event_max_index(&mut mem(), &mut cdecl)
    }
    /// `0x00874060()`: the texture video's start; 1 or 0.
    TextureVideoPlay_00874060 @ 0x0087_4060 => extern "C" fn texture_video_play() -> u32 {
        sc::texture_video_play(&mut mem(), &mut cdecl)
    }
    /// `0x00886430()`: the byte at `0x01330092`.
    SndByte_00886430 @ 0x0088_6430 => extern "C" fn snd_byte() -> u32 {
        sc::snd_byte(&mem())
    }
    /// `0x008cc140()`: bit 0 of the battle save event's `+0x5ec`.
    SaveEventGetBit5ec_008cc140 @ 0x008c_c140 => extern "C" fn save_event_bit_5ec() -> u32 {
        sc::save_event_bit_5ec(&mut mem(), &mut cdecl)
    }
    /// `0x0091a8d0()`: `rcbgSceneWaveDCStop` of the scene's first wave.
    SceneWaveDCStop_0091a8d0 @ 0x0091_a8d0 => extern "C" fn scene_wave_dc_stop() -> u32 {
        sc::scene_wave_stop(&mut mem(), &mut cdecl, sc::WAVE_DC_STOP)
    }
    /// `0x0091a900()`: `rcbgSceneWaveC0Stop` of the scene's first wave.
    SceneWaveC0Stop_0091a900 @ 0x0091_a900 => extern "C" fn scene_wave_c0_stop() -> u32 {
        sc::scene_wave_stop(&mut mem(), &mut cdecl, sc::WAVE_C0_STOP)
    }
    /// `MenuSummon_InitLayout_008ceef0()`: the summon screen's layout;
    /// the row count.
    MenuSummon_InitLayout_008ceef0 @ 0x008c_eef0 => extern "C" fn summon_init_layout() -> u32 {
        menu::summon_init_layout(&mut mem())
    }
    /// The main command window's control callback.
    TkMenuMainCmdWindow_ctrl_008e2040 @ 0x008e_2040 => extern "C" fn cmd_window_ctrl() -> u32 {
        menu::cmd_window_ctrl(&mut mem())
    }
    /// The broken-screen encounter's op-10 draw step `(enc, cmd)`; `enc`.
    Encount4_DrawLit_007f75e0 @ 0x007f_75e0 => extern "C" fn encount4_draw_lit(enc: u32, cmd: u32) -> u32 {
        let mut f = StackFrame::<0x90>::new();
        let frame = f.at().wrapping_add(enc4::FRAME);
        enc4::encount4_draw(&mut mem(), enc, cmd, frame, true, &mut cdecl)
    }
    /// The broken-screen encounter's op-11 draw step `(enc, cmd)`; `enc`.
    Encount4_DrawFog_007f7a40 @ 0x007f_7a40 => extern "C" fn encount4_draw_fog(enc: u32, cmd: u32) -> u32 {
        let mut f = StackFrame::<0x90>::new();
        let frame = f.at().wrapping_add(enc4::FRAME);
        enc4::encount4_draw(&mut mem(), enc, cmd, frame, false, &mut cdecl)
    }
    /// `TOBtlDrawCursors_008930f0()`: the battle cursors; 0.
    TOBtlDrawCursors_008930f0 @ 0x0089_30f0 => extern "C" fn btl_draw_cursors() -> u32 {
        menu::btl_draw_cursors(&mut mem())
    }
    /// The main top window's control callback; the new fade.
    TOMenuTopWindow_ctrl_008a91d0 @ 0x008a_91d0 => extern "C" fn top_window_ctrl() -> u32 {
        menu::top_window_ctrl(&mut mem())
    }
    /// A window's close callback: the password text disposed; 1.
    MenuPasswordClose_008c4490 @ 0x008c_4490 => extern "C" fn password_close() -> u32 {
        menu::password_close(&mut mem())
    }
    /// The summon window's `+0x1c` test; 0 or 1.
    MenuSummonRowTest_008ccad0 @ 0x008c_cad0 => extern "C" fn summon_row_test() -> u32 {
        menu::summon_row_test(&mut mem())
    }
    /// The summon status window's draw; the kick's `eax`.
    MenuSummonStatus_draw_008cd080 @ 0x008c_d080 => extern "C" fn summon_status_draw() -> u32 {
        menu::summon_status_draw(&mut mem())
    }
    /// A customize window's `+8` callback.
    MenuKaizouClear_008d56e0 @ 0x008d_56e0 => extern "C" fn kaizou_clear() {
        menu::kaizou_clear(&mut mem());
    }
    /// The weapon shop's sub window 4 draw; the kick's `eax`.
    MenuWShopSubWindow4_draw_008d7f50 @ 0x008d_7f50 => extern "C" fn wshop_sub_window4_draw() -> u32 {
        menu::wshop_sub_window4_draw(&mut mem())
    }
    /// The weapon shop's window 3 draw; the kick's `eax`.
    MenuWShopWindow3_draw_008da6b0 @ 0x008d_a6b0 => extern "C" fn wshop_window3_draw() -> u32 {
        menu::wshop_window3_draw(&mut mem())
    }
    /// A window's close step: the module files read; its `eax`.
    MenuModuleFilesClose_008e2910 @ 0x008e_2910 => extern "C" fn module_files_close() -> u32 {
        menu::module_files_close(&mut mem())
    }
}
