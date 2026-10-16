//! The FMOD thread's side of the command queue: the 70 handlers of the
//! table at `0x00c3a368`, the `FfxFmod` methods they call, and the simpler
//! methods of the three objects those hand the work to (`FmodMusic`,
//! `FmodVoice`, `FmodSfx`; `docs/formats/sound.md`, "FMOD command queue").
//!
//! The FMOD libraries stay original: their C++ members are `__stdcall`
//! (`this` the first stack argument), reached through the import thunks at
//! `0x00948c70..` with [`Conv::Stdcall`]. So do the object methods that
//! drive FMOD events in depth (start, read, cross-fades); the bridges call
//! them by `thiscall`.
//!
//! `FfxFmod` (the `FmodManager`'s `+8`): `+4` the `EventSystem`, `+8` the
//! low-level `System`, `+0xc` `FmodMusic`, `+0x10` `FmodVoice`, `+0x14`
//! `FmodSfx`, `+0x18` master volume, `+0x1c` music, `+0x20` voice, `+0x24`
//! sfx (f32, 0..1).
//!
//! The PC build keeps the functions of a source file in alphabetical
//! order; the names come from the Switch build's symbols by that order.

/// The original functions, over game memory.
#[allow(clippy::many_single_char_names)] // the originals' arguments
pub mod bridge {
    use ffx_bridge::{Mem, x87};

    use crate::fmod::{Conv, FmodHost as Host};

    /// `FfxFmod +4`: the `EventSystem`.
    pub const EVENT_SYSTEM: u32 = 0x4;
    /// `FfxFmod +8`: the low-level `System`.
    pub const SYSTEM: u32 = 0x8;
    /// `FfxFmod +0xc`: `FmodMusic`.
    pub const MUSIC: u32 = 0xc;
    /// `FfxFmod +0x10`: `FmodVoice`.
    pub const VOICE: u32 = 0x10;
    /// `FfxFmod +0x14`: `FmodSfx`.
    pub const SFX: u32 = 0x14;
    /// `FfxFmod +0x18`: master volume (f32).
    pub const MASTER_VOLUME: u32 = 0x18;
    /// `FfxFmod +0x1c`: music volume (f32).
    pub const MUSIC_VOLUME: u32 = 0x1c;
    /// `FfxFmod +0x20`: voice volume (f32).
    pub const VOICE_VOLUME: u32 = 0x20;
    /// `FfxFmod +0x24`: sfx volume (f32).
    pub const SFX_VOLUME: u32 = 0x24;

    /// `FmodMusic +0x10`: the 0xb6 music records of 0x3c bytes (`+0` the
    /// FMOD `Event`, `+4` the replay point flag, `+8` fading, `+0x20` the
    /// play state).
    pub const MUSIC_RECORDS: u32 = 0x10;
    /// `FmodMusic +0x18`: the music playing (the start signal's).
    pub const MUSIC_START_ID: u32 = 0x18;
    /// `FmodMusic +0x20`: the start signal.
    pub const MUSIC_START_SIGNAL: u32 = 0x20;
    /// `FmodMusic +0x28`: the voice's first word, kept on a voice change.
    pub const MUSIC_VOICE: u32 = 0x28;
    /// One music record's size.
    pub const MUSIC_RECORD: u32 = 0x3c;
    /// The last music id.
    pub const MUSIC_LAST: u32 = 0xb5;

    /// `FmodVoice +4`: English voice (byte).
    pub const VOICE_ENGLISH: u32 = 0x4;
    /// `FmodVoice +8`: the language.
    pub const VOICE_LANG: u32 = 0x8;
    /// `FmodVoice +0x14`: the voice's FMOD `Event`.
    pub const VOICE_EVENT: u32 = 0x14;
    /// `FmodVoice +0x24`: the shout player's last argument.
    pub const VOICE_SHOUT_ARG: u32 = 0x24;
    /// `FmodVoice +0x2c`: the stage-flush byte.
    pub const VOICE_FLUSH: u32 = 0x2c;
    /// `FmodVoice +0x40`, `+0x44`: the play status words (`+0x44` the
    /// start signal).
    pub const VOICE_STATUS: [u32; 2] = [0x40, 0x44];
    /// `FmodVoice +0x50`: the voice volume (f32).
    pub const VOICE_VOLUME_LEVEL: u32 = 0x50;
    /// `FmodVoice +0x54`: paused.
    pub const VOICE_PAUSED: u32 = 0x54;
    /// `FmodVoice +0x80`: the mute id.
    pub const VOICE_MUTE: u32 = 0x80;
    /// `FmodVoice +0x84`: the shout player.
    pub const VOICE_SHOUT: u32 = 0x84;
    /// `FmodVoice +0x88`: the entire volume (f32).
    pub const VOICE_ENTIRE_VOLUME: u32 = 0x88;

    /// `FmodSfx +0`: paused (byte).
    pub const SFX_PAUSED: u32 = 0;
    /// `FmodSfx +2`: English (byte).
    pub const SFX_ENGLISH: u32 = 2;
    /// `FmodSfx +0x64`: the playing events by sound id, a binary tree
    /// (the head's `+0xc` the root, the head itself the leaf; nodes `+0`
    /// left, `+4` right, `+0xc` the id, `+0x10` a list `{count, _,
    /// events*}`).
    pub const SFX_EVENTS: u32 = 0x64;

    /// The two stage globals `FmodVoice::soundStageFlush` clears.
    pub const STAGE_FLUSH: [u32; 2] = [0x00ce_c174, 0x00ce_c184];
    /// `FMOD::Event::getState` (stdcall `this, unsigned*`), import thunk.
    pub const EVENT_GET_STATE: u32 = 0x0094_8ca6;
    /// `FMOD::Event::setPaused` (stdcall `this, bool`), import thunk.
    pub const EVENT_SET_PAUSED: u32 = 0x0094_8cbe;
    /// `FMOD::EventSystem::update` (stdcall `this`), import thunk.
    pub const EVENT_SYSTEM_UPDATE: u32 = 0x0094_8c82;
    /// `FMOD_EVENT_STATE_PLAYING`.
    pub const STATE_PLAYING: u32 = 8;

