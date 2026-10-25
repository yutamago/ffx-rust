//! The FMOD layer's entry points `ffx_sound` had not reached: more of the
//! PS2 sound API's `i*` functions (each queues one command on
//! `g_FmodManager` through `ffx_sound::fmod::bridge`'s queue helpers, or
//! asks the FMOD thread synchronously), the `iSync*` calls on the
//! `SyncManager`, and FMOD's memory and channel callbacks
//! (`docs/formats/sound.md`, "FMOD command queue").

use ffx_bridge::{Global, Mem, x87};
use ffx_graphic::host::{Conv, Host};
use ffx_sound::fmod::bridge::{MANAGER, push0, push1, push2, push3, sync0};

/// The commands these queue (`docs/formats/sound.md`).
pub mod cmd {
    /// `iSoundInit()` (`FmodCmd_soundInit`).
    pub const SOUND_INIT: u32 = 0x01;
    /// `iSoundStageFlush(stage)`.
    pub const SOUND_STAGE_FLUSH: u32 = 0x04;
    /// `iVoiceSetPaused(flag)`.
    pub const VOICE_SET_PAUSED: u32 = 0x08;
    /// `iVoiceDataChange(id)`.
    pub const VOICE_DATA_CHANGE: u32 = 0x09;
    /// `iVoiceStop()`.
    pub const VOICE_STOP: u32 = 0x0a;
    /// `iVoiceStart(pan, volume)`.
    pub const VOICE_START: u32 = 0x0f;
    /// `iVoiceRead(id)`.
    pub const VOICE_READ: u32 = 0x10;
    /// `iVoicePlayStatus()` (synchronous).
    pub const VOICE_PLAY_STATUS: u32 = 0x11;
    /// `iVoiceIsPaused()` (synchronous).
    pub const VOICE_IS_PAUSED: u32 = 0x12;
    /// `iSfxUnloadAllData()`.
    pub const SFX_UNLOAD_ALL_DATA: u32 = 0x31;
    /// `iSfxStart(id, a, b)`.
    pub const SFX_START: u32 = 0x33;
    /// `iSfxStop(id)`.
    pub const SFX_STOP: u32 = 0x36;
    /// `iSfxStopAllNonBlockingSound(id)` (the id as a 64-bit pair).
    pub const SFX_STOP_ALL_NON_BLOCKING: u32 = 0x37;
    /// `iSfxStopNonBlockingSound(id)`.
    pub const SFX_STOP_NON_BLOCKING: u32 = 0x38;
    /// `iSfxSetVolume(id, volume)`.
    pub const SFX_SET_VOLUME: u32 = 0x3b;
    /// `iSfxSetVolumeTranslate(id, volume, frames)`.
    pub const SFX_SET_VOLUME_TRANSLATE: u32 = 0x3c;
}

/// `g_SyncManager` (`0x00ce9014`): the `SyncManager` the `iSync*` calls use.
pub const SYNC_MANAGER: Global<u32> = Global::new(0x00ce_9014);
/// `FmodManager_signalVoiceStart` (thiscall `FmodManager`).
pub const SIGNAL_VOICE_START: u32 = 0x006f_a6c0;
/// `FfxFmod_voiceSetStartSignal` (thiscall `FfxFmod`; on).
pub const VOICE_SET_START_SIGNAL: u32 = 0x0070_7530;
/// `FmodManager +8`: the `FfxFmod`.
pub const MANAGER_FMOD: u32 = 0x8;
/// `SyncManager_syncInit` (thiscall).
pub const SYNC_INIT: u32 = 0x006f_b210;
/// `SyncManager_syncDataChangeAsync` (thiscall; id).
pub const SYNC_DATA_CHANGE: u32 = 0x006f_b080;
/// `SyncManager_syncGetData` (thiscall; a, b, c).
pub const SYNC_GET_DATA: u32 = 0x006f_b160;
/// `SyncManager_initData` (thiscall).
pub const SYNC_INIT_DATA: u32 = 0x006f_a8d0;
/// `FmodMusic_getState`'s table: one word per music id `0..=0xb5`
/// (`0x00b4f1c8`).
pub const MUSIC_STATES: u32 = 0x00b4_f1c8;
/// The last music id of [`MUSIC_STATES`].
pub const MUSIC_LAST: u32 = 0xb5;
/// `user_free_thunk` (cdecl p).
pub const USER_FREE: u32 = 0x0068_7010;
/// The import slot of `FMOD::Channel::setVolume` (stdcall channel, f32).
pub const CHANNEL_SET_VOLUME: u32 = 0x0094_8d12;
/// `PVideoPlaybackWin32 +0x2a0`: the movie's FMOD channel.
pub const VIDEO_CHANNEL: u32 = 0x2a0;
/// `MsGetSaveEventAddress()`: the save block.
pub const SAVE_EVENT_ADDRESS: u32 = 0x0078_5240;
/// Save block `+0x5ec`: the music type flag (bit 0).
pub const SAVE_BGM_TYPE: u32 = 0x5ec;
/// `iMuiscSetBgmType(type)`.
pub const SET_BGM_TYPE: u32 = 0x0067_9fe0;

