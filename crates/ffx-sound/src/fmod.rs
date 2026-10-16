//! The game side of the FMOD layer: the PS2 sound API's `i*` entry points
//! (`iMusic*`, `iSfx*`), which queue commands for the FMOD thread
//! (`docs/formats/sound.md`, "FMOD command queue").
//!
//! The game never calls FMOD from the game thread: `iMusicStart(id)` queues
//! command `0x27` with `Fmod_PushCommand` on the `FmodManager`
//! (`g_FmodManager`, `0x00ce9010`), and the FMOD thread runs the queue at
//! 30 Hz through the handler table at `0x00c3a368`. A few commands are
//! synchronous (`FmodManager_registCommandSync`): the caller waits for the
//! thread's answer. Both stay original here (Win32 mutexes and events); the
//! bridges reach them through [`FmodHost::call_this5`].

/// The `FmodManager` object (`0x50` bytes, created by `iFmodInitialize`).
pub const FMOD_MANAGER: u32 = 0x00ce_9010;

/// `Fmod_PushCommand` (thiscall `FmodManager`; cmd, a, b, c, d): queues a
/// command in the 150-entry ring.
pub const PUSH_COMMAND: u32 = 0x006f_a300;
/// `FmodManager_registCommandSync` (thiscall `FmodManager`; cmd, a, b, c,
/// d): runs a command on the FMOD thread and returns its result.
pub const REGIST_COMMAND_SYNC: u32 = 0x006f_a500;

/// The bridge's host: game memory, cdecl calls ([`ffx_bridge::Host`]) and
/// thiscall calls into the original code.
pub trait FmodHost: ffx_bridge::Host {
    /// Calls the thiscall function at `addr` (rebased) with `this` in `ecx`
    /// and five stack arguments, which it pops; returns `EAX`.
    fn call_this5(&mut self, addr: u32, this: u32, args: [u32; 5]) -> u32;
}

