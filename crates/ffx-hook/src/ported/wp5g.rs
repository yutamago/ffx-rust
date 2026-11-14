//! M5 finish, WP5 part 6 (wp5g): functions in gaps of the dump that the
//! script natives jump to, the event loader queues, or a battle CD read
//! calls back (`ffx_script::wp5g`, `ffx_battle::read`; parity tests in
//! `crates/ffx-oracle/tests/wp5g.rs`). All run on the main thread (the
//! script VM, the CD completion); no shim keeps state.

use super::{call_cdecl, mem, ported};
use ffx_battle::read::bridge as read;
use ffx_script::wp5g::bridge as sc;

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
}