    /// The originals called (thiscall unless noted), with their stack
    /// arguments.
    pub mod callee {
        /// `FmodMusic::clear(id)`.
        pub const MUSIC_CLEAR: u32 = 0x0070_8720;
        /// `FmodMusic::clearAll()`.
        pub const MUSIC_CLEAR_ALL: u32 = 0x0070_8810;
        /// `FmodMusic::crossPlay(id, frames, event system)`.
        pub const MUSIC_CROSS_PLAY: u32 = 0x0070_8900;
        /// `FmodMusic::dataSend(id, event system)`.
        pub const MUSIC_DATA_SEND: u32 = 0x0070_8a00;
        /// `FmodMusic::initData(event system)`.
        pub const MUSIC_INIT_DATA: u32 = 0x0070_8c00;
        /// `FmodMusic::initTempData(event system)`: empty on PC.
        pub const MUSIC_INIT_TEMP_DATA: u32 = 0x0070_8d30;
        /// `FmodMusic::pauseAll(flag)`.
        pub const MUSIC_PAUSE_ALL: u32 = 0x0070_9000;
        /// `FmodMusic::playState(id)`.
        pub const MUSIC_PLAY_STATE: u32 = 0x0070_9060;
        /// `FmodMusic::read(id, event system)`.
        pub const MUSIC_READ: u32 = 0x0070_9080;
        /// `FmodMusic::setEntireVolume(f32)`.
        pub const MUSIC_SET_ENTIRE_VOLUME: u32 = 0x0070_93e0;
        /// `FmodMusic::setMute(id)`.
        pub const MUSIC_SET_MUTE: u32 = 0x0070_9520;
        /// `FmodMusic::start(id)`.
        pub const MUSIC_START: u32 = 0x0070_96f0;
        /// `FmodMusic::stop(id)`.
        pub const MUSIC_STOP: u32 = 0x0070_99a0;
        /// `FmodMusic::stopAll()`.
        pub const MUSIC_STOP_ALL: u32 = 0x0070_9b10;
        /// `FmodMusic::switchBGM(type, event system)`.
        pub const MUSIC_SWITCH_BGM: u32 = 0x0070_9c80;
        /// `FmodMusic::syncStandby()`.
        pub const MUSIC_SYNC_STANDBY: u32 = 0x0070_9d80;
        /// `FmodMusic::volumeDirect(id, f32 volume, c)`.
        pub const MUSIC_VOLUME_DIRECT: u32 = 0x0070_9dd0;
        /// `FmodMusic::volumeFadeOut(id, frames)`.
        pub const MUSIC_VOLUME_FADE_OUT: u32 = 0x0070_9eb0;
        /// `FmodMusic::volumeTranslate(id, frames, volume)`.
        pub const MUSIC_VOLUME_TRANSLATE: u32 = 0x0070_9ff0;
        /// `FmodVoice::clear()`.
        pub const VOICE_CLEAR: u32 = 0x0070_a590;
        /// `FmodVoice_dataChange(id, event system)`.
        pub const VOICE_DATA_CHANGE: u32 = 0x0070_a630;
        /// `FmodVoice_initData(event system, system)`.
        pub const VOICE_INIT_DATA: u32 = 0x0070_aa30;
        /// `FmodVoice_initList()`.
        pub const VOICE_INIT_LIST: u32 = 0x0070_ab90;
        /// `FmodVoice::initTempData(event system, system)`: empty on PC.
        pub const VOICE_INIT_TEMP_DATA: u32 = 0x0070_ace0;
        /// `FmodVoice::read(id, event system)`.
        pub const VOICE_READ: u32 = 0x0070_add0;
        /// `FmodVoice::releaseData()`.
        pub const VOICE_RELEASE_DATA: u32 = 0x0070_b0b0;
        /// `FmodVoice::setEntireVolume(f32)`.
        pub const VOICE_SET_ENTIRE_VOLUME: u32 = 0x0070_b2b0;
        /// `FmodVoice::setMute(id)`.
        pub const VOICE_SET_MUTE: u32 = 0x0070_b330;
        /// `FmodVoice::setPaused(x)`.
        pub const VOICE_SET_PAUSED: u32 = 0x0070_b370;
        /// `FfxFmod_soundInit_setLang` (`FmodVoice`; lang).
        pub const VOICE_SET_LANG: u32 = 0x0070_b3f0;
        /// `FmodVoice::shoutStart(a, b, c, system)`.
        pub const VOICE_SHOUT_START: u32 = 0x0070_b430;
        /// `FmodVoice::soundStageFlush(x)`.
        pub const VOICE_SOUND_STAGE_FLUSH: u32 = 0x0070_b570;
        /// `FmodVoice::start(pan, volume)`.
        pub const VOICE_START: u32 = 0x0070_b5c0;
        /// `FmodVoice::stop()`.
        pub const VOICE_STOP: u32 = 0x0070_b6b0;
        /// `FmodVoice::volumeDirect(f32 volume, b)`.
        pub const VOICE_VOLUME_DIRECT: u32 = 0x0070_b7f0;
        /// `FmodVoice::volumeTranslate(frames, volume)`.
        pub const VOICE_VOLUME_TRANSLATE: u32 = 0x0070_b880;
        /// The shout player's `start(a, b, c, d, mute, &status0, &status1,
        /// arg)`.
        pub const SHOUT_START: u32 = 0x0070_e480;
        /// The shout player's `setVolume(f32)`.
        pub const SHOUT_SET_VOLUME: u32 = 0x0070_e400;
        /// `FmodSfx::canStartNewEvent(id)` (answers in `al`).
        pub const SFX_CAN_START_NEW_EVENT: u32 = 0x0070_c7d0;
        /// `FmodSfx_createSfxEvent(id)`.
        pub const SFX_CREATE_EVENT: u32 = 0x0070_c8f0;
        /// `FmodSfx_delayStart(a, b, c, d)`.
        pub const SFX_DELAY_START: u32 = 0x0070_cc30;
        /// `FmodSfx::forceStart(a, b, c)`.
        pub const SFX_FORCE_START: u32 = 0x0070_ccd0;
        /// `FmodSfx::isPlaying(id)`.
        pub const SFX_IS_PLAYING: u32 = 0x0070_ce90;
        /// `FmodSfx::loadProject(event system, id, flag)`.
        pub const SFX_LOAD_PROJECT: u32 = 0x0070_cf10;
        /// `FmodSfx::pause(x)`.
        pub const SFX_PAUSE: u32 = 0x0070_d0c0;
        /// `FmodSfx_playBattleStreaming(event system, a, b, c)`.
        pub const SFX_PLAY_BATTLE_STREAMING: u32 = 0x0070_d190;
        /// `FmodSfx::setEntireVolume(f32)`.
        pub const SFX_SET_ENTIRE_VOLUME: u32 = 0x0070_d500;
        /// `FmodSfx::setPan(id, pan)`.
        pub const SFX_SET_PAN: u32 = 0x0070_d640;
        /// `FmodSfx::setPan(id, pan, frames)`.
        pub const SFX_SET_PAN_TRANSLATE: u32 = 0x0070_d690;
        /// `FmodSfx::setVolume(id, volume)`.
        pub const SFX_SET_VOLUME: u32 = 0x0070_d6e0;
        /// `FmodSfx::setVolume(id, volume, frames)`.
        pub const SFX_SET_VOLUME_TRANSLATE: u32 = 0x0070_d730;
        /// `FmodSfx_Start(id, b, c)`.
        pub const SFX_START: u32 = 0x0070_d890;
        /// `FmodSfx::stop(id)`.
        pub const SFX_STOP: u32 = 0x0070_d8f0;
        /// `FmodSfx::stopAllNonBlockingSound(a, b)`.
        pub const SFX_STOP_ALL_NON_BLOCKING: u32 = 0x0070_d930;
        /// `FmodSfx::stopAllNonloopSound()`.
        pub const SFX_STOP_ALL_NONLOOP: u32 = 0x0070_d9e0;
        /// `FmodSfx::stopNonBlockingSound(id)`.
        pub const SFX_STOP_NON_BLOCKING: u32 = 0x0070_db90;
        /// `FmodSfx::unloadProjectBySeq(id)`.
        pub const SFX_UNLOAD_BY_SEQ: u32 = 0x0070_dbb0;
        /// `FmodSfx::unloadProjects(a, b)`.
        pub const SFX_UNLOAD_PROJECTS: u32 = 0x0070_dd10;
        /// A sound event's `isPlaying()` (answers in `al`).
        pub const EVENT_IS_PLAYING: u32 = 0x0070_ec40;
        /// A sound event's `delayStart(d, b, c)`.
        pub const EVENT_DELAY_START: u32 = 0x0070_eac0;
        /// A sound event's `start(b, c)`.
        pub const EVENT_START: u32 = 0x0070_f310;
        /// A sound event's `setPan(pan)`.
        pub const EVENT_SET_PAN: u32 = 0x0070_ef80;
        /// A sound event's `setPan(pan, frames)`.
        pub const EVENT_SET_PAN_TRANSLATE: u32 = 0x0070_f000;
        /// A sound event's `setVolume(volume)`.
        pub const EVENT_SET_VOLUME: u32 = 0x0070_f1a0;
        /// A sound event's `setVolume(volume, frames)`.
        pub const EVENT_SET_VOLUME_TRANSLATE: u32 = 0x0070_f210;
        /// A sound event's `stop(immediate)`.
        pub const EVENT_STOP: u32 = 0x0070_f490;
        /// `FfxFmod_initEventSystem()` (left original: creates FMOD).
        pub const FFX_INIT_EVENT_SYSTEM: u32 = 0x0070_6820;
        /// `FfxFmod::shutDown()` (left original: releases FMOD).
        pub const FFX_SHUT_DOWN: u32 = 0x0070_6ff0;
        /// `FfxFmod::soundLoop()` (left original: the 64-bit clock).
        pub const FFX_SOUND_LOOP: u32 = 0x0070_70d0;
        /// `LocalizationManager_GetInstance()` (cdecl).
        pub const LOCALIZATION: u32 = 0x006d_b070;
        /// `ERRCHECK(result, file, line)` (cdecl).
        pub const ERRCHECK: u32 = 0x0070_79d0;
        /// `printToOutput(prefix, format, ...)` (cdecl).
        pub const PRINT: u32 = 0x0070_7b20;
    }

