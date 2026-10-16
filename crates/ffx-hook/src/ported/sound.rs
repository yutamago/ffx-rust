//! `ffx-sound`: the PS2 sound API (`Snd*`), the FMOD queue's `i*` entry
//! points and the `FmodManager` command helpers; on the FMOD thread, the
//! command handlers (`FmodCmd_*`), `FfxFmod` and the simpler `FmodMusic`,
//! `FmodVoice` and `FmodSfx` methods.

use ffx_bridge::RawMem;
use ffx_sound::ffxfmod::bridge::{self as x, callee as xc};
use ffx_sound::fmod::bridge as f;
use ffx_sound::snd::bridge as s;

use super::{mem, ported};

/// The game as the sound bridges' host (`ffx_graphic::host::Host`: memory
/// and calls into the original code by convention).
fn game() -> RawMem {
    mem()
}

ported! {
    /// `iFmodFinalize()`.
    iFmodFinalize @ 0x0067_9fb0 => extern "C" fn i_fmod_finalize() -> u32 {
        f::i_fmod_finalize(&mut game())
    }
    /// `iFmodInitialize()`.
    iFmodInitialize @ 0x0067_9fc0 => extern "C" fn i_fmod_initialize() -> u32 {
        f::i_fmod_initialize(&mut game())
    }
    /// `iMuiscSetBgmType(t)`.
    iMuiscSetBgmType @ 0x0067_9fe0 => extern "C" fn i_muisc_set_bgm_type(t: u32) -> u32 {
        f::i_music_set_bgm_type(&mut game(), t)
    }
    /// `iMusicClear(id)`.
    iMusicClear @ 0x0067_a010 => extern "C" fn i_music_clear(id: u32) -> u32 {
        f::i_music_clear(&mut game(), id)
    }
    /// `iMusicClearAll()`.
    iMusicClearAll @ 0x0067_a030 => extern "C" fn i_music_clear_all() -> u32 {
        f::i_music_clear_all(&mut game())
    }
    /// `iMusicCrossPlay(id, frames)`.
    iMusicCrossPlay @ 0x0067_a040 => extern "C" fn i_music_cross_play(id: u32, frames: u32) -> u32 {
        f::i_music_cross_play(&mut game(), id, frames)
    }
    /// `iMusicDataSend(id)`.
    iMusicDataSend @ 0x0067_a060 => extern "C" fn i_music_data_send(id: u32) -> u32 {
        f::i_music_data_send(&mut game(), id)
    }
    /// `iMusicMultiPlay(id)`.
    iMusicMultiPlay @ 0x0067_a090 => extern "C" fn i_music_multi_play(id: u32) -> u32 {
        f::i_music_multi_play(&mut game(), id)
    }
    /// `iMusicMuteSet(a, b)`.
    iMusicMuteSet @ 0x0067_a0b0 => extern "C" fn i_music_mute_set(a: u32, b: u32) -> u32 {
        f::i_music_mute_set(&mut game(), a, b)
    }
    /// `iMusicPhrasePlay(id, phrase)`.
    iMusicPhrasePlay @ 0x0067_a0d0 => extern "C" fn i_music_phrase_play(id: u32, phrase: u32) -> u32 {
        f::i_music_phrase_play(&mut game(), id, phrase)
    }
    /// `iMusicPitchDirect(a, b)`.
    iMusicPitchDirect @ 0x0067_a0f0 => extern "C" fn i_music_pitch_direct(a: u32, b: u32) -> u32 {
        f::i_music_pitch_direct(&mut game(), a, b)
    }
    /// `iMusicPitchTranslate(a, b, c)`.
    iMusicPitchTranslate @ 0x0067_a110 => extern "C" fn i_music_pitch_translate(a: u32, b: u32, c: u32) -> u32 {
        f::i_music_pitch_translate(&mut game(), a, b, c)
    }
    /// `iMusicPlayFast(id)`.
    iMusicPlayFast @ 0x0067_a130 => extern "C" fn i_music_play_fast(id: u32) -> u32 {
        f::i_music_play_fast(&mut game(), id)
    }
    /// `iMusicPlayState(id)`.
    iMusicPlayState @ 0x0067_a150 => extern "C" fn i_music_play_state(id: u32) -> u32 {
        f::i_music_play_state(&mut game(), id)
    }
    /// `iMusicRead(id)`.
    iMusicRead @ 0x0067_a170 => extern "C" fn i_music_read(id: u32) -> u32 {
        f::i_music_read(&mut game(), id)
    }
    /// `iMusicReplayPointClear(id)`.
    iMusicReplayPointClear @ 0x0067_a190 => extern "C" fn i_music_replay_point_clear(id: u32) -> u32 {
        f::i_music_replay_point_clear(&mut game(), id)
    }
    /// `iMusicReplayPointSet()`.
    iMusicReplayPointSet @ 0x0067_a1b0 => extern "C" fn i_music_replay_point_set() -> u32 {
        f::i_music_replay_point_set(&mut game())
    }
    /// `iMusicSetPaused(flag)`.
    iMusicSetPaused @ 0x0067_a1c0 => extern "C" fn i_music_set_paused(flag: u32) -> u32 {
        f::i_music_set_paused(&mut game(), flag)
    }
    /// `iMusicStart(id)`.
    iMusicStart @ 0x0067_a1e0 => extern "C" fn i_music_start(id: u32) -> u32 {
        f::i_music_start(&mut game(), id)
    }
    /// `iMusicStop(id)`.
    iMusicStop @ 0x0067_a210 => extern "C" fn i_music_stop(id: u32) -> u32 {
        f::i_music_stop(&mut game(), id)
    }
    /// `iMusicStopAll()`.
    iMusicStopAll @ 0x0067_a230 => extern "C" fn i_music_stop_all() -> u32 {
        f::i_music_stop_all(&mut game())
    }
    /// `iMusicSyncStandby()`.
    iMusicSyncStandby @ 0x0067_a240 => extern "C" fn i_music_sync_standby() -> u32 {
        f::i_music_sync_standby(&mut game())
    }
    /// `iMusicTempoDirect(tempo)`.
    iMusicTempoDirect @ 0x0067_a250 => extern "C" fn i_music_tempo_direct(tempo: u32) -> u32 {
        f::i_music_tempo_direct(&mut game(), tempo)
    }
    /// `iMusicTempoTranslate(a, b)`.
    iMusicTempoTranslate @ 0x0067_a270 => extern "C" fn i_music_tempo_translate(a: u32, b: u32) -> u32 {
        f::i_music_tempo_translate(&mut game(), a, b)
    }
    /// `iMusicVolumeDirect(id, volume)`.
    iMusicVolumeDirect @ 0x0067_a290 => extern "C" fn i_music_volume_direct(id: u32, volume: u32) -> u32 {
        f::i_music_volume_direct(&mut game(), id, volume)
    }
    /// `iMusicVolumeFadeOut(id, frames)`.
    iMusicVolumeFadeOut @ 0x0067_a2b0 => extern "C" fn i_music_volume_fade_out(id: u32, frames: u32) -> u32 {
        f::i_music_volume_fade_out(&mut game(), id, frames)
    }
    /// `iMusicVolumeTranslate(id, frames, volume)`.
    iMusicVolumeTranslate @ 0x0067_a2d0 => extern "C" fn i_music_volume_translate(id: u32, frames: u32, volume: u32) -> u32 {
        f::i_music_volume_translate(&mut game(), id, frames, volume)
    }
    /// `iSfxDelayStart(a, b, c, d)`.
    iSfxDelayStart @ 0x0067_a2f0 => extern "C" fn i_sfx_delay_start(a: u32, b: u32, c: u32, d: u32) -> u32 {
        f::i_sfx_delay_start(&mut game(), a, b, c, d)
    }
    /// `iSfxForceStart(a, b, c)`.
    iSfxForceStart @ 0x0067_a310 => extern "C" fn i_sfx_force_start(a: u32, b: u32, c: u32) -> u32 {
        f::i_sfx_force_start(&mut game(), a, b, c)
    }
    /// `iSfxIsPaused()`.
    iSfxIsPaused @ 0x0067_a330 => extern "C" fn i_sfx_is_paused() -> u32 {
        f::i_sfx_is_paused(&mut game())
    }
    /// `iSfxIsPlaying(id)`.
    iSfxIsPlaying @ 0x0067_a340 => extern "C" fn i_sfx_is_playing(id: u32) -> u32 {
        f::i_sfx_is_playing(&mut game(), id)
    }
    /// `iSfxLoadData(id, flag)`.
    iSfxLoadData @ 0x0067_a360 => extern "C" fn i_sfx_load_data(id: u32, flag: u32) -> u32 {
        f::i_sfx_load_data(&mut game(), id, flag)
    }
    /// `iSfxPause(id)`.
    iSfxPause @ 0x0067_a380 => extern "C" fn i_sfx_pause(id: u32) -> u32 {
        f::i_sfx_pause(&mut game(), id)
    }
    /// `iSfxPlayBattleStreaming(a, b, c)`.
    iSfxPlayBattleStreaming @ 0x0067_a3a0 => extern "C" fn i_sfx_play_battle_streaming(a: u32, b: u32, c: u32) -> u32 {
        f::i_sfx_play_battle_streaming(&mut game(), a, b, c)
    }
    /// `iSfxSetPan(id, pan)`.
    iSfxSetPan @ 0x0067_a3c0 => extern "C" fn i_sfx_set_pan(id: u32, pan: u32) -> u32 {
        f::i_sfx_set_pan(&mut game(), id, pan)
    }
    /// `iSfxSetPanTranslate(id, pan, frames)`.
    iSfxSetPanTranslate @ 0x0067_a3e0 => extern "C" fn i_sfx_set_pan_translate(id: u32, pan: u32, frames: u32) -> u32 {
        f::i_sfx_set_pan_translate(&mut game(), id, pan, frames)
    }
    /// `FmodManager_registCommandAsync1(mgr, cmd)`.
    FmodManager_registCommandAsync1 @ 0x006f_a280 => extern "thiscall" fn fmod_manager_regist_command_async1(mgr: u32, cmd: u32) -> u32 {
        f::push0(&mut game(), mgr, cmd)
    }
    /// `FmodManager_registCommandAsync(mgr, cmd, a)`.
    FmodManager_registCommandAsync @ 0x006f_a2a0 => extern "thiscall" fn fmod_manager_regist_command_async(mgr: u32, cmd: u32, a: u32) -> u32 {
        f::push1(&mut game(), mgr, cmd, a)
    }
    /// `FmodManager_registCommandAsync3(mgr, cmd, a, b)`.
    FmodManager_registCommandAsync3 @ 0x006f_a2c0 => extern "thiscall" fn fmod_manager_regist_command_async3(mgr: u32, cmd: u32, a: u32, b: u32) -> u32 {
        f::push2(&mut game(), mgr, cmd, a, b)
    }
    /// `FmodManager_registCommandAsync4(mgr, cmd, a, b, c)`.
    FmodManager_registCommandAsync4 @ 0x006f_a2e0 => extern "thiscall" fn fmod_manager_regist_command_async4(mgr: u32, cmd: u32, a: u32, b: u32, c: u32) -> u32 {
        f::push3(&mut game(), mgr, cmd, a, b, c)
    }
    /// `FmodManager_registCommandSync1(mgr, cmd)`.
    FmodManager_registCommandSync1 @ 0x006f_a480 => extern "thiscall" fn fmod_manager_regist_command_sync1(mgr: u32, cmd: u32) -> u32 {
        f::sync0(&mut game(), mgr, cmd)
    }
    /// `FmodManager_registCommandSync2(mgr, cmd, a)`.
    FmodManager_registCommandSync2 @ 0x006f_a4a0 => extern "thiscall" fn fmod_manager_regist_command_sync2(mgr: u32, cmd: u32, a: u32) -> u32 {
        f::sync1(&mut game(), mgr, cmd, a)
    }
    /// `FmodManager_registCommandSync3(mgr, cmd, a, b)`.
    FmodManager_registCommandSync3 @ 0x006f_a4c0 => extern "thiscall" fn fmod_manager_regist_command_sync3(mgr: u32, cmd: u32, a: u32, b: u32) -> u32 {
        f::sync2(&mut game(), mgr, cmd, a, b)
    }
    /// `FmodManager_registCommandSync4(mgr, cmd, a, b, c)`.
    FmodManager_registCommandSync4 @ 0x006f_a4e0 => extern "thiscall" fn fmod_manager_regist_command_sync4(mgr: u32, cmd: u32, a: u32, b: u32, c: u32) -> u32 {
        f::sync3(&mut game(), mgr, cmd, a, b, c)
    }
    /// `FmodManager_signalMusicStart(mgr, id)`.
    FmodManager_signalMusicStart @ 0x006f_a690 => extern "thiscall" fn fmod_manager_signal_music_start(mgr: u32, id: u32) -> u32 {
        f::fmod_manager_signal_music_start(&mut mem(), mgr, id)
    }
    /// `FfxFmod_musicSeqCheck(fmod, id)`.
    FfxFmod_musicSeqCheck @ 0x0070_6b50 => extern "thiscall" fn ffx_fmod_music_seq_check(_fmod: u32, id: u32) -> u32 {
        f::ffx_fmod_music_seq_check(id)
    }
    /// `FfxFmod_setMusicStartId(fmod, id)`.
    FfxFmod_setMusicStartId @ 0x0070_6b70 => extern "thiscall" fn ffx_fmod_set_music_start_id(fmod: u32, id: u32) -> u32 {
        f::ffx_fmod_set_music_start_id(&mut mem(), fmod, id)
    }
    /// `FfxFmod_setMusicStartSignal(fmod, on)`.
    FfxFmod_setMusicStartSignal @ 0x0070_6b80 => extern "thiscall" fn ffx_fmod_set_music_start_signal(fmod: u32, on: u32) -> u32 {
        f::ffx_fmod_set_music_start_signal(&mut mem(), fmod, on)
    }
    /// `SndCalcVoiceWait()`.
    SndCalcVoiceWait @ 0x0088_5d20 => extern "C" fn snd_calc_voice_wait() -> u32 {
        s::snd_calc_voice_wait(&mem())
    }
    /// `SndInitNowPlayMusic()`.
    SndInitNowPlayMusic @ 0x0088_5db0 => extern "C" fn snd_init_now_play_music() {
        s::snd_init_now_play_music(&mut mem());
    }
    /// `SndClearMusicClearQue()`.
    SndClearMusicClearQue @ 0x0088_5dc0 => extern "C" fn snd_clear_music_clear_que() {
        s::snd_clear_music_clear_que(&mut mem());
    }
    /// `SndClearFootSepWork(unused)`.
    SndClearFootSepWork @ 0x0088_5de0 => extern "C" fn snd_clear_foot_sep_work(_unused: u32) {
        s::snd_clear_foot_sep_work(&mut mem());
    }
    /// `SndClearNowPlayMusic(id)`.
    SndClearNowPlayMusic @ 0x0088_5e80 => extern "C" fn snd_clear_now_play_music(id: u32) {
        s::snd_clear_now_play_music(&mut mem(), id);
    }
    /// `SndCancelMusicClear(id)`.
    SndCancelMusicClear @ 0x0088_5ea0 => extern "C" fn snd_cancel_music_clear(id: u32) {
        s::snd_cancel_music_clear(&mut game(), id);
    }
    /// `SndDataSendSync(x)`.
    SndDataSendSync @ 0x0088_5ee0 => extern "C" fn snd_data_send_sync(x: u32) -> u32 {
        s::snd_data_send_sync(&mut game(), x)
    }
    /// `SndEventClearPcToActorFoot()`.
    SndEventClearPcToActorFoot @ 0x0088_5f10 => extern "C" fn snd_event_clear_pc_to_actor_foot() {
        s::snd_event_clear_pc_to_actor_foot(&mut mem());
    }
    /// `SndEventSetPcToActorFoot(party, actor)`.
    SndEventSetPcToActorFoot @ 0x0088_5fd0 => extern "C" fn snd_event_set_pc_to_actor_foot(party: i32, actor: u32) {
        s::snd_event_set_pc_to_actor_foot(&mut mem(), party, actor);
    }
    /// `SndExecMusicClearQue(mode, id)`.
    SndExecMusicClearQue @ 0x0088_6000 => extern "C" fn snd_exec_music_clear_que(mode: u32, id: u32) {
        s::snd_exec_music_clear_que(&mut game(), mode, id);
    }
    /// `SndForcePlayClick(id)`.
    SndForcePlayClick @ 0x0088_60f0 => extern "C" fn snd_force_play_click(id: u32) -> u32 {
        s::snd_force_play_click(&mut game(), id)
    }
    /// `SndGetEventMask()`.
    SndGetEventMask @ 0x0088_6120 => extern "C" fn snd_get_event_mask() -> u32 {
        s::snd_get_event_mask(&mem())
    }
    /// `SndGetFoot3dMask()`.
    SndGetFoot3dMask @ 0x0088_6130 => extern "C" fn snd_get_foot3d_mask() -> u32 {
        s::snd_get_foot3d_mask(&mem())
    }
    /// `SndGetFootMaxVariation(kind)`.
    SndGetFootMaxVariation @ 0x0088_6140 => extern "C" fn snd_get_foot_max_variation(kind: u32) -> u32 {
        s::snd_get_foot_max_variation(kind)
    }
    /// `SndGetFootLastVariation(kind)`.
    SndGetFootLastVariation @ 0x0088_6190 => extern "C" fn snd_get_foot_last_variation(kind: u32) -> u32 {
        s::snd_get_foot_last_variation(kind)
    }
    /// `SndGetFootMask()`.
    SndGetFootMask @ 0x0088_61d0 => extern "C" fn snd_get_foot_mask() -> u32 {
        s::snd_get_foot_mask(&mem())
    }
    /// `SndGetFootSepWork(kind, party, variation)`.
    SndGetFootSepWork @ 0x0088_61e0 => extern "C" fn snd_get_foot_sep_work(kind: u32, party: i32, variation: i32) -> u32 {
        s::snd_get_foot_sep_work(&mem(), kind, party, variation)
    }
    /// `SndGetMusicPlayState(id)`.
    SndGetMusicPlayState @ 0x0088_63c0 => extern "C" fn snd_get_music_play_state(id: u32) -> u32 {
        s::snd_get_music_play_state(&mut game(), id)
    }
    /// `SndGetMusicVolume(id)`.
    SndGetMusicVolume @ 0x0088_63d0 => extern "C" fn snd_get_music_volume(id: u32) -> u32 {
        s::snd_get_music_volume(&mem(), id)
    }
    /// `SndGetSePlayState()`.
    SndGetSePlayState @ 0x0088_6400 => extern "C" fn snd_get_se_play_state() -> u32 {
        s::snd_get_se_play_state(&mut game())
    }
    /// `SndGetSepSize(sep)`.
    SndGetSepSize @ 0x0088_6420 => extern "C" fn snd_get_sep_size(sep: u32) -> u32 {
        s::snd_get_sep_size(&mem(), sep)
    }
    /// `SndGetSoundType()`.
    SndGetSoundType @ 0x0088_6440 => extern "C" fn snd_get_sound_type() -> u32 {
        s::snd_get_sound_type(&mem())
    }
    /// `SndGetSpuMemorySortSwitch()`.
    SndGetSpuMemorySortSwitch @ 0x0088_6450 => extern "C" fn snd_get_spu_memory_sort_switch() -> u32 {
        s::snd_get_spu_memory_sort_switch(&mem())
    }
    /// `SndGetWaveSize(wave)`.
    SndGetWaveSize @ 0x0088_64e0 => extern "C" fn snd_get_wave_size(wave: u32) -> u32 {
        s::snd_get_wave_size(&mem(), wave)
    }
    /// `SndInitFootSepWork()`.
    SndInitFootSepWork @ 0x0088_6500 => extern "C" fn snd_init_foot_sep_work() {
        s::snd_init_foot_sep_work(&mut mem());
    }
    /// `SndInitWork()`.
    SndInitWork @ 0x0088_6550 => extern "C" fn snd_init_work() {
        s::snd_init_work(&mut game());
    }
    /// `SndIopEioErrorStatus()`.
    SndIopEioErrorStatus @ 0x0088_65e0 => extern "C" fn snd_iop_eio_error_status() -> u32 {
        s::snd_iop_eio_error_status(&mut game())
    }
    /// `SndIsSepPlay(id)`.
    SndIsSepPlay @ 0x0088_6680 => extern "C" fn snd_is_sep_play(id: u32) -> u32 {
        s::snd_is_sep_play(&mut game(), id)
    }
    /// `SndKickFoot(kind, party, kick, variation)`.
    SndKickFoot @ 0x0088_66b0 => extern "C" fn snd_kick_foot(kind: u32, party: u32, kick: u32, variation: i32) {
        s::snd_kick_foot(&mut game(), kind, party, kick, variation);
    }
    /// `SndKickFootEvent(i)`.
    SndKickFootEvent @ 0x0088_67e0 => extern "C" fn snd_kick_foot_event(i: u32) -> u32 {
        s::snd_kick_foot_event(&mem(), i)
    }
    /// `SndMusicClear(id)`.
    SndMusicClear @ 0x0088_6800 => extern "C" fn snd_music_clear(id: u32) {
        s::snd_music_clear(&mut game(), id);
    }
    /// `SndMusicCrossStart(id, frames)`.
    SndMusicCrossStart @ 0x0088_6830 => extern "C" fn snd_music_cross_start(id: u32, frames: u32) {
        s::snd_music_cross_start(&mut game(), id, frames);
    }
    /// `SndMusicMultiStart(id)`.
    SndMusicMultiStart @ 0x0088_68b0 => extern "C" fn snd_music_multi_start(id: u32) {
        s::snd_music_multi_start(&mut game(), id);
    }
    /// `SndMusicPhraseStart(id, phrase)`.
    SndMusicPhraseStart @ 0x0088_68f0 => extern "C" fn snd_music_phrase_start(id: u32, phrase: u32) {
        s::snd_music_phrase_start(&mut game(), id, phrase);
    }
    /// `SndMusicPitchDirect(pitch, b)`.
    SndMusicPitchDirect @ 0x0088_6930 => extern "C" fn snd_music_pitch_direct(pitch: u32, b: u32) -> u32 {
        s::snd_music_pitch_direct(&mut game(), pitch, b)
    }
    /// `SndMusicPitchTranslate(id, pitch, frames)`.
    SndMusicPitchTranslate @ 0x0088_6940 => extern "C" fn snd_music_pitch_translate(id: u32, pitch: u32, frames: u32) -> u32 {
        s::snd_music_pitch_translate(&mut game(), id, pitch, frames)
    }
    /// `SndMusicRead(id)`.
    SndMusicRead @ 0x0088_6960 => extern "C" fn snd_music_read(id: u32) {
        s::snd_music_read(&mut game(), id);
    }
    /// `SndMusicStandby(id)`.
    SndMusicStandby @ 0x0088_6a00 => extern "C" fn snd_music_standby(id: u32) -> u32 {
        s::snd_music_standby(&mut game(), id)
    }
    /// `SndMusicStart(id)`.
    SndMusicStart @ 0x0088_6a20 => extern "C" fn snd_music_start(id: u32) {
        s::snd_music_start(&mut game(), id);
    }
    /// `SndMusicStop(id)`.
    SndMusicStop @ 0x0088_6a60 => extern "C" fn snd_music_stop(id: u32) {
        s::snd_music_stop(&mut game(), id);
    }
    /// `SndMusicTempoDirect(tempo)`.
    SndMusicTempoDirect @ 0x0088_6a90 => extern "C" fn snd_music_tempo_direct(tempo: u32) -> u32 {
        s::snd_music_tempo_direct(&mut game(), tempo)
    }
    /// `SndMusicTempoTranslate(a, tempo)`.
    SndMusicTempoTranslate @ 0x0088_6aa0 => extern "C" fn snd_music_tempo_translate(a: u32, tempo: u32) -> u32 {
        s::snd_music_tempo_translate(&mut game(), a, tempo)
    }
    /// `SndMusicVolumeDirect(id, volume)`.
    SndMusicVolumeDirect @ 0x0088_6ab0 => extern "C" fn snd_music_volume_direct(id: u32, volume: u32) {
        s::snd_music_volume_direct(&mut game(), id, volume);
    }
    /// `SndMusicVolumeFadeOut(id, frames)`.
    SndMusicVolumeFadeOut @ 0x0088_6af0 => extern "C" fn snd_music_volume_fade_out(id: u32, frames: u32) {
        s::snd_music_volume_fade_out(&mut game(), id, frames);
    }
    /// `SndMusicVolumeTranslate(id, frames, volume)`.
    SndMusicVolumeTranslate @ 0x0088_6b40 => extern "C" fn snd_music_volume_translate(id: u32, frames: u32, volume: u32) {
        s::snd_music_volume_translate(&mut game(), id, frames, volume);
    }
    /// `SndPlay(kind, id, pan, volume)`.
    SndPlay @ 0x0088_6b80 => extern "C" fn snd_play(kind: u32, id: u32, pan: u32, volume: u32) -> u32 {
        s::snd_play(&mut game(), kind, id, pan, volume)
    }
    /// `SndPlayClick(id)`.
    SndPlayClick @ 0x0088_6ba0 => extern "C" fn snd_play_click(id: u32) -> u32 {
        s::snd_play_click(&mut game(), id)
    }
    /// `SndRegistNowPlayMusic(id)`.
    SndRegistNowPlayMusic @ 0x0088_6c70 => extern "C" fn snd_regist_now_play_music(id: u32) {
        s::snd_regist_now_play_music(&mut mem(), id);
    }
    /// `SndRemoveFootSepWork()`.
    SndRemoveFootSepWork @ 0x0088_6cb0 => extern "C" fn snd_remove_foot_sep_work() {
        s::snd_remove_foot_sep_work(&mut mem());
    }
    /// `SndReportStandbyStatusDirect(x)`.
    SndReportStandbyStatusDirect @ 0x0088_6d00 => extern "C" fn snd_report_standby_status_direct(x: u32) -> u32 {
        s::snd_report_standby_status_direct(&mut game(), x)
    }
    /// `SndSePitchDirect(id, pitch, x)`.
    SndSePitchDirect @ 0x0088_6d20 => extern "C" fn snd_se_pitch_direct(id: u32, pitch: u32, x: u32) -> u32 {
        s::snd_se_pitch_direct(&mut game(), id, pitch, x)
    }
    /// `SndSePitchTranslate(id, a, pitch, b)`.
    SndSePitchTranslate @ 0x0088_6d50 => extern "C" fn snd_se_pitch_translate(id: u32, a: u32, pitch: u32, b: u32) -> u32 {
        s::snd_se_pitch_translate(&mut game(), id, a, pitch, b)
    }
    /// `SndSepClear(id)`.
    SndSepClear @ 0x0088_6d80 => extern "C" fn snd_sep_clear(id: u32) -> u32 {
        s::snd_sep_clear(&mut game(), id)
    }
    /// `SndSepClearAll()`.
    SndSepClearAll @ 0x0088_6db0 => extern "C" fn snd_sep_clear_all() -> u32 {
        s::snd_sep_clear_all(&mut game())
    }
    /// `SndSepLock(on)`.
    SndSepLock @ 0x0088_6dd0 => extern "C" fn snd_sep_lock(on: u32) -> u32 {
        s::snd_sep_lock(&mut mem(), on)
    }
    /// `SndSepPanDirect(id, pan)`.
    SndSepPanDirect @ 0x0088_6df0 => extern "C" fn snd_sep_pan_direct(id: u32, pan: u32) -> u32 {
        s::snd_sep_pan_direct(&mut game(), id, pan)
    }
    /// `SndSepPanTranslate(id, pan, frames)`.
    SndSepPanTranslate @ 0x0088_6e20 => extern "C" fn snd_sep_pan_translate(id: u32, pan: u32, frames: u32) -> u32 {
        s::snd_sep_pan_translate(&mut game(), id, pan, frames)
    }
    /// `SndSepPlay(id, pan, volume)`.
    SndSepPlay @ 0x0088_6e50 => extern "C" fn snd_sep_play(id: u32, pan: u32, volume: u32) -> u32 {
        s::snd_sep_play(&mut game(), id, pan, volume)
    }
    /// `SndSepPlaySimple(id)`.
    SndSepPlaySimple @ 0x0088_6e80 => extern "C" fn snd_sep_play_simple(id: u32) -> u32 {
        s::snd_sep_play_simple(&mut game(), id)
    }
    /// `SndSepStop(id)`.
    SndSepStop @ 0x0088_6eb0 => extern "C" fn snd_sep_stop(id: u32) -> u32 {
        s::snd_sep_stop(&mut game(), id)
    }
    /// `SndSepTrans(a, b)`.
    SndSepTrans @ 0x0088_6ee0 => extern "C" fn snd_sep_trans(a: u32, b: u32) -> u32 {
        s::snd_sep_trans(&mut game(), a, b)
    }
    /// `SndSepTransNonblock(a, b)`.
    SndSepTransNonblock @ 0x0088_6f00 => extern "C" fn snd_sep_trans_nonblock(a: u32, b: u32) -> u32 {
        s::snd_sep_trans_nonblock(&mut game(), a, b)
    }
    /// `SndSepTransSepOnly(a, b)`.
    SndSepTransSepOnly @ 0x0088_6f30 => extern "C" fn snd_sep_trans_sep_only(a: u32, b: u32) -> u32 {
        s::snd_sep_trans_sep_only(&mut game(), a, b)
    }
    /// `SndSepVolumeDirect(id, volume)`.
    SndSepVolumeDirect @ 0x0088_6f60 => extern "C" fn snd_sep_volume_direct(id: u32, volume: u32) -> u32 {
        s::snd_sep_volume_direct(&mut game(), id, volume)
    }
    /// `SndSepVolumeFadeOut(id, frames)`.
    SndSepVolumeFadeOut @ 0x0088_6f90 => extern "C" fn snd_sep_volume_fade_out(id: u32, frames: u32) -> u32 {
        s::snd_sep_volume_fade_out(&mut game(), id, frames)
    }
    /// `SndSepVolumeFadeOut2(id, volume)`.
    SndSepVolumeFadeOut2 @ 0x0088_6fc0 => extern "C" fn snd_sep_volume_fade_out2(id: u32, volume: u32) -> u32 {
        s::snd_sep_volume_fade_out2(&mut game(), id, volume)
    }
    /// `SndSepVolumeTranslate(id, volume, frames)`.
    SndSepVolumeTranslate @ 0x0088_6ff0 => extern "C" fn snd_sep_volume_translate(id: u32, volume: u32, frames: u32) -> u32 {
        s::snd_sep_volume_translate(&mut game(), id, volume, frames)
    }
    /// `SndSeqIopCommandSendMain()`.
    SndSeqIopCommandSendMain @ 0x0088_7020 => extern "C" fn snd_seq_iop_command_send_main() {
        s::snd_seq_iop_command_send_main(&mut game());
    }
    /// `SndSetEventStatus(party, on)`.
    SndSetEventStatus @ 0x0088_7080 => extern "C" fn snd_set_event_status(party: u32, on: u32) -> u32 {
        s::snd_set_event_status(&mut mem(), party, on)
    }
    /// `SndSetFoot3dStatus(party, on)`.
    SndSetFoot3dStatus @ 0x0088_70b0 => extern "C" fn snd_set_foot3d_status(party: u32, on: u32) -> u32 {
        s::snd_set_foot3d_status(&mut mem(), party, on)
    }
    /// `SndSetFootSep(kind, party, variation, sep)`.
    SndSetFootSep @ 0x0088_70e0 => extern "C" fn snd_set_foot_sep(kind: u32, party: i32, variation: i32, sep: u32) -> u32 {
        s::snd_set_foot_sep(&mut mem(), kind, party, variation, sep)
    }
    /// `SndSetFootStatus(party, on)`.
    SndSetFootStatus @ 0x0088_7100 => extern "C" fn snd_set_foot_status(party: u32, on: u32) -> u32 {
        s::snd_set_foot_status(&mut mem(), party, on)
    }
    /// `SndSetKinuStatus(party, on)`.
    SndSetKinuStatus @ 0x0088_7130 => extern "C" fn snd_set_kinu_status(party: u32, on: u32) -> u32 {
        s::snd_set_kinu_status(&mut mem(), party, on)
    }
    /// `SndSetMusicMute()`.
    SndSetMusicMute @ 0x0088_7160 => extern "C" fn snd_set_music_mute() {
        s::snd_set_music_mute(&mut mem());
    }
    /// `SndSetSoundMode(mode)`.
    SndSetSoundMode @ 0x0088_7180 => extern "C" fn snd_set_sound_mode(mode: i32) -> u32 {
        s::snd_set_sound_mode(&mut game(), mode)
    }
    /// `SndSetSoundType(t)`.
    SndSetSoundType @ 0x0088_71c0 => extern "C" fn snd_set_sound_type(t: i32) -> u32 {
        s::snd_set_sound_type(&mut mem(), t)
    }
    /// `SndSetStraightStreamingDigitalMode(x)`.
    SndSetStraightStreamingDigitalMode @ 0x0088_7210 => extern "C" fn snd_set_straight_streaming_digital_mode(x: u32) -> u32 {
        s::snd_set_straight_streaming_digital_mode(&mut game(), x)
    }
    /// `SndSetWaveData(list)`.
    SndSetWaveData @ 0x0088_7240 => extern "C" fn snd_set_wave_data(list: u32) {
        s::snd_set_wave_data(&mut game(), list);
    }
    /// `SndSortCommonWave(list)`.
    SndSortCommonWave @ 0x0088_7280 => extern "C" fn snd_sort_common_wave(list: u32) -> u32 {
        s::snd_sort_common_wave(&mut game(), list)
    }
    /// `SndSortCommonWaveWithSet(list)`.
    SndSortCommonWaveWithSet @ 0x0088_72a0 => extern "C" fn snd_sort_common_wave_with_set(list: u32) {
        s::snd_sort_common_wave_with_set(&mut game(), list);
    }
    /// `SndSpuMemorySortSwitch(x)`.
    SndSpuMemorySortSwitch @ 0x0088_72c0 => extern "C" fn snd_spu_memory_sort_switch(x: u32) -> u32 {
        s::snd_spu_memory_sort_switch(&mut game(), x)
    }
    /// `SndSqsIopFrame()`.
    SndSqsIopFrame @ 0x0088_7310 => extern "C" fn snd_sqs_iop_frame() -> u32 {
        s::snd_sqs_iop_frame(&mut game())
    }
    /// `SndSqsIopSoundErrorStatus()`.
    SndSqsIopSoundErrorStatus @ 0x0088_7360 => extern "C" fn snd_sqs_iop_sound_error_status() -> u32 {
        s::snd_sqs_iop_sound_error_status(&mut game())
    }
    /// `SndSqsReportIopSoundStatus(i)`.
    SndSqsReportIopSoundStatus @ 0x0088_73c0 => extern "C" fn snd_sqs_report_iop_sound_status(i: u32) -> u32 {
        s::snd_sqs_report_iop_sound_status(&mem(), i)
    }
    /// `SndSqsShoutStreamingPlayState()`.
    SndSqsShoutStreamingPlayState @ 0x0088_73f0 => extern "C" fn snd_sqs_shout_streaming_play_state() -> u32 {
        s::snd_sqs_shout_streaming_play_state(&mut game())
    }
    /// `SndSqsVagStreamingPlayState()`.
    SndSqsVagStreamingPlayState @ 0x0088_7420 => extern "C" fn snd_sqs_vag_streaming_play_state() -> u32 {
        s::snd_sqs_vag_streaming_play_state(&mut game())
    }
    /// `SndStandbyBattleWave(data, size)`.
    SndStandbyBattleWave @ 0x0088_7450 => extern "C" fn snd_standby_battle_wave(data: u32, size: i32) {
        s::snd_standby_battle_wave(&mut game(), data, size);
    }
    /// `SndStandbyBattleWaveBlock(data, size)`.
    SndStandbyBattleWaveBlock @ 0x0088_74d0 => extern "C" fn snd_standby_battle_wave_block(data: u32, size: i32) {
        s::snd_standby_battle_wave_block(&mut game(), data, size);
    }
    /// `SndStandbySepWave()`.
    SndStandbySepWave @ 0x0088_7550 => extern "C" fn snd_standby_sep_wave() {
        s::snd_standby_sep_wave(&mut game());
    }
    /// `SndStandbyStream(id)`.
    SndStandbyStream @ 0x0088_7570 => extern "C" fn snd_standby_stream(id: u32) {
        s::snd_standby_stream(&mut game(), id);
    }
    /// `SndStartBattleStream(a, b, c)`.
    SndStartBattleStream @ 0x0088_75a0 => extern "C" fn snd_start_battle_stream(a: u32, b: u32, c: u32) -> u32 {
        s::snd_start_battle_stream(&mut game(), a, b, c)
    }
    /// `SndStartShoutStream(a, b, volume)`.
    SndStartShoutStream @ 0x0088_75d0 => extern "C" fn snd_start_shout_stream(a: u32, b: u32, volume: u32) -> u32 {
        s::snd_start_shout_stream(&mut game(), a, b, volume)
    }
    /// `SndStartStream()`.
    SndStartStream @ 0x0088_7630 => extern "C" fn snd_start_stream() -> u32 {
        s::snd_start_stream(&mut game())
    }
    /// `SndStartStream2(pan, volume)`.
    SndStartStream2 @ 0x0088_7690 => extern "C" fn snd_start_stream2(pan: u32, volume: u32) -> u32 {
        s::snd_start_stream2(&mut game(), pan, volume)
    }
    /// `SndStatusUpdate()`.
    SndStatusUpdate @ 0x0088_7700 => extern "C" fn snd_status_update() {
        s::snd_status_update(&mut game());
    }
    /// `SndStopBattleStream()`.
    SndStopBattleStream @ 0x0088_7760 => extern "C" fn snd_stop_battle_stream() -> u32 {
        s::snd_stop_battle_stream(&mut game())
    }
    /// `SndStopStream()`.
    SndStopStream @ 0x0088_7780 => extern "C" fn snd_stop_stream() -> u32 {
        s::snd_stop_stream(&mut game())
    }
    /// `SndStreamDVDPos(id)`.
    SndStreamDVDPos @ 0x0088_7790 => extern "C" fn snd_stream_dvdpos(id: u32) -> u32 {
        s::snd_stream_dvd_pos(&mut mem(), id)
    }
    /// `SndStreamPCName(id, buf)`.
    SndStreamPCName @ 0x0088_7870 => extern "C" fn snd_stream_pcname(id: u32, buf: u32) {
        s::snd_stream_pc_name(&mut mem(), id, buf);
    }
    /// `SndSyncMusicStandby()`.
    SndSyncMusicStandby @ 0x0088_7900 => extern "C" fn snd_sync_music_standby() -> u32 {
        s::snd_sync_music_standby()
    }
    /// `SndSyncTrans(block)`.
    SndSyncTrans @ 0x0088_7910 => extern "C" fn snd_sync_trans(block: u32) -> u32 {
        s::snd_sync_trans(block)
    }
    /// `SndSyncWaveTrans(block)`.
    SndSyncWaveTrans @ 0x0088_7930 => extern "C" fn snd_sync_wave_trans(block: u32) -> u32 {
        s::snd_sync_wave_trans(block)
    }
    /// `SndToUpper(text)`.
    SndToUpper @ 0x0088_7940 => extern "C" fn snd_to_upper(text: u32) {
        s::snd_to_upper(&mut mem(), text);
    }
    /// `SndVagStreamVolumeDirect(volume)`.
    SndVagStreamVolumeDirect @ 0x0088_7990 => extern "C" fn snd_vag_stream_volume_direct(volume: u32) -> u32 {
        s::snd_vag_stream_volume_direct(&mut game(), volume)
    }
    /// `SndVagStreamVolumeTranslate(frames, volume)`.
    SndVagStreamVolumeTranslate @ 0x0088_79b0 => extern "C" fn snd_vag_stream_volume_translate(frames: u32, volume: u32) -> u32 {
        s::snd_vag_stream_volume_translate(&mut game(), frames, volume)
    }
    /// `FmodCmd_soundSetMute(fmod, args)`.
    FmodCmd_soundSetMute @ 0x0070_7b90 => extern "C" fn fmod_cmd_sound_set_mute(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x00, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundInit(fmod, args)`.
    FmodCmd_soundInit @ 0x0070_7bb0 => extern "C" fn fmod_cmd_sound_init(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x01, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundUpdate(fmod, args)`.
    FmodCmd_soundUpdate @ 0x0070_7bc0 => extern "C" fn fmod_cmd_sound_update(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x02, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundLoop(fmod, args)`.
    FmodCmd_soundLoop @ 0x0070_7bd0 => extern "C" fn fmod_cmd_sound_loop(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x03, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundStageFlush(fmod, args)`.
    FmodCmd_soundStageFlush @ 0x0070_7be0 => extern "C" fn fmod_cmd_sound_stage_flush(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x04, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundPlayStatus(fmod, args)`.
    FmodCmd_soundPlayStatus @ 0x0070_7c00 => extern "C" fn fmod_cmd_sound_play_status(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x05, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_shoutStart(fmod, args)`.
    FmodCmd_shoutStart @ 0x0070_7c10 => extern "C" fn fmod_cmd_shout_start(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x06, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceReleaseData(fmod, args)`.
    FmodCmd_voiceReleaseData @ 0x0070_7c30 => extern "C" fn fmod_cmd_voice_release_data(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x07, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceSetPaused(fmod, args)`.
    FmodCmd_voiceSetPaused @ 0x0070_7c40 => extern "C" fn fmod_cmd_voice_set_paused(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x08, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceDataChange(fmod, args)`.
    FmodCmd_voiceDataChange @ 0x0070_7c60 => extern "C" fn fmod_cmd_voice_data_change(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x09, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceStop(fmod, args)`.
    FmodCmd_voiceStop @ 0x0070_7c80 => extern "C" fn fmod_cmd_voice_stop(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x0a, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voicePanTranslate(fmod, args)`.
    FmodCmd_voicePanTranslate @ 0x0070_7c90 => extern "C" fn fmod_cmd_voice_pan_translate(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x0b, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voicePanDirect(fmod, args)`.
    FmodCmd_voicePanDirect @ 0x0070_7cf0 => extern "C" fn fmod_cmd_voice_pan_direct(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x0c, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceVolumeTranslate(fmod, args)`.
    FmodCmd_voiceVolumeTranslate @ 0x0070_7d10 => extern "C" fn fmod_cmd_voice_volume_translate(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x0d, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceVolumeDirect(fmod, args)`.
    FmodCmd_voiceVolumeDirect @ 0x0070_7d70 => extern "C" fn fmod_cmd_voice_volume_direct(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x0e, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceStart(fmod, args)`.
    FmodCmd_voiceStart @ 0x0070_7da0 => extern "C" fn fmod_cmd_voice_start(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x0f, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceRead(fmod, args)`.
    FmodCmd_voiceRead @ 0x0070_7dc0 => extern "C" fn fmod_cmd_voice_read(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x10, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voicePlayStatus(fmod, args)`.
    FmodCmd_voicePlayStatus @ 0x0070_7de0 => extern "C" fn fmod_cmd_voice_play_status(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x11, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_voiceIsPaused(fmod, args)`.
    FmodCmd_voiceIsPaused @ 0x0070_7df0 => extern "C" fn fmod_cmd_voice_is_paused(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x12, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_getNowPlayMusic(fmod, args)`.
    FmodCmd_getNowPlayMusic @ 0x0070_7e00 => extern "C" fn fmod_cmd_get_now_play_music(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x13, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicSyncStandby(fmod, args)`.
    FmodCmd_musicSyncStandby @ 0x0070_7e10 => extern "C" fn fmod_cmd_music_sync_standby(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x14, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicDataSend(fmod, args)`.
    FmodCmd_musicDataSend @ 0x0070_7e20 => extern "C" fn fmod_cmd_music_data_send(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x15, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicPlayState(fmod, args)`.
    FmodCmd_musicPlayState @ 0x0070_7e40 => extern "C" fn fmod_cmd_music_play_state(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x16, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicCrossPlay(fmod, args)`.
    FmodCmd_musicCrossPlay @ 0x0070_7e60 => extern "C" fn fmod_cmd_music_cross_play(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x17, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicPlayFast(fmod, args)`.
    FmodCmd_musicPlayFast @ 0x0070_7e80 => extern "C" fn fmod_cmd_music_play_fast(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x18, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicReplayPointSet(fmod, args)`.
    FmodCmd_musicReplayPointSet @ 0x0070_7ea0 => extern "C" fn fmod_cmd_music_replay_point_set(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x19, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicReplayPointClear(fmod, args)`.
    FmodCmd_musicReplayPointClear @ 0x0070_7eb0 => extern "C" fn fmod_cmd_music_replay_point_clear(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x1a, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicPhrasePlay(fmod, args)`.
    FmodCmd_musicPhrasePlay @ 0x0070_7ed0 => extern "C" fn fmod_cmd_music_phrase_play(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x1b, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicVolumeFadeOut(fmod, args)`.
    FmodCmd_musicVolumeFadeOut @ 0x0070_7ef0 => extern "C" fn fmod_cmd_music_volume_fade_out(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x1c, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicTempoTranslate(fmod, args)`.
    FmodCmd_musicTempoTranslate @ 0x0070_7f10 => extern "C" fn fmod_cmd_music_tempo_translate(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x1d, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicTempoDirect(fmod, args)`.
    FmodCmd_musicTempoDirect @ 0x0070_7f70 => extern "C" fn fmod_cmd_music_tempo_direct(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x1e, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicPitchTranslate(fmod, args)`.
    FmodCmd_musicPitchTranslate @ 0x0070_7f90 => extern "C" fn fmod_cmd_music_pitch_translate(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x1f, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicPitchDirect(fmod, args)`.
    FmodCmd_musicPitchDirect @ 0x0070_7fb0 => extern "C" fn fmod_cmd_music_pitch_direct(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x20, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicMultiPlay(fmod, args)`.
    FmodCmd_musicMultiPlay @ 0x0070_7fd0 => extern "C" fn fmod_cmd_music_multi_play(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x21, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicVolumeTranslate(fmod, args)`.
    FmodCmd_musicVolumeTranslate @ 0x0070_7ff0 => extern "C" fn fmod_cmd_music_volume_translate(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x22, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicVolumeDirect(fmod, args)`.
    FmodCmd_musicVolumeDirect @ 0x0070_8050 => extern "C" fn fmod_cmd_music_volume_direct(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x23, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicMuteSet(fmod, args)`.
    FmodCmd_musicMuteSet @ 0x0070_8080 => extern "C" fn fmod_cmd_music_mute_set(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x24, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicClear(fmod, args)`.
    FmodCmd_musicClear @ 0x0070_80a0 => extern "C" fn fmod_cmd_music_clear(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x25, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicStop(fmod, args)`.
    FmodCmd_musicStop @ 0x0070_80c0 => extern "C" fn fmod_cmd_music_stop(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x26, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicStart(fmod, args)`.
    FmodCmd_musicStart @ 0x0070_80e0 => extern "C" fn fmod_cmd_music_start(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x27, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicRead(fmod, args)`.
    FmodCmd_musicRead @ 0x0070_8100 => extern "C" fn fmod_cmd_music_read(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x28, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_shutDown(fmod, args)`.
    FmodCmd_shutDown @ 0x0070_8120 => extern "C" fn fmod_cmd_shut_down(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x29, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_initEventSystem(fmod, args)`.
    FmodCmd_initEventSystem @ 0x0070_8130 => extern "C" fn fmod_cmd_init_event_system(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x2a, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_initLoadMasterBank(fmod, args)`.
    FmodCmd_initLoadMasterBank @ 0x0070_8140 => extern "C" fn fmod_cmd_init_load_master_bank(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x2b, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicStopAll(fmod, args)`.
    FmodCmd_musicStopAll @ 0x0070_8150 => extern "C" fn fmod_cmd_music_stop_all(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x2c, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicClearAll(fmod, args)`.
    FmodCmd_musicClearAll @ 0x0070_8160 => extern "C" fn fmod_cmd_music_clear_all(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x2d, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicSwitchBGM(fmod, args)`.
    FmodCmd_musicSwitchBGM @ 0x0070_8170 => extern "C" fn fmod_cmd_music_switch_bgm(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x2e, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_musicPauseAll(fmod, args)`.
    FmodCmd_musicPauseAll @ 0x0070_8190 => extern "C" fn fmod_cmd_music_pause_all(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x2f, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxLoad(fmod, args)`.
    FmodCmd_sfxLoad @ 0x0070_81b0 => extern "C" fn fmod_cmd_sfx_load(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x30, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxUnloadAllData(fmod, args)`.
    FmodCmd_sfxUnloadAllData @ 0x0070_81d0 => extern "C" fn fmod_cmd_sfx_unload_all_data(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x31, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxUnloadBySeq(fmod, args)`.
    FmodCmd_sfxUnloadBySeq @ 0x0070_81e0 => extern "C" fn fmod_cmd_sfx_unload_by_seq(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x32, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxStart(fmod, args)`.
    FmodCmd_sfxStart @ 0x0070_8200 => extern "C" fn fmod_cmd_sfx_start(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x33, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxForceStart(fmod, args)`.
    FmodCmd_sfxForceStart @ 0x0070_8220 => extern "C" fn fmod_cmd_sfx_force_start(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x34, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxDelayStart(fmod, args)`.
    FmodCmd_sfxDelayStart @ 0x0070_8240 => extern "C" fn fmod_cmd_sfx_delay_start(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x35, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxStop(fmod, args)`.
    FmodCmd_sfxStop @ 0x0070_8260 => extern "C" fn fmod_cmd_sfx_stop(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x36, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxStopAllNonBlockingSound(fmod, args)`.
    FmodCmd_sfxStopAllNonBlockingSound @ 0x0070_8280 => extern "C" fn fmod_cmd_sfx_stop_all_non_blocking_sound(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x37, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxStopNonBlockingSound(fmod, args)`.
    FmodCmd_sfxStopNonBlockingSound @ 0x0070_82b0 => extern "C" fn fmod_cmd_sfx_stop_non_blocking_sound(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x38, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxStopAllNonloopSound(fmod, args)`.
    FmodCmd_sfxStopAllNonloopSound @ 0x0070_82f0 => extern "C" fn fmod_cmd_sfx_stop_all_nonloop_sound(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x39, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxPause(fmod, args)`.
    FmodCmd_sfxPause @ 0x0070_82d0 => extern "C" fn fmod_cmd_sfx_pause(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x3a, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxSetVolume(fmod, args)`.
    FmodCmd_sfxSetVolume @ 0x0070_8300 => extern "C" fn fmod_cmd_sfx_set_volume(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x3b, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxSetVolume3(fmod, args)`.
    FmodCmd_sfxSetVolume3 @ 0x0070_8320 => extern "C" fn fmod_cmd_sfx_set_volume3(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x3c, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxSetPan(fmod, args)`.
    FmodCmd_sfxSetPan @ 0x0070_8340 => extern "C" fn fmod_cmd_sfx_set_pan(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x3d, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxSetPan3(fmod, args)`.
    FmodCmd_sfxSetPan3 @ 0x0070_8360 => extern "C" fn fmod_cmd_sfx_set_pan3(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x3e, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxIsPlaying(fmod, args)`.
    FmodCmd_sfxIsPlaying @ 0x0070_8380 => extern "C" fn fmod_cmd_sfx_is_playing(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x3f, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxPlayBattleStreaming(fmod, args)`.
    FmodCmd_sfxPlayBattleStreaming @ 0x0070_83a0 => extern "C" fn fmod_cmd_sfx_play_battle_streaming(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x40, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_sfxIspaused(fmod, args)`.
    FmodCmd_sfxIspaused @ 0x0070_83c0 => extern "C" fn fmod_cmd_sfx_ispaused(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x41, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundSetMasterVolume(fmod, args)`.
    FmodCmd_soundSetMasterVolume @ 0x0070_83d0 => extern "C" fn fmod_cmd_sound_set_master_volume(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x42, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundSetMusicVolume(fmod, args)`.
    FmodCmd_soundSetMusicVolume @ 0x0070_8400 => extern "C" fn fmod_cmd_sound_set_music_volume(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x43, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundSetVoiceVolume(fmod, args)`.
    FmodCmd_soundSetVoiceVolume @ 0x0070_8430 => extern "C" fn fmod_cmd_sound_set_voice_volume(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x44, fmod, args).unwrap_or(0)
    }
    /// `FmodCmd_soundSetSfxVolume(fmod, args)`.
    FmodCmd_soundSetSfxVolume @ 0x0070_8460 => extern "C" fn fmod_cmd_sound_set_sfx_volume(fmod: u32, args: u32) -> u32 {
        x::handle(&mut game(), 0x45, fmod, args).unwrap_or(0)
    }
    /// `FfxFmod_getNowPlayMusic(fmod)`.
    FfxFmod_getNowPlayMusic @ 0x0070_67d0 => extern "thiscall" fn ffx_fmod_get_now_play_music(fmod: u32) -> u32 {
        x::get_now_play_music(&game(), fmod)
    }
    /// `FfxFmod_initLoadMasterBank(fmod)`.
    FfxFmod_initLoadMasterBank @ 0x0070_6920 => extern "thiscall" fn ffx_fmod_init_load_master_bank(fmod: u32) {
        x::init_load_master_bank(&mut game(), fmod);
    }
    /// `FfxFmod_musicClear(fmod, id)`.
    FfxFmod_musicClear @ 0x0070_69e0 => extern "thiscall" fn ffx_fmod_music_clear(fmod: u32, id: u32) -> u32 {
        x::music_clear(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicClearAll(fmod)`.
    FfxFmod_musicClearAll @ 0x0070_69f0 => extern "thiscall" fn ffx_fmod_music_clear_all(fmod: u32) -> u32 {
        x::music_clear_all(&mut game(), fmod)
    }
    /// `FfxFmod_musicVolumeFadeOut(fmod, id, frames)`.
    FfxFmod_musicVolumeFadeOut @ 0x0070_6a00 => extern "thiscall" fn ffx_fmod_music_volume_fade_out(fmod: u32, id: u32, frames: u32) -> u32 {
        x::music_cross_play(&mut game(), fmod, id, frames)
    }
    /// `FfxFmod_musicDataSend(fmod, id)`.
    FfxFmod_musicDataSend @ 0x0070_6a20 => extern "thiscall" fn ffx_fmod_music_data_send(fmod: u32, id: u32) -> u32 {
        x::music_data_send(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicMultiPlay(fmod, id)`.
    FfxFmod_musicMultiPlay @ 0x0070_6a40 => extern "thiscall" fn ffx_fmod_music_multi_play(fmod: u32, id: u32) -> u32 {
        x::music_multi_play(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicPauseAll(fmod, flag)`.
    FfxFmod_musicPauseAll @ 0x0070_6a70 => extern "thiscall" fn ffx_fmod_music_pause_all(fmod: u32, flag: u32) -> u32 {
        x::music_pause_all(&mut game(), fmod, flag)
    }
    /// `FfxFmod_musicPhrasePlay(fmod, id, phrase)`.
    FfxFmod_musicPhrasePlay @ 0x0070_6a80 => extern "thiscall" fn ffx_fmod_music_phrase_play(fmod: u32, id: u32, _phrase: u32) -> u32 {
        x::music_multi_play(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicPlayState(fmod, id)`.
    FfxFmod_musicPlayState @ 0x0070_6ad0 => extern "thiscall" fn ffx_fmod_music_play_state(fmod: u32, id: u32) -> u32 {
        x::music_play_state(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicRead(fmod, id)`.
    FfxFmod_musicRead @ 0x0070_6ae0 => extern "thiscall" fn ffx_fmod_music_read(fmod: u32, id: u32) -> u32 {
        x::music_read(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicReplayPointClear(fmod, id)`.
    FfxFmod_musicReplayPointClear @ 0x0070_6b00 => extern "thiscall" fn ffx_fmod_music_replay_point_clear(fmod: u32, id: u32) -> u32 {
        x::music_replay_point_clear(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicReplayPointSet(fmod)`.
    FfxFmod_musicReplayPointSet @ 0x0070_6b30 => extern "thiscall" fn ffx_fmod_music_replay_point_set(fmod: u32) -> u32 {
        x::music_replay_point_set(&mut game(), fmod)
    }
    /// `FfxFmod_musicStart(fmod, id)`.
    FfxFmod_musicStart @ 0x0070_6b90 => extern "thiscall" fn ffx_fmod_music_start(fmod: u32, id: u32) -> u32 {
        x::music_start(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicStop(fmod, id)`.
    FfxFmod_musicStop @ 0x0070_6ba0 => extern "thiscall" fn ffx_fmod_music_stop(fmod: u32, id: u32) -> u32 {
        x::music_stop(&mut game(), fmod, id)
    }
    /// `FfxFmod_musicStopAll(fmod)`.
    FfxFmod_musicStopAll @ 0x0070_6bb0 => extern "thiscall" fn ffx_fmod_music_stop_all(fmod: u32) -> u32 {
        x::music_stop_all(&mut game(), fmod)
    }
    /// `FfxFmod_musicSwitchBGM(fmod, t)`.
    FfxFmod_musicSwitchBGM @ 0x0070_6bc0 => extern "thiscall" fn ffx_fmod_music_switch_bgm(fmod: u32, t: u32) -> u32 {
        x::music_switch_bgm(&mut game(), fmod, t)
    }
    /// `FfxFmod_musicSyncStandby(fmod)`.
    FfxFmod_musicSyncStandby @ 0x0070_6bf0 => extern "thiscall" fn ffx_fmod_music_sync_standby(fmod: u32) -> u32 {
        x::music_sync_standby(&mut game(), fmod)
    }
    /// `FfxFmod_musicVolumeDirect(fmod, id, volume, c)`.
    FfxFmod_musicVolumeDirect @ 0x0070_6c20 => extern "thiscall" fn ffx_fmod_music_volume_direct(fmod: u32, id: u32, volume: u32, c: u32) -> u32 {
        x::music_volume_direct(&mut game(), fmod, id, volume, c)
    }
    /// `FfxFmod_musicVolumeFadeOut2(fmod, id, frames)`.
    FfxFmod_musicVolumeFadeOut2 @ 0x0070_6c40 => extern "thiscall" fn ffx_fmod_music_volume_fade_out2(fmod: u32, id: u32, frames: u32) -> u32 {
        x::music_volume_fade_out(&mut game(), fmod, id, frames)
    }
    /// `FfxFmod_musicVolumeTranslate(fmod, id, frames, volume)`.
    FfxFmod_musicVolumeTranslate @ 0x0070_6c50 => extern "thiscall" fn ffx_fmod_music_volume_translate(fmod: u32, id: u32, frames: u32, volume: u32) -> u32 {
        x::music_volume_translate(&mut game(), fmod, id, frames, volume)
    }
    /// `FfxFmod_sfxDelayStart(fmod, a, b, c, d)`.
    FfxFmod_sfxDelayStart @ 0x0070_6db0 => extern "thiscall" fn ffx_fmod_sfx_delay_start(fmod: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_DELAY_START, &[a, b, c, d])
    }
    /// `FfxFmod_sfxForceStart(fmod, a, b, c)`.
    FfxFmod_sfxForceStart @ 0x0070_6dd0 => extern "thiscall" fn ffx_fmod_sfx_force_start(fmod: u32, a: u32, b: u32, c: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_FORCE_START, &[a, b, c])
    }
    /// `FfxFmod_sfxIsPlaying(fmod, id)`.
    FfxFmod_sfxIsPlaying @ 0x0070_6df0 => extern "thiscall" fn ffx_fmod_sfx_is_playing(fmod: u32, id: u32) -> u32 {
        x::sfx_is_playing(&mut game(), fmod, id)
    }
    /// `FfxFmod_sfxIspaused(fmod)`.
    FfxFmod_sfxIspaused @ 0x0070_6e10 => extern "thiscall" fn ffx_fmod_sfx_ispaused(fmod: u32) -> u32 {
        x::sfx_is_paused(&game(), fmod)
    }
    /// `FfxFmod_sfxLoad(fmod, id, flag)`.
    FfxFmod_sfxLoad @ 0x0070_6e20 => extern "thiscall" fn ffx_fmod_sfx_load(fmod: u32, id: u32, flag: u32) -> u32 {
        x::sfx_load(&mut game(), fmod, id, flag)
    }
    /// `FfxFmod_sfxPause(fmod, flag)`.
    FfxFmod_sfxPause @ 0x0070_6e50 => extern "thiscall" fn ffx_fmod_sfx_pause(fmod: u32, flag: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_PAUSE, &[flag])
    }
    /// `FfxFmod_sfxPlayBattleStreaming(fmod, a, b, c)`.
    FfxFmod_sfxPlayBattleStreaming @ 0x0070_6e70 => extern "thiscall" fn ffx_fmod_sfx_play_battle_streaming(fmod: u32, a: u32, b: u32, c: u32) -> u32 {
        x::sfx_play_battle_streaming(&mut game(), fmod, a, b, c)
    }
    /// `FfxFmod_sfxSetPan(fmod, id, pan)`.
    FfxFmod_sfxSetPan @ 0x0070_6e90 => extern "thiscall" fn ffx_fmod_sfx_set_pan(fmod: u32, id: u32, pan: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_SET_PAN, &[id, pan])
    }
    /// `FfxFmod_sfxSetPan3(fmod, id, pan, frames)`.
    FfxFmod_sfxSetPan3 @ 0x0070_6eb0 => extern "thiscall" fn ffx_fmod_sfx_set_pan3(fmod: u32, id: u32, pan: u32, frames: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_SET_PAN_TRANSLATE, &[id, pan, frames])
    }
    /// `FfxFmod_sfxSetVolume(fmod, id, volume)`.
    FfxFmod_sfxSetVolume @ 0x0070_6ed0 => extern "thiscall" fn ffx_fmod_sfx_set_volume(fmod: u32, id: u32, volume: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_SET_VOLUME, &[id, volume])
    }
    /// `FfxFmod_sfxSetVolume3(fmod, id, volume, frames)`.
    FfxFmod_sfxSetVolume3 @ 0x0070_6ef0 => extern "thiscall" fn ffx_fmod_sfx_set_volume3(fmod: u32, id: u32, volume: u32, frames: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_SET_VOLUME_TRANSLATE, &[id, volume, frames])
    }
    /// `FfxFmod_sfxStart(fmod, id, b, c)`.
    FfxFmod_sfxStart @ 0x0070_6f10 => extern "thiscall" fn ffx_fmod_sfx_start(fmod: u32, id: u32, b: u32, c: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_START, &[id, b, c])
    }
    /// `FfxFmod_sfxStop(fmod, id)`.
    FfxFmod_sfxStop @ 0x0070_6f30 => extern "thiscall" fn ffx_fmod_sfx_stop(fmod: u32, id: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_STOP, &[id])
    }
    /// `FfxFmod_sfxStopAllNonBlockingSound(fmod, a)`.
    FfxFmod_sfxStopAllNonBlockingSound @ 0x0070_6f50 => extern "thiscall" fn ffx_fmod_sfx_stop_all_non_blocking_sound(fmod: u32, a: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_STOP_ALL_NON_BLOCKING, &[a, 0])
    }
    /// `FfxFmod_sfxStopAllNonloopSound(fmod)`.
    FfxFmod_sfxStopAllNonloopSound @ 0x0070_6f70 => extern "thiscall" fn ffx_fmod_sfx_stop_all_nonloop_sound(fmod: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_STOP_ALL_NONLOOP, &[])
    }
    /// `FfxFmod_sfxStopNonBlockingSound(fmod, id)`.
    FfxFmod_sfxStopNonBlockingSound @ 0x0070_6f80 => extern "thiscall" fn ffx_fmod_sfx_stop_non_blocking_sound(fmod: u32, id: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_STOP_NON_BLOCKING, &[id])
    }
    /// `FfxFmod_sfxUnloadAllData(fmod)`.
    FfxFmod_sfxUnloadAllData @ 0x0070_6fa0 => extern "thiscall" fn ffx_fmod_sfx_unload_all_data(fmod: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_UNLOAD_PROJECTS, &[0, 0])
    }
    /// `FfxFmod_sfxUnloadBySeq(fmod, seq)`.
    FfxFmod_sfxUnloadBySeq @ 0x0070_6fb0 => extern "thiscall" fn ffx_fmod_sfx_unload_by_seq(fmod: u32, seq: u32) -> u32 {
        x::sfx_call(&mut game(), fmod, xc::SFX_UNLOAD_BY_SEQ, &[seq])
    }
    /// `FfxFmod_shoutStart(fmod, a, b, c)`.
    FfxFmod_shoutStart @ 0x0070_6fd0 => extern "thiscall" fn ffx_fmod_shout_start(fmod: u32, a: u32, b: u32, c: u32) -> u32 {
        x::shout_start(&mut game(), fmod, a, b, c)
    }
    /// `FfxFmod_soundInit(fmod)`.
    FfxFmod_soundInit @ 0x0070_7080 => extern "thiscall" fn ffx_fmod_sound_init(fmod: u32) -> u32 {
        x::sound_init(&mut game(), fmod)
    }
    /// `FfxFmod_soundPlayStatus(fmod)`.
    FfxFmod_soundPlayStatus @ 0x0070_71c0 => extern "thiscall" fn ffx_fmod_sound_play_status(fmod: u32) -> u32 {
        x::sound_play_status(&game(), fmod)
    }
    /// `FfxFmod_soundSetMasterVolume(fmod, volume)`.
    FfxFmod_soundSetMasterVolume @ 0x0070_71e0 => extern "thiscall" fn ffx_fmod_sound_set_master_volume(fmod: u32, volume: u32) -> u32 {
        x::sound_set_volume(&mut game(), fmod, x::MASTER_VOLUME, volume)
    }
    /// `FfxFmod_soundSetMusicVolume(fmod, volume)`.
    FfxFmod_soundSetMusicVolume @ 0x0070_7280 => extern "thiscall" fn ffx_fmod_sound_set_music_volume(fmod: u32, volume: u32) -> u32 {
        x::sound_set_volume(&mut game(), fmod, x::MUSIC_VOLUME, volume)
    }
    /// `FfxFmod_soundSetMute(fmod, id, kind)`.
    FfxFmod_soundSetMute @ 0x0070_72f0 => extern "thiscall" fn ffx_fmod_sound_set_mute(fmod: u32, id: u32, kind: u32) -> u32 {
        x::sound_set_mute(&mut game(), fmod, id, kind)
    }
    /// `FfxFmod_soundSetSfxVolume(fmod, volume)`.
    FfxFmod_soundSetSfxVolume @ 0x0070_7330 => extern "thiscall" fn ffx_fmod_sound_set_sfx_volume(fmod: u32, volume: u32) -> u32 {
        x::sound_set_volume(&mut game(), fmod, x::SFX_VOLUME, volume)
    }
    /// `FfxFmod_soundSetVoiceVolume(fmod, volume)`.
    FfxFmod_soundSetVoiceVolume @ 0x0070_73a0 => extern "thiscall" fn ffx_fmod_sound_set_voice_volume(fmod: u32, volume: u32) -> u32 {
        x::sound_set_volume(&mut game(), fmod, x::VOICE_VOLUME, volume)
    }
    /// `FfxFmod_soundStageFlush(fmod, stage)`.
    FfxFmod_soundStageFlush @ 0x0070_7410 => extern "thiscall" fn ffx_fmod_sound_stage_flush(fmod: u32, stage: u32) -> u32 {
        x::sound_stage_flush(&mut game(), fmod, stage)
    }
    /// `FfxFmod_soundUpdate(fmod)`.
    FfxFmod_soundUpdate @ 0x0070_7420 => extern "thiscall" fn ffx_fmod_sound_update(fmod: u32) -> u32 {
        x::sound_update(&mut game(), fmod)
    }
    /// `FfxFmod_voiceDataChange(fmod, id)`.
    FfxFmod_voiceDataChange @ 0x0070_7480 => extern "thiscall" fn ffx_fmod_voice_data_change(fmod: u32, id: u32) -> u32 {
        x::voice_data_change(&mut game(), fmod, id)
    }
    /// `FfxFmod_voiceIsPaused(fmod)`.
    FfxFmod_voiceIsPaused @ 0x0070_74b0 => extern "thiscall" fn ffx_fmod_voice_is_paused(fmod: u32) -> u32 {
        x::voice_is_paused(&game(), fmod)
    }
    /// `FfxFmod_voicePlayStatus(fmod)`.
    FfxFmod_voicePlayStatus @ 0x0070_74e0 => extern "thiscall" fn ffx_fmod_voice_play_status(fmod: u32) -> u32 {
        x::sound_play_status(&game(), fmod)
    }
    /// `FfxFmod_voiceRead(fmod, id)`.
    FfxFmod_voiceRead @ 0x0070_74f0 => extern "thiscall" fn ffx_fmod_voice_read(fmod: u32, id: u32) -> u32 {
        x::voice_read(&mut game(), fmod, id)
    }
    /// `FfxFmod_voiceReleaseData(fmod)`.
    FfxFmod_voiceReleaseData @ 0x0070_7510 => extern "thiscall" fn ffx_fmod_voice_release_data(fmod: u32) -> u32 {
        x::voice_release_data(&mut game(), fmod)
    }
    /// `FfxFmod_voiceSetPaused(fmod, flag)`.
    FfxFmod_voiceSetPaused @ 0x0070_7520 => extern "thiscall" fn ffx_fmod_voice_set_paused(fmod: u32, flag: u32) -> u32 {
        x::voice_set_paused(&mut game(), fmod, flag)
    }
    /// `FfxFmod_voiceSetStartSignal(fmod, on)`.
    FfxFmod_voiceSetStartSignal @ 0x0070_7530 => extern "thiscall" fn ffx_fmod_voice_set_start_signal(fmod: u32, on: u32) -> u32 {
        x::voice_set_start_signal(&mut game(), fmod, on)
    }
    /// `FfxFmod_voiceStart(fmod, pan, volume)`.
    FfxFmod_voiceStart @ 0x0070_7540 => extern "thiscall" fn ffx_fmod_voice_start(fmod: u32, pan: u32, volume: u32) -> u32 {
        x::voice_start(&mut game(), fmod, pan, volume)
    }
    /// `FfxFmod_voiceStop(fmod)`.
    FfxFmod_voiceStop @ 0x0070_7550 => extern "thiscall" fn ffx_fmod_voice_stop(fmod: u32) -> u32 {
        x::voice_stop(&mut game(), fmod)
    }
    /// `FfxFmod_voiceVolumeDirect(fmod, volume, b)`.
    FfxFmod_voiceVolumeDirect @ 0x0070_7560 => extern "thiscall" fn ffx_fmod_voice_volume_direct(fmod: u32, volume: u32, b: u32) -> u32 {
        x::voice_volume_direct(&mut game(), fmod, volume, b)
    }
    /// `FfxFmod_voiceVolumeTranslate(fmod, frames, volume)`.
    FfxFmod_voiceVolumeTranslate @ 0x0070_7580 => extern "thiscall" fn ffx_fmod_voice_volume_translate(fmod: u32, frames: u32, volume: u32) -> u32 {
        x::voice_volume_translate(&mut game(), fmod, frames, volume)
    }
    /// `FmodMusic_playState(music, id)`.
    FmodMusic_playState @ 0x0070_9060 => extern "thiscall" fn fmod_music_play_state(music: u32, _id: u32) -> u32 {
        x::music_object_play_state(&game(), music)
    }
    /// `FmodMusic_syncStandby(music)`.
    FmodMusic_syncStandby @ 0x0070_9d80 => extern "thiscall" fn fmod_music_sync_standby(music: u32) -> u32 {
        x::music_object_sync_standby(&mut game(), music)
    }
    /// `FmodMusic_volumeFadeOut(music, id, frames)`.
    FmodMusic_volumeFadeOut @ 0x0070_9eb0 => extern "thiscall" fn fmod_music_volume_fade_out(music: u32, id: u32, frames: u32) -> u32 {
        x::music_object_volume_fade_out(&mut game(), music, id, frames)
    }
    /// `FmodVoice_setEntireVolume(voice, volume)`.
    FmodVoice_setEntireVolume @ 0x0070_b2b0 => extern "thiscall" fn fmod_voice_set_entire_volume(voice: u32, volume: u32) -> u32 {
        x::voice_set_entire_volume(&mut game(), voice, volume).unwrap_or(0)
    }
    /// `FmodVoice_setMute(voice, id)`.
    FmodVoice_setMute @ 0x0070_b330 => extern "thiscall" fn fmod_voice_set_mute(voice: u32, id: u32) -> u32 {
        x::voice_set_mute(&mut game(), voice, id)
    }
    /// `FmodVoice_setPaused(voice, flag)`.
    FmodVoice_setPaused @ 0x0070_b370 => extern "thiscall" fn fmod_voice_set_paused(voice: u32, flag: u32) -> u32 {
        x::voice_object_set_paused(&mut game(), voice, flag)
    }
    /// `FfxFmod_soundInit_setLang(voice, lang)`.
    FfxFmod_soundInit_setLang @ 0x0070_b3f0 => extern "thiscall" fn ffx_fmod_sound_init_set_lang(voice: u32, lang: u32) -> u32 {
        x::voice_set_lang(&mut game(), voice, lang)
    }
    /// `FmodVoice_shoutStart(voice, a, b, c, d)`.
    FmodVoice_shoutStart @ 0x0070_b430 => extern "thiscall" fn fmod_voice_shout_start(voice: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        x::voice_shout_start(&mut game(), voice, [a, b, c, d])
    }
    /// `FmodVoice_soundStageFlush(voice, stage)`.
    FmodVoice_soundStageFlush @ 0x0070_b570 => extern "thiscall" fn fmod_voice_sound_stage_flush(voice: u32, stage: u32) -> u32 {
        x::voice_sound_stage_flush(&mut game(), voice, stage)
    }
    /// `SfxEventTree_Find(tree, key)`.
    SfxEventTree_Find @ 0x0070_ba70 => extern "thiscall" fn sfx_event_tree_find(tree: u32, key: u32) -> u32 {
        x::sfx_find(&game(), tree, ffx_bridge::Mem::read(&game(), key))
    }
    /// `FmodSfx_canStartNewEvent(sfx, id)`.
    FmodSfx_canStartNewEvent @ 0x0070_c7d0 => extern "thiscall" fn fmod_sfx_can_start_new_event(sfx: u32, id: u32) -> u32 {
        x::sfx_can_start_new_event(&mut game(), sfx, id)
    }
    /// `FmodSfx_delayStart(sfx, id, b, c, d)`.
    FmodSfx_delayStart @ 0x0070_cc30 => extern "thiscall" fn fmod_sfx_delay_start(sfx: u32, id: u32, b: u32, c: u32, d: u32) -> u32 {
        x::sfx_delay_start(&mut game(), sfx, [id, b, c, d]).unwrap_or(0)
    }
    /// `FmodSfx_forceStart(sfx, id, b, c)`.
    FmodSfx_forceStart @ 0x0070_ccd0 => extern "thiscall" fn fmod_sfx_force_start(sfx: u32, id: u32, b: u32, c: u32) -> u32 {
        x::sfx_force_start(&mut game(), sfx, id, b, c).unwrap_or(0)
    }
    /// `FmodSfx_Start(sfx, id, b, c)`.
    FmodSfx_Start @ 0x0070_d890 => extern "thiscall" fn fmod_sfx_start(sfx: u32, id: u32, b: u32, c: u32) -> u32 {
        x::sfx_start(&mut game(), sfx, id, b, c).unwrap_or(0)
    }
    /// `FmodSfx_setPan(sfx, id, pan)`.
    FmodSfx_setPan @ 0x0070_d640 => extern "thiscall" fn fmod_sfx_set_pan(sfx: u32, id: u32, pan: u32) {
        x::sfx_set_pan(&mut game(), sfx, id, pan);
    }
    /// `FmodSfx_setPan3(sfx, id, pan, frames)`.
    FmodSfx_setPan3 @ 0x0070_d690 => extern "thiscall" fn fmod_sfx_set_pan3(sfx: u32, id: u32, pan: u32, frames: u32) {
        x::sfx_set_pan_translate(&mut game(), sfx, id, pan, frames);
    }
    /// `FmodSfx_setVolume(sfx, id, volume)`.
    FmodSfx_setVolume @ 0x0070_d6e0 => extern "thiscall" fn fmod_sfx_set_volume(sfx: u32, id: u32, volume: u32) {
        x::sfx_set_volume(&mut game(), sfx, id, volume);
    }
    /// `FmodSfx_setVolume3(sfx, id, volume, frames)`.
    FmodSfx_setVolume3 @ 0x0070_d730 => extern "thiscall" fn fmod_sfx_set_volume3(sfx: u32, id: u32, volume: u32, frames: u32) {
        x::sfx_set_volume_translate(&mut game(), sfx, id, volume, frames);
    }
    /// `FmodSfx_stop(sfx, id)`.
    FmodSfx_stop @ 0x0070_d8f0 => extern "thiscall" fn fmod_sfx_stop(sfx: u32, id: u32) {
        x::sfx_stop(&mut game(), sfx, id);
    }
    /// `FmodSfx_stopNonBlockingSound(sfx, id)`.
    FmodSfx_stopNonBlockingSound @ 0x0070_db90 => extern "thiscall" fn fmod_sfx_stop_non_blocking_sound(sfx: u32, id: u32) -> u32 {
        x::sfx_stop_non_blocking(&mut game(), sfx, id).unwrap_or(0)
    }
}
