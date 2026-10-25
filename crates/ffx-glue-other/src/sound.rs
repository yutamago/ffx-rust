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

/// `Phyre_PSerialization_PStreamFileWin32_isOpen(file)` (thiscall, `al`).
pub const STREAM_IS_OPEN: u32 = 0x0060_7f20;
/// The stream seek `0x006080e0` (thiscall; pos, whence).
pub const STREAM_SEEK: u32 = 0x0060_80e0;
/// `Phyre_PSerialization_PStreamFile_closeFile(file)` (thiscall).
pub const STREAM_CLOSE: u32 = 0x0060_7d80;
/// The stream's empty method `0x00607c10` (a bare `ret`, thiscall).
pub const STREAM_RELEASE: u32 = 0x0060_7c10;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PhyreFIOSStreamFile_seek` (thiscall; pos, whence).
pub const FIOS_SEEK: u32 = 0x006f_9d80;
/// `FMOD_ERR_INVALID_PARAM`, what the file callbacks return for a null
/// handle (`lea eax, [ecx + 0x25]` with `ecx` 0).
pub const FMOD_ERR_INVALID_PARAM: u32 = 0x25;

/// `eventcallback(event, type, a, b, out)` (0x00707a50, stdcall, FMOD's
/// event callback): for types 2 and 4, `*out = 0`; returns 0.
pub fn event_callback(h: &mut impl Mem, kind: u32, out: u32) -> u32 {
    if kind == 2 || kind == 4 {
        h.write(out, 0u32);
    }
    0
}

/// `PhyreFIOSStreamFile_seek(this; pos, whence)` (0x006f9d80, thiscall):
/// the stream's seek when it is open (`al` of `isOpen`), else -1.
pub fn fios_seek(h: &mut impl Host, this: u32, pos: u32, whence: u32) -> u32 {
    let open = h.call_sym(STREAM_IS_OPEN, Conv::Thiscall(this), &[]) & 0xff;
    if open == 0 {
        return u32::MAX;
    }
    h.call_sym(STREAM_SEEK, Conv::Thiscall(this), &[pos, whence])
}

/// `FmodFile_seek(handle, pos, user)` (0x007085e0, stdcall, FMOD's file
/// seek callback): `PhyreFIOSStreamFile_seek(handle; pos, 0)`, 0
/// (`FMOD_OK`); `FMOD_ERR_INVALID_PARAM` for a null handle.
pub fn fmod_file_seek(h: &mut impl Host, handle: u32, pos: u32) -> u32 {
    if handle == 0 {
        return FMOD_ERR_INVALID_PARAM;
    }
    h.call_sym(FIOS_SEEK, Conv::Thiscall(handle), &[pos, 0]);
    0
}

/// `FmodFile_close(handle, user)` (0x007084d0, stdcall, FMOD's file close
/// callback): closes the stream, its empty release method, `operator
/// delete`; 0; `FMOD_ERR_INVALID_PARAM` for a null handle.
pub fn fmod_file_close(h: &mut impl Host, handle: u32) -> u32 {
    if handle == 0 {
        return FMOD_ERR_INVALID_PARAM;
    }
    h.call_sym(STREAM_CLOSE, Conv::Thiscall(handle), &[]);
    h.call_sym(STREAM_RELEASE, Conv::Thiscall(handle), &[]);
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[handle]);
    0
}

/// `FmodSfx_anyEventFlag_0070ed30(list)` (thiscall): whether one of the
/// list's `count` (`+0`, signed) 8-byte entries at `+8` has its byte at
/// `+4` set (`al`; the rest of `eax` the caller's, which no caller reads).
pub fn sfx_any_event_flag(h: &impl Mem, list: u32) -> u8 {
    let n = h.read::<u32>(list).cast_signed();
    if n <= 0 {
        return 0;
    }
    let mut p = h.read::<u32>(list + 8).wrapping_add(4);
    for _ in 0..n {
        if h.read::<u8>(p) != 0 {
            return 1;
        }
        p = p.wrapping_add(8);
    }
    0
}

/// `user_malloc(n)` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
/// `Phyre_PSerialization_PStreamFile_read(file; buf, size)` (thiscall).
pub const STREAM_READ: u32 = 0x0060_8090;
/// `FMOD_ERR_FILE_EOF`, `FmodFile_read`'s short-read result.
pub const FMOD_ERR_FILE_EOF: u32 = 0x16;
/// The import thunk of `FMOD::Event::setPaused` (stdcall event, bool).
pub const EVENT_SET_PAUSED: u32 = 0x0094_8cbe;
/// `ERRCHECK(result, file, line)` (cdecl).
pub const ERRCHECK: u32 = 0x0070_79d0;
/// `SfxEvent.cpp`'s path for `ERRCHECK` (`.rdata`).
pub const SFX_EVENT_FILE: u32 = 0x00b4_fddc;

/// `FmodMemory_alloc(size, type, source)` (0x007066d0, stdcall, FMOD's
/// alloc callback): `user_malloc(size + 16)` moved up to the next
/// 16-byte boundary past the block's start (16 when it is aligned), the
/// distance kept in the byte before it (`FmodMemory_free` reads it); 0
/// for size 0.
pub fn fmod_memory_alloc(h: &mut impl Host, size: u32) -> u32 {
    if size == 0 {
        return 0;
    }
    let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[size.wrapping_add(0x10)]);
    let q = p.wrapping_sub(p & 0xf).wrapping_add(0x10);
    h.write(q.wrapping_sub(1), (q as u8).wrapping_sub(p as u8));
    q
}