    /// `FfxFmod.cpp`'s file name for `ERRCHECK`.
    pub const FFX_FMOD_CPP: u32 = 0x00b4_d268;
    /// `FmodMusic.cpp`'s file name.
    pub const FMOD_MUSIC_CPP: u32 = 0x00b4_f4a0;
    /// `FmodVoice.cpp`'s file name.
    pub const FMOD_VOICE_CPP: u32 = 0x00b4_f8c0;
    /// `printToOutput`'s prefix.
    pub const PRINT_PREFIX: u32 = 0x00b4_c30c;
    /// `FmodSfx`'s "can't create" format (start, forceStart).
    pub const SFX_CREATE_FAILED: u32 = 0x00b4_fb80;
    /// `FmodSfx`'s "can't create" format (delayStart).
    pub const SFX_DELAY_CREATE_FAILED: u32 = 0x00b4_fbe0;

    use callee as c;

    fn this_call(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    fn field(h: &impl Mem, obj: u32, off: u32) -> u32 {
        h.read(obj.wrapping_add(off))
    }

    fn float(h: &impl Mem, obj: u32, off: u32) -> f32 {
        h.read(obj.wrapping_add(off))
    }

    /// A method of `fmod`'s object at `obj` (`MUSIC`, `VOICE`, `SFX`).
    fn on(h: &mut impl Host, fmod: u32, obj: u32, sym: u32, args: &[u32]) -> u32 {
        let o = field(h, fmod, obj);
        this_call(h, sym, o, args)
    }

    fn errcheck(h: &mut impl Host, r: u32, file: u32, line: u32) {
        let file = h.rebase(file);
        h.call_sym(c::ERRCHECK, Conv::Cdecl, &[r, file, line]);
    }

    /// `FMOD::Event::getState(event, &state)`, the state's slot first
    /// holding `init` (what the original's stack slot held, kept when FMOD
    /// fails); returns (result, state).
    fn get_state(h: &mut impl Host, event: u32, init: u32) -> (u32, u32) {
        let slot = h.scratch();
        h.write(slot, init);
        let r = h.call_sym(EVENT_GET_STATE, Conv::Stdcall, &[event, slot]);
        (r, h.read(slot))
    }

    /// `x` cut to 0..1 as the setters do (NaN stays).
    #[must_use]
    pub fn clamp01(x: f32) -> f32 {
        x.clamp(0.0, 1.0)
    }

    /// The low byte of `eax`, where a callee answers a `bool`.
    fn al(eax: u32) -> u8 {
        eax.to_le_bytes()[0]
    }

    /// `f32(a * b)` bits, as `fmul dword` then `fstp dword`.
    fn product(a: f32, b: f32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let p = (f64::from(a) * f64::from(b)) as f32;
        p.to_bits()
    }

    /// An f32 passed through the FPU (`fld`, `fstp`), as bits.
    fn through_fpu(bits: u32) -> u32 {
        x87::round_trip(f32::from_bits(bits)).to_bits()
    }

    // ---- FfxFmod (thiscall, `this` the FfxFmod)

    /// `FfxFmod::getNowPlayMusic()` (0x007067d0).
    pub fn get_now_play_music(h: &impl Mem, fmod: u32) -> u32 {
        field(h, field(h, fmod, MUSIC), MUSIC_START_ID)
    }

    /// `FfxFmod::initLoadMasterBank()` (0x00706920): the two `initTempData`
    /// (empty on PC). Leaves `eax` as they do: nothing reads it.
    pub fn init_load_master_bank(h: &mut impl Host, fmod: u32) {
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, MUSIC, c::MUSIC_INIT_TEMP_DATA, &[es]);
        let sys = field(h, fmod, SYSTEM);
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, VOICE, c::VOICE_INIT_TEMP_DATA, &[es, sys]);
    }

    /// `FfxFmod::musicClear(id)` (0x007069e0).
    pub fn music_clear(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_CLEAR, &[id])
    }

    /// `FfxFmod::musicClearAll()` (0x007069f0).
    pub fn music_clear_all(h: &mut impl Host, fmod: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_CLEAR_ALL, &[])
    }

    /// `FfxFmod_musicVolumeFadeOut(id, frames)` (0x00706a00): by the PC
    /// order and command 0x17 (`iMusicCrossPlay`), `FfxFmod::musicCrossPlay`.
    pub fn music_cross_play(h: &mut impl Host, fmod: u32, id: u32, frames: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, MUSIC, c::MUSIC_CROSS_PLAY, &[id, frames, es])
    }

    /// `FfxFmod::musicDataSend(id)` (0x00706a20).
    pub fn music_data_send(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, MUSIC, c::MUSIC_DATA_SEND, &[id, es])
    }

    /// `FfxFmod::musicMultiPlay(id)` (0x00706a40), also
    /// `FfxFmod::musicPhrasePlay(id, phrase)` (0x00706a80): a plain start
    /// of a valid id on PC; 0.
    pub fn music_multi_play(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        if id <= MUSIC_LAST {
            on(h, fmod, MUSIC, c::MUSIC_START, &[id]);
        }
        0
    }

    /// `FfxFmod::musicPauseAll(flag)` (0x00706a70).
    pub fn music_pause_all(h: &mut impl Host, fmod: u32, flag: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_PAUSE_ALL, &[flag])
    }

    /// `FfxFmod::musicPlayState(id)` (0x00706ad0).
    pub fn music_play_state(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_PLAY_STATE, &[id])
    }

    /// `FfxFmod_musicRead(id)` (0x00706ae0).
    pub fn music_read(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, MUSIC, c::MUSIC_READ, &[id, es])
    }

    fn record(h: &impl Mem, music: u32, id: u32) -> u32 {
        field(h, music, MUSIC_RECORDS).wrapping_add(id.wrapping_mul(MUSIC_RECORD))
    }

    /// `FfxFmod::musicReplayPointClear(id)` (0x00706b00): a valid id's
    /// replay flag 0; 0.
    pub fn music_replay_point_clear(h: &mut impl Mem, fmod: u32, id: u32) -> u32 {
        let music = field(h, fmod, MUSIC);
        if id <= MUSIC_LAST {
            let r = record(h, music, id);
            h.write(r.wrapping_add(4), 0u32);
        }
        0
    }

    /// `FfxFmod::musicReplayPointSet()` (0x00706b30): the playing music's
    /// replay flag 1 (no range check); 0.
    pub fn music_replay_point_set(h: &mut impl Mem, fmod: u32) -> u32 {
        let music = field(h, fmod, MUSIC);
        let id = field(h, music, MUSIC_START_ID);
        let r = record(h, music, id);
        h.write(r.wrapping_add(4), 1u32);
        0
    }

    /// `FfxFmod::musicStart(id)` (0x00706b90).
    pub fn music_start(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_START, &[id])
    }

    /// `FfxFmod::musicStop(id)` (0x00706ba0).
    pub fn music_stop(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_STOP, &[id])
    }

    /// `FfxFmod::musicStopAll()` (0x00706bb0).
    pub fn music_stop_all(h: &mut impl Host, fmod: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_STOP_ALL, &[])
    }

    /// `FfxFmod::musicSwitchBGM(type)` (0x00706bc0): types 0 and 1 go to
    /// the music; any other type is returned as is.
    pub fn music_switch_bgm(h: &mut impl Host, fmod: u32, t: u32) -> u32 {
        if t > 1 {
            return t;
        }
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, MUSIC, c::MUSIC_SWITCH_BGM, &[t, es])
    }

    /// `FfxFmod::musicSyncStandby()` (0x00706bf0).
    pub fn music_sync_standby(h: &mut impl Host, fmod: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_SYNC_STANDBY, &[])
    }

    /// `FfxFmod::musicVolumeDirect(id, f32 volume, c)` (0x00706c20).
    pub fn music_volume_direct(h: &mut impl Host, fmod: u32, id: u32, volume: u32, x: u32) -> u32 {
        let v = through_fpu(volume);
        on(h, fmod, MUSIC, c::MUSIC_VOLUME_DIRECT, &[id, v, x])
    }

    /// `FfxFmod::musicVolumeFadeOut(id, frames)` (0x00706c40).
    pub fn music_volume_fade_out(h: &mut impl Host, fmod: u32, id: u32, frames: u32) -> u32 {
        on(h, fmod, MUSIC, c::MUSIC_VOLUME_FADE_OUT, &[id, frames])
    }

    /// `FfxFmod::musicVolumeTranslate(id, frames, volume)` (0x00706c50).
    pub fn music_volume_translate(
        h: &mut impl Host,
        fmod: u32,
        id: u32,
        frames: u32,
        volume: u32,
    ) -> u32 {
        on(
            h,
            fmod,
            MUSIC,
            c::MUSIC_VOLUME_TRANSLATE,
            &[id, frames, volume],
        )
    }

    /// An `FfxFmod::sfx*` forwarder (0x00706db0..0x00706fb0): the sfx
    /// method `sym` with `args`; 0.
    pub fn sfx_call(h: &mut impl Host, fmod: u32, sym: u32, args: &[u32]) -> u32 {
        on(h, fmod, SFX, sym, args);
        0
    }

    /// `FfxFmod::sfxIsPlaying(id)` (0x00706df0): the sfx's `al`.
    pub fn sfx_is_playing(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        u32::from(al(on(h, fmod, SFX, c::SFX_IS_PLAYING, &[id])))
    }

    /// `FfxFmod::sfxIspaused()` (0x00706e10): the sfx's paused byte.
    pub fn sfx_is_paused(h: &impl Mem, fmod: u32) -> u32 {
        u32::from(h.read::<u8>(field(h, fmod, SFX).wrapping_add(SFX_PAUSED)))
    }

    /// `FfxFmod_sfxLoad(id, flag)` (0x00706e20); 0.
    pub fn sfx_load(h: &mut impl Host, fmod: u32, id: u32, flag: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        sfx_call(
            h,
            fmod,
            c::SFX_LOAD_PROJECT,
            &[es, id, u32::from(flag != 0)],
        )
    }

    /// `FfxFmod_sfxPlayBattleStreaming(a, b, c)` (0x00706e70); 0.
    pub fn sfx_play_battle_streaming(h: &mut impl Host, fmod: u32, a: u32, b: u32, x: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        sfx_call(h, fmod, c::SFX_PLAY_BATTLE_STREAMING, &[es, a, b, x])
    }

    /// `FfxFmod::shoutStart(a, b, c)` (0x00706fd0): the voice's shout with
    /// the low-level `System`.
    pub fn shout_start(h: &mut impl Host, fmod: u32, a: u32, b: u32, x: u32) -> u32 {
        let sys = field(h, fmod, SYSTEM);
        on(h, fmod, VOICE, c::VOICE_SHOUT_START, &[a, b, x, sys])
    }

    /// `FfxFmod_soundInit()` (0x00707080): the language from the
    /// localization manager to the voice, the sfx's English byte (1 and
    /// 2), then the music and voice data; 0.
    pub fn sound_init(h: &mut impl Host, fmod: u32) -> u32 {
        let loc = h.call_sym(c::LOCALIZATION, Conv::Cdecl, &[]);
        let lang: u32 = h.read(loc);
        on(h, fmod, VOICE, c::VOICE_SET_LANG, &[lang]);
        let sfx = field(h, fmod, SFX);
        h.write(
            sfx.wrapping_add(SFX_ENGLISH),
            u8::from(lang == 1 || lang == 2),
        );
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, MUSIC, c::MUSIC_INIT_DATA, &[es]);
        on(h, fmod, VOICE, c::VOICE_INIT_LIST, &[]);
        let sys = field(h, fmod, SYSTEM);
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, VOICE, c::VOICE_INIT_DATA, &[es, sys]);
        0
    }

    /// `FfxFmod::soundPlayStatus()` (0x007071c0), also
    /// `FfxFmod::voicePlayStatus()` (0x007074e0): the voice's status words
    /// or'ed.
    pub fn sound_play_status(h: &impl Mem, fmod: u32) -> u32 {
        let v = field(h, fmod, VOICE);
        field(h, v, VOICE_STATUS[1]) | field(h, v, VOICE_STATUS[0])
    }

    /// The volume setters (`FfxFmod::soundSetMasterVolume` 0x007071e0,
    /// `soundSetMusicVolume` 0x00707280, `soundSetSfxVolume` 0x00707330,
    /// `soundSetVoiceVolume` 0x007073a0; `which` the volume's offset, `x`
    /// f32 bits): `x` cut to 0..1; when it differs from the kept one, it
    /// is kept and the objects get their volume times the master (all
    /// three for the master); 0.
    pub fn sound_set_volume(h: &mut impl Host, fmod: u32, which: u32, x: u32) -> u32 {
        let x = clamp01(f32::from_bits(through_fpu(x)));
        #[allow(clippy::float_cmp)] // the original compares exactly
        let same = float(h, fmod, which) == x;
        if same {
            return 0;
        }
        h.write(fmod.wrapping_add(which), x);
        match which {
            MASTER_VOLUME => {
                let p = product(x, float(h, fmod, MUSIC_VOLUME));
                on(h, fmod, MUSIC, c::MUSIC_SET_ENTIRE_VOLUME, &[p]);
                let p = product(float(h, fmod, VOICE_VOLUME), float(h, fmod, MASTER_VOLUME));
                on(h, fmod, VOICE, c::VOICE_SET_ENTIRE_VOLUME, &[p]);
                let p = product(float(h, fmod, SFX_VOLUME), float(h, fmod, MASTER_VOLUME));
                on(h, fmod, SFX, c::SFX_SET_ENTIRE_VOLUME, &[p]);
            }
            MUSIC_VOLUME => {
                let p = product(x, float(h, fmod, MASTER_VOLUME));
                on(h, fmod, MUSIC, c::MUSIC_SET_ENTIRE_VOLUME, &[p]);
            }
            VOICE_VOLUME => {
                let p = product(x, float(h, fmod, MASTER_VOLUME));
                on(h, fmod, VOICE, c::VOICE_SET_ENTIRE_VOLUME, &[p]);
            }
            _ => {
                let p = product(x, float(h, fmod, MASTER_VOLUME));
                on(h, fmod, SFX, c::SFX_SET_ENTIRE_VOLUME, &[p]);
            }
        }
        0
    }

    /// `FfxFmod::soundSetMute(id, kind)` (0x007072f0): kind 1 the music's,
    /// 2 the voice's; 0.
    pub fn sound_set_mute(h: &mut impl Host, fmod: u32, id: u32, kind: u32) -> u32 {
        match kind {
            1 => {
                on(h, fmod, MUSIC, c::MUSIC_SET_MUTE, &[id]);
            }
            2 => {
                on(h, fmod, VOICE, c::VOICE_SET_MUTE, &[id]);
            }
            _ => {}
        }
        0
    }

    /// `FfxFmod::soundStageFlush(x)` (0x00707410).
    pub fn sound_stage_flush(h: &mut impl Host, fmod: u32, x: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_SOUND_STAGE_FLUSH, &[x])
    }

    /// `FfxFmod::soundUpdate()` (0x00707420): `EventSystem::update`,
    /// checked; 0.
    pub fn sound_update(h: &mut impl Host, fmod: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        if es != 0 {
            let r = h.call_sym(EVENT_SYSTEM_UPDATE, Conv::Stdcall, &[es]);
            errcheck(h, r, FFX_FMOD_CPP, 0x121);
        }
        0
    }

    /// `FfxFmod::voiceClear()` (0x00707470).
    pub fn voice_clear(h: &mut impl Host, fmod: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_CLEAR, &[])
    }

    /// `FfxFmod::voiceDataChange(id)` (0x00707480): when the voice takes
    /// it, the music keeps the voice's first word; 0.
    pub fn voice_data_change(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        if on(h, fmod, VOICE, c::VOICE_DATA_CHANGE, &[id, es]) != 0 {
            let w = field(h, field(h, fmod, VOICE), 0);
            let music = field(h, fmod, MUSIC);
            h.write(music.wrapping_add(MUSIC_VOICE), w);
        }
        0
    }

    /// `FfxFmod::voiceIsPaused()` (0x007074b0).
    pub fn voice_is_paused(h: &impl Mem, fmod: u32) -> u32 {
        field(h, field(h, fmod, VOICE), VOICE_PAUSED)
    }

    /// `FfxFmod_voiceRead(id)` (0x007074f0).
    pub fn voice_read(h: &mut impl Host, fmod: u32, id: u32) -> u32 {
        let es = field(h, fmod, EVENT_SYSTEM);
        on(h, fmod, VOICE, c::VOICE_READ, &[id, es])
    }

    /// `FfxFmod::voiceReleaseData()` (0x00707510).
    pub fn voice_release_data(h: &mut impl Host, fmod: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_RELEASE_DATA, &[])
    }

    /// `FfxFmod::voiceSetPaused(x)` (0x00707520).
    pub fn voice_set_paused(h: &mut impl Host, fmod: u32, x: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_SET_PAUSED, &[x])
    }

    /// `FfxFmod::voiceSetStartSignal(x)` (0x00707530): the voice's `+0x44`;
    /// returns `x`.
    pub fn voice_set_start_signal(h: &mut impl Mem, fmod: u32, x: u32) -> u32 {
        let v = field(h, fmod, VOICE);
        h.write(v.wrapping_add(VOICE_STATUS[1]), x);
        x
    }

    /// `FfxFmod::voiceStart(pan, volume)` (0x00707540).
    pub fn voice_start(h: &mut impl Host, fmod: u32, a: u32, b: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_START, &[a, b])
    }

    /// `FfxFmod::voiceStop()` (0x00707550).
    pub fn voice_stop(h: &mut impl Host, fmod: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_STOP, &[])
    }

    /// `FfxFmod::voiceVolumeDirect(f32 volume, b)` (0x00707560).
    pub fn voice_volume_direct(h: &mut impl Host, fmod: u32, volume: u32, b: u32) -> u32 {
        let v = through_fpu(volume);
        on(h, fmod, VOICE, c::VOICE_VOLUME_DIRECT, &[v, b])
    }

    /// `FfxFmod::voiceVolumeTranslate(frames, volume)` (0x00707580).
    pub fn voice_volume_translate(h: &mut impl Host, fmod: u32, a: u32, b: u32) -> u32 {
        on(h, fmod, VOICE, c::VOICE_VOLUME_TRANSLATE, &[a, b])
    }

    // ---- the command handlers (cdecl `fmod, args*`)

    /// The two "translate" handlers' frame count: `ftol(ceil(x / 2) +
    /// 0.5)`, each step stored as f32 as the original does.
    pub fn half_up(h: &impl Mem, x: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // fild, fstp dword
        let f = x.cast_signed() as f32;
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let half = (f64::from(f) * 0.5) as f32;
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let up = f64::from(half).ceil() as f32;
        x87::ftol(h, f64::from(up) + 0.5)
    }

    /// An `i32` as f32 bits (`fild`, `fstp dword`).
    #[must_use]
    pub fn to_f32(x: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // fild, fstp dword
        let f = x.cast_signed() as f32;
        f.to_bits()
    }

    /// The volume commands' percentage as f32 bits: `f32(f32(x) / 100)`.
    #[must_use]
    pub fn percent(x: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // fild, fstp dword
        let f = x.cast_signed() as f32;
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let p = (f64::from(f) / 100.0) as f32;
        p.to_bits()
    }

    /// Runs command `cmd`'s handler (the table at `0x00c3a368`) with the
    /// argument block at `args` on `FfxFmod` `fmod`: its result, `None`
    /// for commands past 0x45. The empty PC methods (voice pan, music play
    /// fast, tempo, pitch, mute set) leave their handlers returning 0.
    #[allow(clippy::too_many_lines)] // one arm per command
    pub fn handle(h: &mut impl Host, cmd: u32, fmod: u32, args: u32) -> Option<u32> {
        let a: [u32; 4] = [
            field(h, args, 0),
            field(h, args, 4),
            field(h, args, 8),
            field(h, args, 0xc),
        ];
        // the handlers read only the words they pass
        let r = match cmd {
            0x00 => sound_set_mute(h, fmod, a[0], a[1]),
            0x01 => sound_init(h, fmod),
            0x02 => sound_update(h, fmod),
            0x03 => {
                this_call(h, c::FFX_SOUND_LOOP, fmod, &[]);
                0
            }
            0x04 => {
                sound_stage_flush(h, fmod, a[0]);
                0
            }
            0x05 | 0x11 => sound_play_status(h, fmod),
            0x06 => {
                shout_start(h, fmod, a[0], a[1], a[2]);
                0
            }
            0x07 => {
                voice_release_data(h, fmod);
                0
            }
            0x08 => {
                voice_set_paused(h, fmod, a[0]);
                0
            }
            0x09 => voice_data_change(h, fmod, a[0]),
            0x0a => {
                voice_stop(h, fmod);
                0
            }
            0x0d => {
                let frames = half_up(h, a[0]);
                voice_volume_translate(h, fmod, frames, a[1]);
                0
            }
            0x0e => {
                voice_volume_direct(h, fmod, to_f32(a[0]), a[1]);
                0
            }
            0x0f => {
                voice_start(h, fmod, a[0], a[1]);
                0
            }
            0x10 => {
                voice_read(h, fmod, a[0]);
                0
            }
            0x12 => voice_is_paused(h, fmod),
            0x13 => get_now_play_music(h, fmod),
            0x14 => {
                music_sync_standby(h, fmod);
                0
            }
            0x15 => {
                music_data_send(h, fmod, a[0]);
                0
            }
            0x16 => music_play_state(h, fmod, a[0]),
            0x17 => {
                music_cross_play(h, fmod, a[0], a[1]);
                0
            }
            0x19 => music_replay_point_set(h, fmod),
            0x1a => music_replay_point_clear(h, fmod, a[0]),
            0x1b | 0x21 => music_multi_play(h, fmod, a[0]),
            0x1c => {
                music_volume_fade_out(h, fmod, a[0], a[1]);
                0
            }
            0x22 => {
                let frames = half_up(h, a[1]);
                music_volume_translate(h, fmod, a[0], frames, a[2]);
                0
            }
            0x23 => {
                music_volume_direct(h, fmod, a[0], to_f32(a[1]), a[2]);
                0
            }
            0x25 => {
                music_clear(h, fmod, a[0]);
                0
            }
            0x26 => {
                music_stop(h, fmod, a[0]);
                0
            }
            0x27 => {
                music_start(h, fmod, a[0]);
                0
            }
            0x28 => {
                music_read(h, fmod, a[0]);
                0
            }
            0x29 => {
                this_call(h, c::FFX_SHUT_DOWN, fmod, &[]);
                0
            }
            0x2a => {
                this_call(h, c::FFX_INIT_EVENT_SYSTEM, fmod, &[]);
                0
            }
            0x2b => {
                init_load_master_bank(h, fmod);
                0
            }
            0x2c => {
                music_stop_all(h, fmod);
                0
            }
            0x2d => {
                music_clear_all(h, fmod);
                0
            }
            0x2e => {
                music_switch_bgm(h, fmod, a[0]);
                0
            }
            0x2f => {
                music_pause_all(h, fmod, a[0]);
                0
            }
            0x30 => sfx_load(h, fmod, a[0], a[1]),
            0x31 => sfx_call(h, fmod, c::SFX_UNLOAD_PROJECTS, &[0, 0]),
            0x32 => sfx_call(h, fmod, c::SFX_UNLOAD_BY_SEQ, &[a[0]]),
            0x33 => sfx_call(h, fmod, c::SFX_START, &a[..3]),
            0x34 => sfx_call(h, fmod, c::SFX_FORCE_START, &a[..3]),
            0x35 => sfx_call(h, fmod, c::SFX_DELAY_START, &a),
            0x36 => sfx_call(h, fmod, c::SFX_STOP, &[a[0]]),
            // the handler widens a1:a0 to 64 bits; the method takes a0
            0x37 => sfx_call(h, fmod, c::SFX_STOP_ALL_NON_BLOCKING, &[a[0], 0]),
            0x38 => sfx_call(h, fmod, c::SFX_STOP_NON_BLOCKING, &[a[0]]),
            0x39 => sfx_call(h, fmod, c::SFX_STOP_ALL_NONLOOP, &[]),
            0x3a => sfx_call(h, fmod, c::SFX_PAUSE, &[u32::from(a[0] != 0)]),
            0x3b => sfx_call(h, fmod, c::SFX_SET_VOLUME, &a[..2]),
            0x3c => sfx_call(h, fmod, c::SFX_SET_VOLUME_TRANSLATE, &a[..3]),
            0x3d => sfx_call(h, fmod, c::SFX_SET_PAN, &a[..2]),
            0x3e => sfx_call(h, fmod, c::SFX_SET_PAN_TRANSLATE, &a[..3]),
            0x3f => sfx_is_playing(h, fmod, a[0]),
            0x40 => sfx_play_battle_streaming(h, fmod, a[0], a[1], a[2]),
            0x41 => sfx_is_paused(h, fmod),
            0x42 => sound_set_volume(h, fmod, MASTER_VOLUME, percent(a[0])),
            0x43 => sound_set_volume(h, fmod, MUSIC_VOLUME, percent(a[0])),
            0x44 => sound_set_volume(h, fmod, VOICE_VOLUME, percent(a[0])),
            0x45 => sound_set_volume(h, fmod, SFX_VOLUME, percent(a[0])),
            // the empty methods (0x0b still works out a frame count)
            0x0b | 0x0c | 0x18 | 0x1d..=0x20 | 0x24 => 0,
            _ => return None,
        };
        Some(r)
    }

    /// The handlers' addresses by command (the table at `0x00c3a368`).
    pub const HANDLERS: [u32; 0x46] = [
        0x0070_7b90,
        0x0070_7bb0,
        0x0070_7bc0,
        0x0070_7bd0,
        0x0070_7be0,
        0x0070_7c00,
        0x0070_7c10,
        0x0070_7c30,
        0x0070_7c40,
        0x0070_7c60,
        0x0070_7c80,
        0x0070_7c90,
        0x0070_7cf0,
        0x0070_7d10,
        0x0070_7d70,
        0x0070_7da0,
        0x0070_7dc0,
        0x0070_7de0,
        0x0070_7df0,
        0x0070_7e00,
        0x0070_7e10,
        0x0070_7e20,
        0x0070_7e40,
        0x0070_7e60,
        0x0070_7e80,
        0x0070_7ea0,
        0x0070_7eb0,
        0x0070_7ed0,
        0x0070_7ef0,
        0x0070_7f10,
        0x0070_7f70,
        0x0070_7f90,
        0x0070_7fb0,
        0x0070_7fd0,
        0x0070_7ff0,
        0x0070_8050,
        0x0070_8080,
        0x0070_80a0,
        0x0070_80c0,
        0x0070_80e0,
        0x0070_8100,
        0x0070_8120,
        0x0070_8130,
        0x0070_8140,
        0x0070_8150,
        0x0070_8160,
        0x0070_8170,
        0x0070_8190,
        0x0070_81b0,
        0x0070_81d0,
        0x0070_81e0,
        0x0070_8200,
        0x0070_8220,
        0x0070_8240,
        0x0070_8260,
        0x0070_8280,
        0x0070_82b0,
        0x0070_82f0,
        0x0070_82d0,
        0x0070_8300,
        0x0070_8320,
        0x0070_8340,
        0x0070_8360,
        0x0070_8380,
        0x0070_83a0,
        0x0070_83c0,
        0x0070_83d0,
        0x0070_8400,
        0x0070_8430,
        0x0070_8460,
    ];

    // ---- FmodMusic (thiscall, `this` the music)

    /// `FmodMusic::playState(id)` (0x00709060): the playing music's state
    /// or'ed with the start signal (`id` unused).
    pub fn music_object_play_state(h: &impl Mem, music: u32) -> u32 {
        let r = record(h, music, field(h, music, MUSIC_START_ID));
        field(h, r, 0x20) | field(h, music, MUSIC_START_SIGNAL)
    }

    /// `FmodMusic::syncStandby()` (0x00709d80): 1 when music 0x10's event
    /// answers state 2, else 0.
    pub fn music_object_sync_standby(h: &mut impl Host, music: u32) -> u32 {
        let event = field(h, record(h, music, 0x10), 0);
        if event == 0 {
            return 0;
        }
        // the state's slot first holds `this` (the frame's `push ecx`)
        let (r, state) = get_state(h, event, music);
        errcheck(h, r, FMOD_MUSIC_CPP, 0xdd);
        u32::from(r == 0 && state == 2)
    }

    /// `FmodMusic::volumeFadeOut(id, frames)` (0x00709eb0): a playing
    /// music is marked fading and translated to volume 0; 0.
    pub fn music_object_volume_fade_out(
        h: &mut impl Host,
        music: u32,
        id: u32,
        frames: u32,
    ) -> u32 {
        if id > MUSIC_LAST {
            return 0;
        }
        let event = field(h, record(h, music, id), 0);
        if event == 0 {
            return 0;
        }
        // the state goes to the argument's slot, which holds the id
        let (r, state) = get_state(h, event, id);
        errcheck(h, r, FMOD_MUSIC_CPP, 0x299);
        if state & STATE_PLAYING != 0 {
            let rec = record(h, music, id);
            h.write(rec.wrapping_add(8), 1u32);
            this_call(h, c::MUSIC_VOLUME_TRANSLATE, music, &[id, frames, 0]);
        }
        0
    }

    // ---- FmodVoice (thiscall, `this` the voice)

    /// `FmodVoice::setEntireVolume(f32 x)` (0x0070b2b0): `x` cut to 0..1;
    /// when it changes, the shout player and the voice volume follow:
    /// `volumeDirect`'s result. `None` when unchanged (`eax` then holds the
    /// FPU status; nothing reads it).
    pub fn voice_set_entire_volume(h: &mut impl Host, voice: u32, x: u32) -> Option<u32> {
        let x = clamp01(f32::from_bits(through_fpu(x)));
        #[allow(clippy::float_cmp)] // the original compares exactly
        let same = float(h, voice, VOICE_ENTIRE_VOLUME) == x;
        if same {
            return None;
        }
        h.write(voice.wrapping_add(VOICE_ENTIRE_VOLUME), x);
        let shout = field(h, voice, VOICE_SHOUT);
        this_call(h, c::SHOUT_SET_VOLUME, shout, &[x.to_bits()]);
        let level = through_fpu(field(h, voice, VOICE_VOLUME_LEVEL));
        Some(this_call(h, c::VOICE_VOLUME_DIRECT, voice, &[level, 0]))
    }

    /// `FmodVoice::setMute(id)` (0x0070b330): keeps `id`; the volume goes
    /// to 0 for id 1, else back to the voice volume: `volumeDirect`'s
    /// result.
    pub fn voice_set_mute(h: &mut impl Host, voice: u32, id: u32) -> u32 {
        h.write(voice.wrapping_add(VOICE_MUTE), id);
        let v = if id == 1 {
            0
        } else {
            through_fpu(field(h, voice, VOICE_VOLUME_LEVEL))
        };
        this_call(h, c::VOICE_VOLUME_DIRECT, voice, &[v, 0])
    }

    /// `FmodVoice::setPaused(x)` (0x0070b370): with an event and a change,
    /// a playing event is paused (x 1) or resumed (else); `x` is kept.
    /// Without an event, nothing. 0.
    pub fn voice_object_set_paused(h: &mut impl Host, voice: u32, x: u32) -> u32 {
        let event = field(h, voice, VOICE_EVENT);
        if event == 0 {
            return 0;
        }
        if field(h, voice, VOICE_PAUSED) != x {
            let (r, state) = get_state(h, event, x);
            errcheck(h, r, FMOD_VOICE_CPP, 0x276);
            if state & STATE_PLAYING != 0 {
                let (paused, line) = if x == 1 { (1, 0x27d) } else { (0, 0x282) };
                let event = field(h, voice, VOICE_EVENT);
                let r = h.call_sym(EVENT_SET_PAUSED, Conv::Stdcall, &[event, paused]);
                errcheck(h, r, FMOD_VOICE_CPP, line);
            }
        }
        h.write(voice.wrapping_add(VOICE_PAUSED), x);
        0
    }

    /// `FfxFmod_soundInit_setLang(lang)` (0x0070b3f0, `this` the voice):
    /// keeps the language; English for 1, 2 and anything past 4; returns
    /// `lang`.
    pub fn voice_set_lang(h: &mut impl Mem, voice: u32, lang: u32) -> u32 {
        h.write(voice.wrapping_add(VOICE_LANG), lang);
        let english = !matches!(lang, 0 | 3 | 4);
        h.write(voice.wrapping_add(VOICE_ENGLISH), u8::from(english));
        lang
    }

    /// `FmodVoice::shoutStart(a, b, c, d)` (0x0070b430): the shout player
    /// starts with the mute id, the status words and the shout argument,
    /// then takes the entire volume; 0.
    pub fn voice_shout_start(h: &mut impl Host, voice: u32, args: [u32; 4]) -> u32 {
        let shout = field(h, voice, VOICE_SHOUT);
        let mute = field(h, voice, VOICE_MUTE);
        let arg = field(h, voice, VOICE_SHOUT_ARG);
        let s0 = voice.wrapping_add(VOICE_STATUS[0]);
        let s1 = voice.wrapping_add(VOICE_STATUS[1]);
        let [a, b, x, d] = args;
        this_call(h, c::SHOUT_START, shout, &[a, b, x, d, mute, s0, s1, arg]);
        let shout = field(h, voice, VOICE_SHOUT);
        let v = through_fpu(field(h, voice, VOICE_ENTIRE_VOLUME));
        this_call(h, c::SHOUT_SET_VOLUME, shout, &[v]);
        0
    }

    /// `FmodVoice::soundStageFlush(x)` (0x0070b570): clears the flush
    /// byte, and the two stage globals unless `x` is 0x189..=0x18e; 0.
    pub fn voice_sound_stage_flush(h: &mut impl Mem, voice: u32, x: u32) -> u32 {
        h.write(voice.wrapping_add(VOICE_FLUSH), 0u8);
        if !(0x189..=0x18e).contains(&x) {
            for g in STAGE_FLUSH {
                let at = h.rebase(g);
                h.write(at, 0u32);
            }
        }
        0
    }

    // ---- FmodSfx (thiscall, `this` the sfx)

    /// The event tree's find (0x0070ba70, thiscall `tree; &id`): the node
    /// of `id`, or 0.
    pub fn sfx_find(h: &impl Mem, tree: u32, id: u32) -> u32 {
        let id = id.cast_signed();
        let mut node = field(h, tree, 0xc);
        while node != tree {
            let key = field(h, node, 0xc).cast_signed();
            if key == id {
                return node;
            }
            node = field(h, node, if key > id { 0 } else { 4 });
        }
        0
    }

    /// The event list of sound `id`, or 0.
    fn sfx_list(h: &impl Mem, sfx: u32, id: u32) -> u32 {
        let node = sfx_find(h, sfx.wrapping_add(SFX_EVENTS), id);
        if node == 0 {
            0
        } else {
            node.wrapping_add(0x10)
        }
    }

    /// Event `i` of a list, while `i` is below its (re-read) count.
    fn sfx_event(h: &impl Mem, list: u32, i: u32) -> Option<u32> {
        (i < field(h, list, 0)).then(|| field(h, field(h, list, 8), 4 * i))
    }

    /// Calls `f` (thiscall, `args`) on every event of sound `id`.
    fn sfx_each(h: &mut impl Host, sfx: u32, id: u32, f: u32, args: &[u32]) {
        let list = sfx_list(h, sfx, id);
        if list == 0 {
            return;
        }
        let mut i = 0;
        while let Some(e) = sfx_event(h, list, i) {
            if e != 0 {
                this_call(h, f, e, args);
            }
            i += 1;
        }
    }

    /// `FmodSfx::canStartNewEvent(id)` (0x0070c7d0): 0 while one of the
    /// sound's events plays, else 1; id 1 always may. Only `al` is the
    /// answer.
    pub fn sfx_can_start_new_event(h: &mut impl Host, sfx: u32, id: u32) -> u32 {
        if id == 1 {
            return 1;
        }
        let list = sfx_list(h, sfx, id);
        if list == 0 {
            return 1;
        }
        let mut i = 0;
        while let Some(e) = sfx_event(h, list, i) {
            if e != 0 && al(this_call(h, c::EVENT_IS_PLAYING, e, &[])) != 0 {
                return 0;
            }
            i += 1;
        }
        1
    }

    /// The three starts: unless paused (`delayStart`, `start`), a sound
    /// that may start gets a new event, started with `args`; a failed
    /// event is reported. `Some` with the event start's result, `None`
    /// when nothing started (`eax` is left over then; nothing reads it).
    fn sfx_start_with(
        h: &mut impl Host,
        sfx: u32,
        id: u32,
        check_paused: bool,
        start: (u32, &[u32]),
        message: u32,
    ) -> Option<u32> {
        if check_paused && h.read::<u8>(sfx.wrapping_add(SFX_PAUSED)) != 0 {
            return None;
        }
        if al(this_call(h, c::SFX_CAN_START_NEW_EVENT, sfx, &[id])) == 0 {
            return None;
        }
        let e = this_call(h, c::SFX_CREATE_EVENT, sfx, &[id]);
        if e != 0 {
            return Some(this_call(h, start.0, e, start.1));
        }
        let prefix = h.rebase(PRINT_PREFIX);
        let message = h.rebase(message);
        h.call_sym(c::PRINT, Conv::Cdecl, &[prefix, message, id]);
        None
    }

    /// `FmodSfx_delayStart(id, b, c, delay)` (0x0070cc30).
    pub fn sfx_delay_start(h: &mut impl Host, sfx: u32, args: [u32; 4]) -> Option<u32> {
        let [id, b, x, d] = args;
        sfx_start_with(
            h,
            sfx,
            id,
            true,
            (c::EVENT_DELAY_START, &[d, b, x]),
            SFX_DELAY_CREATE_FAILED,
        )
    }

    /// `FmodSfx::forceStart(id, b, c)` (0x0070ccd0): `start` without the
    /// pause check.
    pub fn sfx_force_start(h: &mut impl Host, sfx: u32, id: u32, b: u32, x: u32) -> Option<u32> {
        sfx_start_with(
            h,
            sfx,
            id,
            false,
            (c::EVENT_START, &[b, x]),
            SFX_CREATE_FAILED,
        )
    }

    /// `FmodSfx_Start(id, b, c)` (0x0070d890).
    pub fn sfx_start(h: &mut impl Host, sfx: u32, id: u32, b: u32, x: u32) -> Option<u32> {
        sfx_start_with(
            h,
            sfx,
            id,
            true,
            (c::EVENT_START, &[b, x]),
            SFX_CREATE_FAILED,
        )
    }

    /// `FmodSfx::setPan(id, pan)` (0x0070d640): every event of the sound.
    pub fn sfx_set_pan(h: &mut impl Host, sfx: u32, id: u32, pan: u32) {
        sfx_each(h, sfx, id, c::EVENT_SET_PAN, &[pan]);
    }

    /// `FmodSfx::setPan(id, pan, frames)` (0x0070d690).
    pub fn sfx_set_pan_translate(h: &mut impl Host, sfx: u32, id: u32, pan: u32, frames: u32) {
        sfx_each(h, sfx, id, c::EVENT_SET_PAN_TRANSLATE, &[pan, frames]);
    }

    /// `FmodSfx::setVolume(id, volume)` (0x0070d6e0).
    pub fn sfx_set_volume(h: &mut impl Host, sfx: u32, id: u32, volume: u32) {
        sfx_each(h, sfx, id, c::EVENT_SET_VOLUME, &[volume]);
    }

    /// `FmodSfx::setVolume(id, volume, frames)` (0x0070d730).
    pub fn sfx_set_volume_translate(
        h: &mut impl Host,
        sfx: u32,
        id: u32,
        volume: u32,
        frames: u32,
    ) {
        sfx_each(h, sfx, id, c::EVENT_SET_VOLUME_TRANSLATE, &[volume, frames]);
    }

    /// `FmodSfx::stop(id)` (0x0070d8f0): every event of the sound stops
    /// (fading).
    pub fn sfx_stop(h: &mut impl Host, sfx: u32, id: u32) {
        sfx_each(h, sfx, id, c::EVENT_STOP, &[0]);
    }

    /// `FmodSfx::stopNonBlockingSound(id)` (0x0070db90): `stop` for ids
    /// past 0x7f (signed); `Some(id)` (its `eax`) for the others.
    pub fn sfx_stop_non_blocking(h: &mut impl Host, sfx: u32, id: u32) -> Option<u32> {
        if id.cast_signed() <= 0x7f {
            return Some(id);
        }
        sfx_stop(h, sfx, id);
        None
    }
}
