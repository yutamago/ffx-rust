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
use ffx_prender::wp5g::bridge as misc;
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
    /// `0x0087e6f0()`: the event debug memory restored; the copy's `eax`.
    EventDebugRestore_0087e6f0 @ 0x0087_e6f0 => extern "C" fn event_debug_restore() -> u32 {
        misc::event_debug_copy(&mut mem(), true)
    }
    /// `0x0087e720()`: the event debug memory saved; the copy's `eax`.
    EventDebugSave_0087e720 @ 0x0087_e720 => extern "C" fn event_debug_save() -> u32 {
        misc::event_debug_copy(&mut mem(), false)
    }
    /// `ChnSystemMgrpReset`'s tail: the system motion group disposed.
    ChnSystemMgrpReset_tail_00837690 @ 0x0083_7690 => extern "C" fn system_mgrp_dispose() -> u32 {
        misc::system_mgrp_dispose(&mut mem())
    }
    /// The semaphore queue's destructor (thiscall).
    SemaphoreQueue_dtor_006db6f0 @ 0x006d_b6f0 => extern "thiscall" fn semaphore_queue_dtor(this: u32) -> u32 {
        misc::semaphore_queue_dtor(&mut mem(), this)
    }
    /// The battle position system's `+0x30` lists allocated.
    MsBtlPosAllocLists30_007c91a0 @ 0x007c_91a0 => extern "C" fn btl_pos_alloc_30() -> u32 {
        misc::btl_pos_alloc_lists(&mut mem(), 0x30, 0x34, 0x4c)
    }
    /// The battle position system's `+0xc` lists allocated.
    MsBtlPosAllocListsC_007cc180 @ 0x007c_c180 => extern "C" fn btl_pos_alloc_c() -> u32 {
        misc::btl_pos_alloc_lists(&mut mem(), 0xc, 0x10, 0x28)
    }
    /// `rcbgSceneDraw` of the scene's first wave.
    SceneWaveDraw_009215e0 @ 0x0092_15e0 => extern "C" fn scene_wave_draw() -> u32 {
        misc::scene_wave_draw(&mut mem(), 0x0092_0740)
    }
    /// `rcbgSceneSetChadist_thunk` of the scene's first wave.
    SceneWaveChadist_00921610 @ 0x0092_1610 => extern "C" fn scene_wave_chadist() -> u32 {
        misc::scene_wave_draw(&mut mem(), 0x0092_0780)
    }
    /// `0x009215e0` when the scene has a wave.
    SceneWaveDrawIfAny_00921a10 @ 0x0092_1a10 => extern "C" fn scene_wave_draw_if_any() -> u32 {
        misc::scene_wave_draw_if_any(&mut mem(), 0x0092_15e0)
    }
    /// `0x00921610` when the scene has a wave.
    SceneWaveChadistIfAny_00921a30 @ 0x0092_1a30 => extern "C" fn scene_wave_chadist_if_any() -> u32 {
        misc::scene_wave_draw_if_any(&mut mem(), 0x0092_1610)
    }
    /// The sound standby's end.
    SqsStandbyDone_0081fcb0 @ 0x0081_fcb0 => extern "C" fn sqs_standby_done() -> u32 {
        misc::sqs_standby_done(&mut mem())
    }
    /// A request list's destructor (thiscall).
    RequestList_dtor_00690a30 @ 0x0069_0a30 => extern "thiscall" fn request_list_dtor(this: u32) -> u32 {
        misc::request_list_dtor(&mut mem(), this)
    }
    /// The PS2 debug list's release.
    AtelReleaseDebugList_00873d60 @ 0x0087_3d60 => extern "C" fn release_debug_list() -> u32 {
        misc::release_debug_list(&mut mem())
    }
    /// The HDD mode's refresh.
    HddModeRefresh_00885560 @ 0x0088_5560 => extern "C" fn hdd_mode_refresh() -> u32 {
        misc::hdd_mode_refresh(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2a40 @ 0x008a_2a40 => extern "C" fn btl_scene_stop_8a2a40() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2a50 @ 0x008a_2a50 => extern "C" fn btl_scene_stop_8a2a50() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2a60 @ 0x008a_2a60 => extern "C" fn btl_scene_stop_8a2a60() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2a70 @ 0x008a_2a70 => extern "C" fn btl_scene_stop_8a2a70() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2a80 @ 0x008a_2a80 => extern "C" fn btl_scene_stop_8a2a80() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2a90 @ 0x008a_2a90 => extern "C" fn btl_scene_stop_8a2a90() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2aa0 @ 0x008a_2aa0 => extern "C" fn btl_scene_stop_8a2aa0() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene stop: `TOSeqStopExecScene(2)`; 1.
    BtlSceneStop_008a2ab0 @ 0x008a_2ab0 => extern "C" fn btl_scene_stop_8a2ab0() -> u32 {
        misc::btl_scene_stop(&mut mem())
    }
    /// A battle menu scene start: `TOSeqStartScene(2, name)`; 1.
    BtlSceneStart_008a86b0 @ 0x008a_86b0 => extern "C" fn btl_scene_start_8a86b0() -> u32 {
        misc::btl_scene_start(&mut mem(), 0x00b5_f088)
    }
    /// A battle menu scene start: `TOSeqStartScene(2, name)`; 1.
    BtlSceneStart_008a86f0 @ 0x008a_86f0 => extern "C" fn btl_scene_start_8a86f0() -> u32 {
        misc::btl_scene_start(&mut mem(), 0x00b5_f088)
    }
    /// A battle menu scene start: `TOSeqStartScene(2, name)`; 1.
    BtlSceneStart_008a8710 @ 0x008a_8710 => extern "C" fn btl_scene_start_8a8710() -> u32 {
        misc::btl_scene_start(&mut mem(), 0x00b5_f088)
    }
    /// A battle menu scene start: `TOSeqStartScene(2, name)`; 1.
    BtlSceneStart_008a8730 @ 0x008a_8730 => extern "C" fn btl_scene_start_8a8730() -> u32 {
        misc::btl_scene_start(&mut mem(), 0x00b5_f088)
    }
    /// A battle menu scene start: `TOSeqStartScene(2, name)`; 1.
    BtlSceneStart_008a8780 @ 0x008a_8780 => extern "C" fn btl_scene_start_8a8780() -> u32 {
        misc::btl_scene_start(&mut mem(), 0x00b5_f094)
    }
    /// `TOBtlDrawTargetCursor()`; 0.
    BtlTargetCursor0_008a84c0 @ 0x008a_84c0 => extern "C" fn btl_target_cursor_0() -> u32 {
        misc::btl_target_cursor_0(&mut mem())
    }
    /// The sphere grid's draw job 0.
    AbmapJobLinks_00a53450 @ 0x00a5_3450 => extern "C" fn abmap_job_links() {
        misc::abmap_job_links(&mut mem());
    }
    /// The sphere grid's draw job 1.
    AbmapJobHalos_00a534b0 @ 0x00a5_34b0 => extern "C" fn abmap_job_halos() {
        misc::abmap_job_halos(&mut mem());
    }
    /// The sphere grid's draw job 2.
    AbmapJobSpheres_00a534c0 @ 0x00a5_34c0 => extern "C" fn abmap_job_spheres() {
        misc::abmap_job_spheres(&mut mem());
    }
    /// The sphere grid's draw job 3.
    AbmapJobShapes_00a53510 @ 0x00a5_3510 => extern "C" fn abmap_job_shapes() {
        misc::abmap_job_shapes(&mut mem());
    }
    /// The grid's "get node" panel state: the saved state put back.
    AbmapState_RestoreSaved_00a48280 @ 0x00a4_8280 => extern "C" fn abmap_restore_state() {
        misc::abmap_restore_state(&mut mem());
    }
    /// The grid panels' draw hook: the help message.
    AbmapDraw_HelpPart_00a51700 @ 0x00a5_1700 => extern "C" fn abmap_help_draw() -> u32 {
        misc::abmap_help_draw(&mut mem())
    }
    /// The grid warp's draw hook.
    AbmapDraw_Warp_00a4c430 @ 0x00a4_c430 => extern "C" fn abmap_warp_draw() {
        misc::abmap_warp_draw(&mut mem());
    }
    /// The grid's select-homing state.
    AbmapState_SelectHoming_00a5a020 @ 0x00a5_a020 => extern "C" fn abmap_select_homing() {
        misc::abmap_select_homing(&mut mem());
    }
    /// A PC UI pass step: the depth views cleared.
    PcUi_ClearDepth_00a373c0 @ 0x00a3_73c0 => extern "C" fn pcui_clear_depth() {
        misc::pcui_clear_depth(&mut mem());
    }
    /// A PC UI pass step (stdcall, one word): the node lists requeued.
    PcUi_RequeueNodes_00a3b090 @ 0x00a3_b090 => extern "stdcall" fn pcui_requeue_nodes(_p: u32) -> u32 {
        misc::pcui_requeue_nodes(&mut mem())
    }
    /// The read task's step.
    ReadTaskStep_0076e5f0 @ 0x0076_e5f0 => extern "C" fn read_task_step() -> u32 {
        misc::read_task_step(&mut mem())
    }
    /// The field scene's step.
    Yn_SceneStep_00922060 @ 0x0092_2060 => extern "C" fn yn_scene_step() {
        misc::yn_scene_step(&mut mem());
    }
}
