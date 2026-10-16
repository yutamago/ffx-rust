//! `ffx-sound`: the PS2 sound API (`Snd*`), the FMOD queue's `i*` entry
//! points and the `FmodManager` command helpers.

use ffx_bridge::{Host, Mem, RawMem};
use ffx_sound::fmod::{FmodHost, bridge as f};
use ffx_sound::snd::bridge as s;

use super::{call_cdecl, mem, ported};

/// The game as the sound bridges' host: its memory, cdecl calls
/// ([`call_cdecl`]) and the thiscall `FmodManager` commands.
struct Game(RawMem);

impl Mem for Game {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.0.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl Host for Game {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        call_cdecl(addr, args)
    }
}

impl FmodHost for Game {
    fn call_this5(&mut self, addr: u32, this: u32, a: [u32; 5]) -> u32 {
        type F = unsafe extern "thiscall" fn(u32, u32, u32, u32, u32, u32) -> u32;
        // SAFETY: the bridges pass `Fmod_PushCommand` and
        // `FmodManager_registCommandSync`, thiscall functions of exactly
        // five stack arguments (`ret 0x14`).
        unsafe {
            let g: F = std::mem::transmute::<usize, F>(addr as usize);
            g(this, a[0], a[1], a[2], a[3], a[4])
        }
    }
}

fn game() -> Game {
    Game(mem())
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
    /// `SndSepKind(id)`.
    SndSepKind @ 0x0088_5d00 => extern "C" fn snd_sep_kind(id: u32) -> u32 {
        s::snd_sep_kind(id)
    }
    /// `SndSepId(id)`.
    SndSepId @ 0x0088_5d10 => extern "C" fn snd_sep_id(id: u32) -> u32 {
        s::snd_sep_id(id)
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
    /// `SndClearFootStatus()`.
    SndClearFootStatus @ 0x0088_5e50 => extern "C" fn snd_clear_foot_status() {
        s::snd_clear_foot_status(&mut mem());
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
    /// `SndEventSetFoot3dCtrlp(p)`.
    SndEventSetFoot3dCtrlp @ 0x0088_5fc0 => extern "C" fn snd_event_set_foot3d_ctrlp(p: u32) -> u32 {
        s::snd_event_set_foot3d_ctrlp(&mut mem(), p)
    }
    /// `SndEventSetPcToActorFoot(party, actor)`.
    SndEventSetPcToActorFoot @ 0x0088_5fd0 => extern "C" fn snd_event_set_pc_to_actor_foot(party: i32, actor: u32) {
        s::snd_event_set_pc_to_actor_foot(&mut mem(), party, actor);
    }
    /// `SndEventSetSoundEvent(table)`.
    SndEventSetSoundEvent @ 0x0088_5ff0 => extern "C" fn snd_event_set_sound_event(table: u32) -> u32 {
        s::snd_event_set_sound_event(&mut mem(), table)
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
    /// `SndGetKinuMask()`.
    SndGetKinuMask @ 0x0088_63b0 => extern "C" fn snd_get_kinu_mask() -> u32 {
        s::snd_get_kinu_mask(&mem())
    }
    /// `SndGetMusicPlayState(id)`.
    SndGetMusicPlayState @ 0x0088_63c0 => extern "C" fn snd_get_music_play_state(id: u32) -> u32 {
        s::snd_get_music_play_state(&mut game(), id)
    }
    /// `SndGetMusicVolume(id)`.
    SndGetMusicVolume @ 0x0088_63d0 => extern "C" fn snd_get_music_volume(id: u32) -> u32 {
        s::snd_get_music_volume(&mem(), id)
    }
    /// `SndGetNowPlayMusic()`.
    SndGetNowPlayMusic @ 0x0088_63f0 => extern "C" fn snd_get_now_play_music() -> u32 {
        s::snd_get_now_play_music(&mem())
    }
    /// `SndGetSePlayState()`.
    SndGetSePlayState @ 0x0088_6400 => extern "C" fn snd_get_se_play_state() -> u32 {
        s::snd_get_se_play_state(&mut game())
    }
    /// `SndGetSepNumber(sep)`.
    SndGetSepNumber @ 0x0088_6410 => extern "C" fn snd_get_sep_number(sep: u32) -> u32 {
        s::snd_get_sep_number(&mem(), sep)
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
    /// `SndGetStreamState()`.
    SndGetStreamState @ 0x0088_64b0 => extern "C" fn snd_get_stream_state() -> u32 {
        s::snd_get_stream_state(&mut mem())
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
    /// `SndIsReadShoutVoiceStatus()`.
    SndIsReadShoutVoiceStatus @ 0x0088_6640 => extern "C" fn snd_is_read_shout_voice_status() -> u32 {
        s::snd_is_read_shout_voice_status(&mem())
    }
    /// `SndIsReadVagVoiceStatus()`.
    SndIsReadVagVoiceStatus @ 0x0088_6660 => extern "C" fn snd_is_read_vag_voice_status() -> u32 {
        s::snd_is_read_vag_voice_status(&mem())
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
    /// `SndPlayFootStatus()`.
    SndPlayFootStatus @ 0x0088_6be0 => extern "C" fn snd_play_foot_status() {
        s::snd_play_foot_status(&mut mem());
    }
    /// `SndPopFootStatus()`.
    SndPopFootStatus @ 0x0088_6c10 => extern "C" fn snd_pop_foot_status() -> u32 {
        s::snd_pop_foot_status(&mut mem())
    }
    /// `SndPushFootStatus()`.
    SndPushFootStatus @ 0x0088_6c40 => extern "C" fn snd_push_foot_status() -> u32 {
        s::snd_push_foot_status(&mut mem())
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
}