fn mgr(h: &impl Mem) -> u32 {
    MANAGER.get(h)
}

/// `iSoundInit()` (0x0067a500): command 1, its queue's `eax`.
pub fn i_sound_init(h: &mut impl Host) -> u32 {
    let m = mgr(h);
    push0(h, m, cmd::SOUND_INIT)
}

/// `iSoundStageFlush(stage)` (0x0067a5d0).
pub fn i_sound_stage_flush(h: &mut impl Host, stage: u32) -> u32 {
    let m = mgr(h);
    push1(h, m, cmd::SOUND_STAGE_FLUSH, stage)
}

/// `iSfxUnloadAllData()` (0x0067a4d0).
pub fn i_sfx_unload_all_data(h: &mut impl Host) -> u32 {
    let m = mgr(h);
    push0(h, m, cmd::SFX_UNLOAD_ALL_DATA)
}

/// `iSfxStop(id)` (0x0067a460).
pub fn i_sfx_stop(h: &mut impl Host, id: u32) -> u32 {
    let m = mgr(h);
    push1(h, m, cmd::SFX_STOP, id)
}

/// `iSfxStopNonBlockingSound(id)` (0x0067a4a0).
pub fn i_sfx_stop_non_blocking_sound(h: &mut impl Host, id: u32) -> u32 {
    let m = mgr(h);
    push1(h, m, cmd::SFX_STOP_NON_BLOCKING, id)
}

/// `iSfxStopAllNonBlockingSound(id)` (0x0067a480): the id as a 64-bit
/// pair (`cdq`).
pub fn i_sfx_stop_all_non_blocking_sound(h: &mut impl Host, id: u32) -> u32 {
    let high = (id.cast_signed() >> 31).cast_unsigned();
    let m = mgr(h);
    push2(h, m, cmd::SFX_STOP_ALL_NON_BLOCKING, id, high)
}

/// `iSfxSetVolume(id, volume)` (0x0067a400).
pub fn i_sfx_set_volume(h: &mut impl Host, id: u32, volume: u32) -> u32 {
    let m = mgr(h);
    push2(h, m, cmd::SFX_SET_VOLUME, id, volume)
}

/// `iSfxSetVolumeTranslate(id, volume, frames)` (0x0067a420).
pub fn i_sfx_set_volume_translate(h: &mut impl Host, id: u32, volume: u32, frames: u32) -> u32 {
    let m = mgr(h);
    push3(h, m, cmd::SFX_SET_VOLUME_TRANSLATE, id, volume, frames)
}

/// `iSfxStart(id, a, b)` (0x0067a440).
pub fn i_sfx_start(h: &mut impl Host, id: u32, a: u32, b: u32) -> u32 {
    let m = mgr(h);
    push3(h, m, cmd::SFX_START, id, a, b)
}

/// `iVoiceStop()` (0x0067a7a0).
pub fn i_voice_stop(h: &mut impl Host) -> u32 {
    let m = mgr(h);
    push0(h, m, cmd::VOICE_STOP)
}

/// `iVoiceRead(id)` (0x0067a720).
pub fn i_voice_read(h: &mut impl Host, id: u32) -> u32 {
    let m = mgr(h);
    push1(h, m, cmd::VOICE_READ, id)
}

/// `iVoiceSetPaused(flag)` (0x0067a750).
pub fn i_voice_set_paused(h: &mut impl Host, flag: u32) -> u32 {
    let m = mgr(h);
    push1(h, m, cmd::VOICE_SET_PAUSED, flag)
}

/// `iVoiceDataChange(id)` (0x0067a6a0) -> 0.
pub fn i_voice_data_change(h: &mut impl Host, id: u32) -> u32 {
    let m = mgr(h);
    push1(h, m, cmd::VOICE_DATA_CHANGE, id);
    0
}

/// `iVoiceStart(pan, volume)` (0x0067a770) -> 0: the start signal first
/// (`FmodManager_signalVoiceStart`, `g_FmodManager` read for each call),
/// then command `0x0f`.
pub fn i_voice_start(h: &mut impl Host, pan: u32, volume: u32) -> u32 {
    let m = mgr(h);
    h.call_sym(SIGNAL_VOICE_START, Conv::Thiscall(m), &[]);
    let m = mgr(h);
    push2(h, m, cmd::VOICE_START, pan, volume);
    0
}