/// The original functions, over game memory.
#[allow(clippy::many_single_char_names)] // the originals' arguments
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{FMOD_MANAGER, FmodHost, PUSH_COMMAND, REGIST_COMMAND_SYNC};

    /// `g_FmodManager`.
    pub const MANAGER: Global<u32> = Global::new(FMOD_MANAGER);

    /// `iFmodFinalize` 0x00679fb0: deletes the manager (`0x006fa190`).
    pub const FMOD_FINALIZE: u32 = 0x006f_a190;
    /// `iFmodInitialize` 0x00679fc0: creates the manager (`0x006fa0b0`).
    pub const FMOD_INITIALIZE: u32 = 0x006f_a0b0;

    /// `FmodManager +8`: the `FfxFmod` object.
    pub const MANAGER_FMOD: u32 = 0x8;
    /// `FfxFmod +0xc`: the `FmodMusic` object.
    pub const FMOD_MUSIC: u32 = 0xc;
    /// `FmodMusic +0x18`: the music the next start signal is for.
    pub const MUSIC_START_ID: u32 = 0x18;
    /// `FmodMusic +0x20`: the start signal.
    pub const MUSIC_START_SIGNAL: u32 = 0x20;

    /// `Fmod_PushCommand(mgr; cmd, a, b, c, d)`.
    pub fn push_command(h: &mut impl FmodHost, mgr: u32, cmd: u32, args: [u32; 4]) -> u32 {
        let f = h.rebase(PUSH_COMMAND);
        h.call_this5(f, mgr, [cmd, args[0], args[1], args[2], args[3]])
    }

    /// `FmodManager_registCommandSync(mgr; cmd, a, b, c, d)`.
    pub fn regist_command_sync(h: &mut impl FmodHost, mgr: u32, cmd: u32, args: [u32; 4]) -> u32 {
        let f = h.rebase(REGIST_COMMAND_SYNC);
        h.call_this5(f, mgr, [cmd, args[0], args[1], args[2], args[3]])
    }

    /// `Fmod_PushCommand0(mgr; cmd)` 0x006fa280: the command with no
    /// arguments.
    pub fn push0(h: &mut impl FmodHost, mgr: u32, cmd: u32) -> u32 {
        push_command(h, mgr, cmd, [0; 4])
    }

    /// `Fmod_PushCommand1(mgr; cmd, a)` 0x006fa2a0.
    pub fn push1(h: &mut impl FmodHost, mgr: u32, cmd: u32, a: u32) -> u32 {
        push_command(h, mgr, cmd, [a, 0, 0, 0])
    }

    /// `Fmod_PushCommand2(mgr; cmd, a, b)` 0x006fa2c0.
    pub fn push2(h: &mut impl FmodHost, mgr: u32, cmd: u32, a: u32, b: u32) -> u32 {
        push_command(h, mgr, cmd, [a, b, 0, 0])
    }

    /// `Fmod_PushCommand3(mgr; cmd, a, b, c)` 0x006fa2e0.
    pub fn push3(h: &mut impl FmodHost, mgr: u32, cmd: u32, a: u32, b: u32, c: u32) -> u32 {
        push_command(h, mgr, cmd, [a, b, c, 0])
    }

    /// `Fmod_SyncCommand0(mgr; cmd)` 0x006fa480.
    pub fn sync0(h: &mut impl FmodHost, mgr: u32, cmd: u32) -> u32 {
        regist_command_sync(h, mgr, cmd, [0; 4])
    }

    /// `Fmod_SyncCommand1(mgr; cmd, a)` 0x006fa4a0.
    pub fn sync1(h: &mut impl FmodHost, mgr: u32, cmd: u32, a: u32) -> u32 {
        regist_command_sync(h, mgr, cmd, [a, 0, 0, 0])
    }

    /// `Fmod_SyncCommand2(mgr; cmd, a, b)` 0x006fa4c0.
    pub fn sync2(h: &mut impl FmodHost, mgr: u32, cmd: u32, a: u32, b: u32) -> u32 {
        regist_command_sync(h, mgr, cmd, [a, b, 0, 0])
    }

    /// `Fmod_SyncCommand3(mgr; cmd, a, b, c)` 0x006fa4e0.
    pub fn sync3(h: &mut impl FmodHost, mgr: u32, cmd: u32, a: u32, b: u32, c: u32) -> u32 {
        regist_command_sync(h, mgr, cmd, [a, b, c, 0])
    }

    /// The commands the `i*` functions queue (`docs/formats/sound.md`).
    pub mod cmd {
        /// `iMusicDataSend(id)`.
        pub const MUSIC_DATA_SEND: u32 = 0x15;
        /// `iMusicPlayState(id)` (synchronous).
        pub const MUSIC_PLAY_STATE: u32 = 0x16;
        /// `iMusicCrossPlay(id, frames)`.
        pub const MUSIC_CROSS_PLAY: u32 = 0x17;
        /// `iMusicPlayFast(id)`.
        pub const MUSIC_PLAY_FAST: u32 = 0x18;
        /// `iMusicReplayPointSet()`.
        pub const MUSIC_REPLAY_POINT_SET: u32 = 0x19;
        /// `iMusicReplayPointClear(id)`.
        pub const MUSIC_REPLAY_POINT_CLEAR: u32 = 0x1a;
        /// `iMusicPhrasePlay(id, phrase)`.
        pub const MUSIC_PHRASE_PLAY: u32 = 0x1b;
        /// `iMusicVolumeFadeOut(id, frames)`.
        pub const MUSIC_VOLUME_FADE_OUT: u32 = 0x1c;
        /// `iMusicTempoTranslate(id, tempo)`.
        pub const MUSIC_TEMPO_TRANSLATE: u32 = 0x1d;
        /// `iMusicTempoDirect(tempo)`.
        pub const MUSIC_TEMPO_DIRECT: u32 = 0x1e;
        /// `iMusicPitchTranslate(a, b, c)`.
        pub const MUSIC_PITCH_TRANSLATE: u32 = 0x1f;
        /// `iMusicPitchDirect(a, b)`.
        pub const MUSIC_PITCH_DIRECT: u32 = 0x20;
        /// `iMusicMultiPlay(id)`.
        pub const MUSIC_MULTI_PLAY: u32 = 0x21;
        /// `iMusicVolumeTranslate(id, frames, volume)`.
        pub const MUSIC_VOLUME_TRANSLATE: u32 = 0x22;
        /// `iMusicVolumeDirect(id, volume, 1)`.
        pub const MUSIC_VOLUME_DIRECT: u32 = 0x23;
        /// `iMusicMuteSet(a, b)`.
        pub const MUSIC_MUTE_SET: u32 = 0x24;
        /// `iMusicClear(id)`.
        pub const MUSIC_CLEAR: u32 = 0x25;
        /// `iMusicStop(id)`.
        pub const MUSIC_STOP: u32 = 0x26;
        /// `iMusicStart(id)`.
        pub const MUSIC_START: u32 = 0x27;
        /// `iMusicRead(id)`.
        pub const MUSIC_READ: u32 = 0x28;
        /// `iMusicStopAll()`.
        pub const MUSIC_STOP_ALL: u32 = 0x2c;
        /// `iMusicClearAll()`.
        pub const MUSIC_CLEAR_ALL: u32 = 0x2d;
        /// `iMuiscSetBgmType(type)` (0 or 1).
        pub const MUSIC_SET_BGM_TYPE: u32 = 0x2e;
        /// `iMusicSetPause(flag)`.
        pub const MUSIC_SET_PAUSE: u32 = 0x2f;
        /// `iSfxLoadData(id, flag)`.
        pub const SFX_LOAD_DATA: u32 = 0x30;
        /// `iSfxStart(id, pan, volume)`.
        pub const SFX_START: u32 = 0x33;
        /// `iSfxCommand34(a, b, c)`.
        pub const SFX_34: u32 = 0x34;
        /// `iSfxCommand35(a, b, c, d)`.
        pub const SFX_35: u32 = 0x35;
        /// `iSfxPause(id)`.
        pub const SFX_PAUSE: u32 = 0x3a;
        /// `iSfxSetVolume(id, volume)`: replaces a queued one for the same id.
        pub const SFX_SET_VOLUME: u32 = 0x3b;
        /// `iSfxSetPan(id, pan)`: replaces a queued one for the same id.
        pub const SFX_SET_PAN: u32 = 0x3d;
        /// `iSfxSetPanTranslate(id, pan, frames)`.
        pub const SFX_SET_PAN_TRANSLATE: u32 = 0x3e;
        /// `iSfxIsPlaying(id)` (synchronous).
        pub const SFX_IS_PLAYING: u32 = 0x3f;
        /// `iSfxPlayBattleStreaming(a, b, c)`.
        pub const SFX_PLAY_BATTLE_STREAMING: u32 = 0x40;
        /// `iSfxCommand41()` (synchronous).
        pub const SFX_41: u32 = 0x41;
        /// `iMusicSyncStandby()` (synchronous).
        pub const MUSIC_SYNC_STANDBY: u32 = 0x14;
    }

    fn mgr(h: &impl Mem) -> u32 {
        MANAGER.get(h)
    }

    /// `iFmodFinalize()` (0x00679fb0) -> 0.
    pub fn i_fmod_finalize(h: &mut impl FmodHost) -> u32 {
        let f = h.rebase(FMOD_FINALIZE);
        h.call(f, &[]);
        0
    }

    /// `iFmodInitialize()` (0x00679fc0) -> 0.
    pub fn i_fmod_initialize(h: &mut impl FmodHost) -> u32 {
        let f = h.rebase(FMOD_INITIALIZE);
        h.call(f, &[]);
        0
    }

    /// `iMuiscSetBgmType(type)` (0x00679fe0): 1 and command `0x2e` for
    /// type 0 or 1, else 0.
    pub fn i_music_set_bgm_type(h: &mut impl FmodHost, t: u32) -> u32 {
        if t != 0 && t != 1 {
            return 0;
        }
        let m = mgr(h);
        push1(h, m, cmd::MUSIC_SET_BGM_TYPE, t);
        1
    }

    // One `i*` function's queueing (`g_FmodManager` read at the call), by
    // argument count, returning what the queue returns.
    fn q0(h: &mut impl FmodHost, cmd: u32) -> u32 {
        let m = mgr(h);
        push0(h, m, cmd)
    }

    fn q1(h: &mut impl FmodHost, cmd: u32, a: u32) -> u32 {
        let m = mgr(h);
        push1(h, m, cmd, a)
    }

    fn q2(h: &mut impl FmodHost, cmd: u32, a: u32, b: u32) -> u32 {
        let m = mgr(h);
        push2(h, m, cmd, a, b)
    }

    fn q3(h: &mut impl FmodHost, cmd: u32, args: [u32; 3]) -> u32 {
        let m = mgr(h);
        push3(h, m, cmd, args[0], args[1], args[2])
    }

    fn s0(h: &mut impl FmodHost, cmd: u32) -> u32 {
        let m = mgr(h);
        sync0(h, m, cmd)
    }

    fn s1(h: &mut impl FmodHost, cmd: u32, a: u32) -> u32 {
        let m = mgr(h);
        sync1(h, m, cmd, a)
    }

    /// `iMusicClear(id)` (0x0067a010).
    pub fn i_music_clear(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_CLEAR, id)
    }

    /// `iMusicClearAll()` (0x0067a030).
    pub fn i_music_clear_all(h: &mut impl FmodHost) -> u32 {
        q0(h, cmd::MUSIC_CLEAR_ALL)
    }

    /// `iMusicCrossPlay(id, frames)` (0x0067a040).
    pub fn i_music_cross_play(h: &mut impl FmodHost, id: u32, frames: u32) -> u32 {
        q2(h, cmd::MUSIC_CROSS_PLAY, id, frames)
    }

    /// `iMusicDataSend(id)` (0x0067a060): the id as a 64-bit pair (`cdq`).
    pub fn i_music_data_send(h: &mut impl FmodHost, id: u32) -> u32 {
        let high = (id.cast_signed() >> 31).cast_unsigned();
        q2(h, cmd::MUSIC_DATA_SEND, id, high)
    }

    /// `iMusicMultiPlay(id)` (0x0067a090).
    pub fn i_music_multi_play(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_MULTI_PLAY, id)
    }

    /// `iMusicMuteSet(a, b)` (0x0067a0b0).
    pub fn i_music_mute_set(h: &mut impl FmodHost, a: u32, b: u32) -> u32 {
        q2(h, cmd::MUSIC_MUTE_SET, a, b)
    }

    /// `iMusicPhrasePlay(id, phrase)` (0x0067a0d0).
    pub fn i_music_phrase_play(h: &mut impl FmodHost, id: u32, phrase: u32) -> u32 {
        q2(h, cmd::MUSIC_PHRASE_PLAY, id, phrase)
    }

    /// `iMusicPitchDirect(a, b)` (0x0067a0f0).
    pub fn i_music_pitch_direct(h: &mut impl FmodHost, a: u32, b: u32) -> u32 {
        q2(h, cmd::MUSIC_PITCH_DIRECT, a, b)
    }

    /// `iMusicPitchTranslate(a, b, c)` (0x0067a110).
    pub fn i_music_pitch_translate(h: &mut impl FmodHost, a: u32, b: u32, c: u32) -> u32 {
        q3(h, cmd::MUSIC_PITCH_TRANSLATE, [a, b, c])
    }

    /// `iMusicPlayFast(id)` (0x0067a130).
    pub fn i_music_play_fast(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_PLAY_FAST, id)
    }

    /// `iMusicPlayState(id)` (0x0067a150), synchronous.
    pub fn i_music_play_state(h: &mut impl FmodHost, id: u32) -> u32 {
        s1(h, cmd::MUSIC_PLAY_STATE, id)
    }

    /// `iMusicRead(id)` (0x0067a170).
    pub fn i_music_read(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_READ, id)
    }

    /// `iMusicReplayPointClear(id)` (0x0067a190).
    pub fn i_music_replay_point_clear(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_REPLAY_POINT_CLEAR, id)
    }

    /// `iMusicReplayPointSet()` (0x0067a1b0).
    pub fn i_music_replay_point_set(h: &mut impl FmodHost) -> u32 {
        q0(h, cmd::MUSIC_REPLAY_POINT_SET)
    }

    /// `iMusicSetPause(flag)` (0x0067a1c0): the FMOD thread pauses the music events (`FmodMusic` 0x00709000).
    pub fn i_music_set_pause(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_SET_PAUSE, id)
    }

    /// `iMusicStart(id)` (0x0067a1e0) -> 0: the start signal
    /// ([`fmod_manager_signal_music_start`]), then command `0x27`.
    pub fn i_music_start(h: &mut impl FmodHost, id: u32) -> u32 {
        let m = mgr(h);
        fmod_manager_signal_music_start(h, m, id);
        let m = mgr(h);
        push1(h, m, cmd::MUSIC_START, id);
        0
    }

    /// `iMusicStop(id)` (0x0067a210).
    pub fn i_music_stop(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::MUSIC_STOP, id)
    }

    /// `iMusicStopAll()` (0x0067a230).
    pub fn i_music_stop_all(h: &mut impl FmodHost) -> u32 {
        q0(h, cmd::MUSIC_STOP_ALL)
    }

    /// `iMusicSyncStandby()` (0x0067a240), synchronous.
    pub fn i_music_sync_standby(h: &mut impl FmodHost) -> u32 {
        s0(h, cmd::MUSIC_SYNC_STANDBY)
    }

    /// `iMusicTempoDirect(tempo)` (0x0067a250).
    pub fn i_music_tempo_direct(h: &mut impl FmodHost, tempo: u32) -> u32 {
        q1(h, cmd::MUSIC_TEMPO_DIRECT, tempo)
    }

    /// `iMusicTempoTranslate(a, b)` (0x0067a270).
    pub fn i_music_tempo_translate(h: &mut impl FmodHost, a: u32, b: u32) -> u32 {
        q2(h, cmd::MUSIC_TEMPO_TRANSLATE, a, b)
    }

    /// `iMusicVolumeDirect(id, volume)` (0x0067a290): command `0x23` with a
    /// third argument 1.
    pub fn i_music_volume_direct(h: &mut impl FmodHost, id: u32, volume: u32) -> u32 {
        q3(h, cmd::MUSIC_VOLUME_DIRECT, [id, volume, 1])
    }

    /// `iMusicVolumeFadeOut(id, frames)` (0x0067a2b0).
    pub fn i_music_volume_fade_out(h: &mut impl FmodHost, id: u32, frames: u32) -> u32 {
        q2(h, cmd::MUSIC_VOLUME_FADE_OUT, id, frames)
    }

    /// `iMusicVolumeTranslate(id, frames, volume)` (0x0067a2d0).
    pub fn i_music_volume_translate(
        h: &mut impl FmodHost,
        id: u32,
        frames: u32,
        volume: u32,
    ) -> u32 {
        q3(h, cmd::MUSIC_VOLUME_TRANSLATE, [id, frames, volume])
    }

    /// `iSfxCommand35(a, b, c, d)` (0x0067a2f0): `Fmod_PushCommand` itself.
    pub fn i_sfx_command35(h: &mut impl FmodHost, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let m = mgr(h);
        push_command(h, m, cmd::SFX_35, [a, b, c, d])
    }

    /// `iSfxCommand34(a, b, c)` (0x0067a310).
    pub fn i_sfx_command34(h: &mut impl FmodHost, a: u32, b: u32, c: u32) -> u32 {
        q3(h, cmd::SFX_34, [a, b, c])
    }

    /// `iSfxCommand41()` (0x0067a330), synchronous.
    pub fn i_sfx_command41(h: &mut impl FmodHost) -> u32 {
        s0(h, cmd::SFX_41)
    }

    /// `iSfxIsPlaying(id)` (0x0067a340), synchronous.
    pub fn i_sfx_is_playing(h: &mut impl FmodHost, id: u32) -> u32 {
        s1(h, cmd::SFX_IS_PLAYING, id)
    }

    /// `iSfxLoadData(id, flag)` (0x0067a360).
    pub fn i_sfx_load_data(h: &mut impl FmodHost, id: u32, flag: u32) -> u32 {
        q2(h, cmd::SFX_LOAD_DATA, id, flag)
    }

    /// `iSfxPause(id)` (0x0067a380).
    pub fn i_sfx_pause(h: &mut impl FmodHost, id: u32) -> u32 {
        q1(h, cmd::SFX_PAUSE, id)
    }

    /// `iSfxPlayBattleStreaming(a, b, c)` (0x0067a3a0).
    pub fn i_sfx_play_battle_streaming(h: &mut impl FmodHost, a: u32, b: u32, c: u32) -> u32 {
        q3(h, cmd::SFX_PLAY_BATTLE_STREAMING, [a, b, c])
    }

    /// `iSfxSetPan(id, pan)` (0x0067a3c0).
    pub fn i_sfx_set_pan(h: &mut impl FmodHost, id: u32, pan: u32) -> u32 {
        q2(h, cmd::SFX_SET_PAN, id, pan)
    }

    /// `iSfxSetPanTranslate(id, pan, frames)` (0x0067a3e0).
    pub fn i_sfx_set_pan_translate(h: &mut impl FmodHost, id: u32, pan: u32, frames: u32) -> u32 {
        q3(h, cmd::SFX_SET_PAN_TRANSLATE, [id, pan, frames])
    }

    /// `FfxFmod_musicSeqCheck(fmod; id)` (0x00706b50, thiscall): whether
    /// `id` is a music of the PC table (below `0xb6`).
    #[must_use]
    pub fn ffx_fmod_music_seq_check(id: u32) -> u32 {
        u32::from(id <= 0xb5)
    }

    /// `FfxFmod_setMusicStartId(fmod; id)` (0x00706b70, thiscall): the
    /// music's start id (`FmodMusic +0x18`); returns `id`.
    pub fn ffx_fmod_set_music_start_id(h: &mut impl Mem, fmod: u32, id: u32) -> u32 {
        let music = h.read::<u32>(fmod + FMOD_MUSIC);
        h.write(music + MUSIC_START_ID, id);
        id
    }

    /// `FfxFmod_setMusicStartSignal(fmod; on)` (0x00706b80, thiscall)
    /// (`FmodMusic +0x20`); returns `on`.
    pub fn ffx_fmod_set_music_start_signal(h: &mut impl Mem, fmod: u32, on: u32) -> u32 {
        let music = h.read::<u32>(fmod + FMOD_MUSIC);
        h.write(music + MUSIC_START_SIGNAL, on);
        on
    }

    /// `FmodManager_signalMusicStart(mgr; id)` (0x006fa690, thiscall): for
    /// a PC music, records it and raises the start signal. Returns what
    /// the last call left in `eax`.
    pub fn fmod_manager_signal_music_start(h: &mut impl Mem, mgr: u32, id: u32) -> u32 {
        let seq = ffx_fmod_music_seq_check(id);
        if seq == 0 {
            return seq;
        }
        let fmod = h.read::<u32>(mgr + MANAGER_FMOD);
        ffx_fmod_set_music_start_id(h, fmod, id);
        let fmod = h.read::<u32>(mgr + MANAGER_FMOD);
        ffx_fmod_set_music_start_signal(h, fmod, 1)
    }
}