/// `FmodFile_read(handle, buf, size, read, user)` (0x007085a0, stdcall,
/// FMOD's file read callback): with a `read` pointer,
/// `Phyre_PSerialization_PStreamFile_read(handle; buf, size)` stored there,
/// `FMOD_ERR_FILE_EOF` when short (unsigned); else 0;
/// `FMOD_ERR_INVALID_PARAM` for a null handle.
pub fn fmod_file_read(h: &mut impl Host, handle: u32, buf: u32, size: u32, read: u32) -> u32 {
    if handle == 0 {
        return FMOD_ERR_INVALID_PARAM;
    }
    if read == 0 {
        return 0;
    }
    let n = h.call_sym(STREAM_READ, Conv::Thiscall(handle), &[buf, size]);
    h.write(read, n);
    if n < size { FMOD_ERR_FILE_EOF } else { 0 }
}

/// `SfxEvent_setPaused(list; paused)` (0x0070f100, thiscall): for each of
/// the list's `count` (`+0`, signed) events (`+8`, 8-byte entries),
/// `ERRCHECK(FMOD::Event::setPaused(event, paused), "SfxEvent.cpp", 0xbd)`
/// (the file and line pushed before the FMOD call). Callers read no
/// result.
pub fn sfx_event_set_paused(h: &mut impl Host, list: u32, paused: u32) {
    let n = h.read::<u32>(list).cast_signed();
    let file = h.rebase(SFX_EVENT_FILE);
    for i in 0..n.max(0).cast_unsigned() {
        let events = h.read::<u32>(list + 8);
        let event = h.read::<u32>(events.wrapping_add(8 * i));
        let r = h.call_sym(EVENT_SET_PAUSED, Conv::Stdcall, &[event, paused]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xbd]);
    }
}

/// `SfxEvent_volumeFrom127_0070ea20(v)`'s divisor (`.rdata` `0x00b922e8`,
/// 127.0).
pub const D_127: Global<f64> = Global::new(0x00b9_22e8);
/// `FmodMusic_loadEventProject(project, name, out, slot)` (thiscall).
pub const LOAD_EVENT_PROJECT: u32 = 0x0070_8d40;
/// `printToOutput(tag, message)` (cdecl).
pub const PRINT_TO_OUTPUT: u32 = 0x0070_7b20;
/// `"[Fmod Error]"`, `"FmodMusic::initData: NULL == eventSystem!\n"`,
/// the two project names (`.rdata`).
pub const FMOD_ERROR_TAG: u32 = 0x00b4_c30c;
/// `FmodMusic_initData`'s message.
pub const NULL_EVENT_SYSTEM: u32 = 0x00b4_f4fc;
/// The first project's name.
pub const PROJECT_A: u32 = 0x00b4_f528;
/// The second project's name.
pub const PROJECT_B: u32 = 0x00b4_f53c;

/// `SfxEvent_volumeFrom127_0070ea20(v)` (stdcall, `ST0`; from
/// `SfxEvent_setPan2`/`_setVolume2`): `f32(f32(v) / 127.0)` clamped to
/// `0..=1` (above 1 gives 1.0, below 0 gives 0.0, through `fstp`/`fld`).
pub fn volume_from_127(h: &impl Mem, v: u32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // fild, then fstp dword
    let a = v.cast_signed() as f32;
    let f = (x87::Reg::load(a) / x87::Reg::constant(D_127.get(h))).store();
    if f > 1.0 {
        return 1.0;
    }
    if f < 0.0 {
        return 0.0;
    }
    f
}

/// `FmodMusic_initData(this; system)` (0x00708c00, thiscall): without an
/// event system, `printToOutput("[Fmod Error]", ...)`'s `eax`; else the
/// two event projects loaded (`FmodMusic_loadEventProject(system, name,
/// this + 8, this)` and `(system, name, this + 0xc, this + 4)`), the
/// second's `eax`.
pub fn fmod_music_init_data(h: &mut impl Host, this: u32, system: u32) -> u32 {
    if system == 0 {
        let tag = h.rebase(FMOD_ERROR_TAG);
        let message = h.rebase(NULL_EVENT_SYSTEM);
        return h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[tag, message]);
    }
    let a = h.rebase(PROJECT_A);
    h.call_sym(
        LOAD_EVENT_PROJECT,
        Conv::Thiscall(this),
        &[system, a, this + 8, this],
    );
    let b = h.rebase(PROJECT_B);
    h.call_sym(
        LOAD_EVENT_PROJECT,
        Conv::Thiscall(this),
        &[system, b, this + 0xc, this + 4],
    )
}

/// The word-array assignments `SfxIdArray_assign_0070c5c0` (reserve
/// `0x0070d3a0`) and `IdArray_assign_007109e0` (`Array_Reserve_00667e00`)
/// (thiscall `(this; src)`): unless `src` is `this`, the count cleared,
/// the storage reserved for `src`'s count, `src`'s words (`+8`) copied
/// (to each non-null slot, `src`'s count read again each round, signed)
/// and the count set; returns `this`.
pub fn word_array_assign(h: &mut impl Host, this: u32, src: u32, reserve: u32) -> u32 {
    if src == this {
        return this;
    }
    h.write(this, 0u32);
    let n = h.read::<u32>(src);
    h.call_sym(reserve, Conv::Thiscall(this), &[n]);
    let mut i = 0u32;
    while i.cast_signed() < h.read::<u32>(src).cast_signed() {
        let d = h.read::<u32>(this + 8).wrapping_add(4 * i);
        if d != 0 {
            let words = h.read::<u32>(src + 8);
            let w = h.read::<u32>(words.wrapping_add(4 * i));
            h.write(d, w);
        }
        i = i.wrapping_add(1);
    }
    let n = h.read::<u32>(src);
    h.write(this, n);
    this
}