/// `iVoicePlayStatus()` (0x0067a710), synchronous.
pub fn i_voice_play_status(h: &mut impl Host) -> u32 {
    let m = mgr(h);
    sync0(h, m, cmd::VOICE_PLAY_STATUS)
}

/// `iVoiceIsPaused()` (0x0067a6c0), synchronous.
pub fn i_voice_is_paused(h: &mut impl Host) -> u32 {
    let m = mgr(h);
    sync0(h, m, cmd::VOICE_IS_PAUSED)
}

/// `FmodManager_signalVoiceStart(mgr)` (0x006fa6c0, thiscall):
/// `FfxFmod_voiceSetStartSignal(mgr->fmod; 1)`'s `eax`.
pub fn fmod_manager_signal_voice_start(h: &mut impl Host, mgr: u32) -> u32 {
    let fmod = h.read::<u32>(mgr + MANAGER_FMOD);
    h.call_sym(VOICE_SET_START_SIGNAL, Conv::Thiscall(fmod), &[1])
}

/// `iSyncInit()` (0x0067a950): a tail jump to
/// `SyncManager_syncInit(g_SyncManager)`.
pub fn i_sync_init(h: &mut impl Host) -> u32 {
    let s = SYNC_MANAGER.get(h);
    h.call_sym(SYNC_INIT, Conv::Thiscall(s), &[])
}

/// `iSyncDataChange(id)` (0x0067a8e0).
pub fn i_sync_data_change(h: &mut impl Host, id: u32) -> u32 {
    let s = SYNC_MANAGER.get(h);
    h.call_sym(SYNC_DATA_CHANGE, Conv::Thiscall(s), &[id])
}

/// `iSyncGetData(a, b, c)` (0x0067a930).
pub fn i_sync_get_data(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
    let s = SYNC_MANAGER.get(h);
    h.call_sym(SYNC_GET_DATA, Conv::Thiscall(s), &[a, b, c])
}

/// `SyncManager_SyncManager(this)` (0x006fa8b0, thiscall): the
/// constructor, `SyncManager_initData(this)`; returns `this`.
pub fn sync_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(SYNC_INIT_DATA, Conv::Thiscall(this), &[]);
    this
}

/// `FmodMusic_getState(id)` (0x00708be0): the table's word for ids up to
/// `0xb5` (unsigned), else -1.
pub fn fmod_music_get_state(h: &impl Mem, id: u32) -> u32 {
    if id > MUSIC_LAST {
        return u32::MAX;
    }
    h.read::<u32>(h.rebase(MUSIC_STATES) + 4 * id)
}

/// `FmodMemory_free(p, type, source)` (0x00706710, stdcall, FMOD's free
/// callback): a block `FmodMemory_alloc` aligned, so its offset from the
/// allocation is the byte before it; `user_free(p - offset)` unless `p`
/// is null. FMOD reads no result.
pub fn fmod_memory_free(h: &mut impl Host, p: u32) {
    if p == 0 {
        return;
    }
    let offset = u32::from(h.read::<u8>(p.wrapping_sub(1)));
    h.call_sym(USER_FREE, Conv::Cdecl, &[p.wrapping_sub(offset)]);
}

/// `PVideoPlaybackWin32_setVolume(this; volume)` (0x00a28330, thiscall):
/// `FMOD::Channel::setVolume(this->channel, volume)` (the float copied
/// through `fld`/`fstp`); 1 when it returns `FMOD_OK`, else 0.
pub fn video_set_volume(h: &mut impl Host, this: u32, volume: u32) -> u32 {
    let v = x87::round_trip(f32::from_bits(volume)).to_bits();
    let channel = h.read::<u32>(this + VIDEO_CHANNEL);
    let r = h.call_sym(CHANNEL_SET_VOLUME, Conv::Stdcall, &[channel, v]);
    u32::from(r == 0)
}

/// `iMusicRefreshBgmType()` (0x008cc170): `iMuiscSetBgmType` of bit 0 of
/// the save block's music type byte; its `eax`.
pub fn i_music_refresh_bgm_type(h: &mut impl Host) -> u32 {
    let save = h.call_sym(SAVE_EVENT_ADDRESS, Conv::Cdecl, &[]);
    let t = u32::from(h.read::<u8>(save + SAVE_BGM_TYPE)) & 1;
    h.call_sym(SET_BGM_TYPE, Conv::Cdecl, &[t])
}
