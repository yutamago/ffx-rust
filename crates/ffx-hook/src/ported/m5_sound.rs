//! M5 stage 1, list `sound` (`ffx_sound::m5_sound`): the vsync emulation
//! thread, the rest of the `i*` FMOD queue entry points, and the FMOD
//! objects' constructors, teardown and shutdown.
//!
//! Threads: the main thread (start-up, the `i*` calls, shutdown), the
//! vsync emulation thread (`VsyncThreadImpl_run`), the FMOD thread
//! (`FfxFmod` teardown); every new thread starts in `Thread_Start`. The
//! shims keep no state; frames are per call.

use ffx_bridge::RawMem;
use ffx_sound::m5_sound::bridge as b;

use super::{StackFrame, mem, ported};

/// The game as the bridges' host (`ffx_graphic::host::Host`).
fn game() -> RawMem {
    mem()
}

ported! {
    // ---- the vsync emulation thread ----
    /// `createVsyncEmuThread(handler)`: the thread name in a 100-byte
    /// frame buffer at `ebp - 0x68`.
    createVsyncEmuThread @ 0x0067_9e60 => extern "C" fn create_vsync_emu_thread(handler: u32) -> u32 {
        let mut f = StackFrame::<0x1b>::new();
        b::create_vsync_emu_thread(&mut game(), f.at() + 0x68, handler)
    }
    /// `endVsyncEmuThread()`.
    endVsyncEmuThread @ 0x0067_9f20 => extern "C" fn end_vsync_emu_thread() -> u32 {
        b::end_vsync_emu_thread(&mut game())
    }
    /// `VsyncEmu_Wake()`.
    VsyncEmu_Wake @ 0x0067_9f80 => extern "C" fn vsync_emu_wake() -> u32 {
        b::vsync_emu_wake(&mut game())
    }
    /// `Sys_Thread`'s deleting destructor `(this; flags)`.
    SysThread_deletingDtor_00679bb0 @ 0x0067_9bb0 => extern "thiscall" fn sys_thread_deleting_dtor(this: u32, flags: u32) -> u32 {
        b::sys_thread_deleting_dtor(&mut game(), this, flags)
    }
    /// `VsyncThreadImpl`'s deleting destructor `(this; flags)`.
    VsyncThreadImpl_deletingDtor_00679c00 @ 0x0067_9c00 => extern "thiscall" fn vsync_thread_deleting_dtor(this: u32, flags: u32) -> u32 {
        b::vsync_thread_deleting_dtor(&mut game(), this, flags)
    }
    /// `VsyncThreadImpl_run(this)` (vtable +4).
    VsyncThreadImpl_run @ 0x0067_9c60 => extern "thiscall" fn vsync_thread_run(this: u32) -> u32 {
        b::vsync_thread_run(&mut game(), this)
    }
    /// `Thread_Start_00679db0(thread)`: a thread's start address.
    Thread_Start_00679db0 @ 0x0067_9db0 => extern "stdcall" fn thread_start(thread: u32) -> u32 {
        b::thread_start(&mut game(), thread)
    }
    /// `code_0044cd10(p)`: `jmp Phyre_PhyreFree`.
    Phyre_PhyreFree_thunk_0044cd10 @ 0x0044_cd10 => extern "C" fn phyre_free_thunk(p: u32) -> u32 {
        b::phyre_free_thunk(&mut game(), p)
    }

    // ---- the `i*` entry points ----
    /// `iSfxStopNonLoopSound()`.
    iSfxStopNonLoopSound @ 0x0067_a4c0 => extern "C" fn i_sfx_stop_non_loop_sound() -> u32 {
        b::i_sfx_stop_non_loop_sound(&mut game())
    }
    /// `iShoutStart(a, b, volume)`.
    iShoutStart @ 0x0067_a4e0 => extern "C" fn i_shout_start(a: u32, bb: u32, volume: u32) -> u32 {
        b::i_shout_start(&mut game(), a, bb, volume)
    }
    /// `iSoundSetMasterVolume(v)`: command 0x42.
    iSoundSetMasterVolume_0067a530 @ 0x0067_a530 => extern "C" fn i_sound_set_master_volume(v: u32) -> u32 {
        b::i_set1(&mut game(), 0x42, v)
    }
    /// `iSoundSetMusicVolume(v)`: command 0x43.
    iSoundSetMusicVolume_0067a550 @ 0x0067_a550 => extern "C" fn i_sound_set_music_volume(v: u32) -> u32 {
        b::i_set1(&mut game(), 0x43, v)
    }
    /// `iSoundSetMute(a, b)`: command 0.
    iSoundSetMute @ 0x0067_a570 => extern "C" fn i_sound_set_mute(a: u32, bb: u32) -> u32 {
        b::i_set2(&mut game(), 0, a, bb)
    }
    /// `iSoundSetSfxVolume(v)`: command 0x45.
    iSoundSetSfxVolume @ 0x0067_a590 => extern "C" fn i_sound_set_sfx_volume(v: u32) -> u32 {
        b::i_set1(&mut game(), 0x45, v)
    }
    /// `iSoundSetVoiceVolume(v)`: command 0x44.
    iSoundSetVoiceVolume @ 0x0067_a5b0 => extern "C" fn i_sound_set_voice_volume(v: u32) -> u32 {
        b::i_set1(&mut game(), 0x44, v)
    }
    /// `iVoicePanDirect(v)`: command 0x0c.
    iVoicePanDirect @ 0x0067_a6d0 => extern "C" fn i_voice_pan_direct(v: u32) -> u32 {
        b::i_set1(&mut game(), 0xc, v)
    }
    /// `iVoicePanTranslate(a, b)`: command 0x0b.
    iVoicePanTranslate @ 0x0067_a6f0 => extern "C" fn i_voice_pan_translate(a: u32, bb: u32) -> u32 {
        b::i_set2(&mut game(), 0xb, a, bb)
    }

    // ---- the FMOD objects ----
    /// `FmodVoice`'s constructor (`ecx`).
    FmodVoice_ctor_0070a1a0 @ 0x0070_a1a0 => extern "thiscall" fn voice_ctor(this: u32) -> u32 {
        b::voice_ctor(&mut game(), this)
    }
    /// `CloseHandle(*this)`.
    HandleHolder_close_006f9f60 @ 0x006f_9f60 => extern "thiscall" fn handle_close(this: u32) -> u32 {
        b::handle_close(&mut game(), this)
    }
    /// `FmodMusic`'s teardown (`ecx`).
    FmodMusic_dtor_007086b0 @ 0x0070_86b0 => extern "thiscall" fn music_dtor(this: u32) {
        b::music_dtor(&mut game(), this);
    }
    /// `FmodVoice`'s teardown (`ecx`).
    FmodVoice_dtor_0070a330 @ 0x0070_a330 => extern "thiscall" fn voice_dtor(this: u32) {
        b::voice_dtor(&mut game(), this);
    }
    /// The `FmodSfx +0x64` tree's teardown (`ecx`).
    SfxNodeTree_dtor_0070c3c0 @ 0x0070_c3c0 => extern "thiscall" fn sfx_node_tree_dtor(tree: u32) {
        b::sfx_node_tree_dtor(&mut game(), tree);
    }
    /// `FmodSfx_stopAllSound(this; immediate)`.
    FmodSfx_stopAllSound @ 0x0070_da90 => extern "thiscall" fn sfx_stop_all_sound(this: u32, immediate: u32) -> u32 {
        b::sfx_stop_all_sound(&mut game(), this, immediate)
    }
    /// `SfxBank_stopAllEvents(this; immediate)`.
    SfxBank_stopAllEvents @ 0x0070_fcd0 => extern "thiscall" fn sfx_bank_stop_all_events(this: u32, immediate: u32) {
        b::sfx_bank_stop_all_events(&mut game(), this, immediate);
    }
    /// `FmodSfx_shutdown(this)`.
    FmodSfx_shutdown @ 0x0070_d780 => extern "thiscall" fn sfx_shutdown(this: u32) {
        b::sfx_shutdown(&mut game(), this);
    }
    /// `FmodSfx`'s teardown (`ecx`).
    FmodSfx_dtor_0070c4a0 @ 0x0070_c4a0 => extern "thiscall" fn sfx_dtor(this: u32) {
        b::sfx_dtor(&mut game(), this);
    }
    /// `FfxFmod`'s teardown (`ecx`).
    FfxFmod_dtor_00706590 @ 0x0070_6590 => extern "thiscall" fn ffx_fmod_dtor(this: u32) {
        b::ffx_fmod_dtor(&mut game(), this);
    }

    // ---- the FmodManager, the FMOD objects' shutdown (b2) ----
    /// `FmodManager`'s constructor (`ecx`): the thread's parameter block
    /// in a frame at `ebp - 0x48`.
    FmodManager_ctor_006f9de0 @ 0x006f_9de0 => extern "thiscall" fn fmod_manager_ctor(this: u32) -> u32 {
        let mut f = StackFrame::<0x13>::new();
        b::fmod_manager_ctor(&mut game(), f.at() + 0x48, this)
    }
    /// `FmodManager`'s teardown (`ecx`).
    FmodManager_dtor_006f9f70 @ 0x006f_9f70 => extern "thiscall" fn fmod_manager_dtor(this: u32) -> u32 {
        b::fmod_manager_dtor(&mut game(), this)
    }
    /// `FmodManager`'s deleting destructor `(this; flags)`.
    FmodManager_deletingDtor_006fa040 @ 0x006f_a040 => extern "thiscall" fn fmod_manager_deleting_dtor(this: u32, flags: u32) -> u32 {
        b::fmod_manager_deleting_dtor(&mut game(), this, flags)
    }
    /// `FmodManager_create_006fa0b0()`.
    FmodManager_create_006fa0b0 @ 0x006f_a0b0 => extern "C" fn fmod_manager_create() {
        b::fmod_manager_create(&mut game());
    }
    /// `FmodManager_tick(this)`.
    FmodManager_tick @ 0x006f_a700 => extern "thiscall" fn fmod_manager_tick(this: u32) -> u32 {
        b::fmod_manager_tick(&mut game(), this)
    }
    /// `FmodManager_Destroy()`.
    FmodManager_Destroy @ 0x006f_a190 => extern "C" fn fmod_manager_destroy() {
        b::fmod_manager_destroy(&mut game());
    }
    /// `FmodManager_getSystemTimeInMsec()`: a double in `ST0`.
    FmodManager_getSystemTimeInMsec @ 0x006f_a210 => extern "C" fn get_system_time_in_msec() -> f64 {
        b::get_system_time_in_msec(&mut game())
    }
    /// `FmodMusic_shutdown(this)`.
    FmodMusic_shutdown @ 0x0070_9640 => extern "thiscall" fn music_shutdown(this: u32) {
        b::music_shutdown(&mut game(), this);
    }
    /// The voice stream's release (`ecx`).
    FmodVoice_releaseStream_0070e450 @ 0x0070_e450 => extern "thiscall" fn voice_stream_release(this: u32) -> u32 {
        b::voice_stream_release(&mut game(), this)
    }
    /// `FmodVoice_shutdown(this)`.
    FmodVoice_shutdown @ 0x0070_b480 => extern "thiscall" fn voice_shutdown(this: u32) {
        b::voice_shutdown(&mut game(), this);
    }
    /// `FfxFmod_shutDown(this)`.
    FfxFmod_shutDown @ 0x0070_6ff0 => extern "thiscall" fn ffx_fmod_shutdown(this: u32) -> u32 {
        b::ffx_fmod_shutdown(&mut game(), this)
    }
    /// `FmodMemory_getCurrent_007067f0()`: two words of frame.
    FmodMemory_getCurrent_007067f0 @ 0x0070_67f0 => extern "C" fn fmod_memory_current() -> u32 {
        let mut f = StackFrame::<2>::new();
        b::fmod_memory_current(&mut game(), f.at() + 8)
    }
    /// `FmodMemory_realloc(ptr, size, type, source)` (FMOD's callback).
    FmodMemory_realloc @ 0x0070_6730 => extern "stdcall" fn fmod_memory_realloc(ptr: u32, size: u32, _kind: u32, _source: u32) -> u32 {
        b::fmod_memory_realloc(&mut game(), ptr, size)
    }
    /// `FMOD_ErrorString(result)`.
    FMOD_ErrorString @ 0x0070_7590 => extern "C" fn fmod_error_string(result: u32) -> u32 {
        b::fmod_error_string(&mut game(), result)
    }
    /// `FmodMusic_pauseAll(this; paused)`.
    FmodMusic_pauseAll @ 0x0070_9000 => extern "thiscall" fn music_pause_all(this: u32, paused: u32) {
        b::music_pause_all(&mut game(), this, paused);
    }
    /// `FmodMusic_setMute(this; mute)`.
    FmodMusic_setMute @ 0x0070_9520 => extern "thiscall" fn music_set_mute(this: u32, mute: u32) {
        b::music_set_mute(&mut game(), this, mute);
    }
    /// `FmodMusic_volumeReset(this; id, volume, keep)` (volume f32 bits).
    FmodMusic_volumeReset @ 0x0070_9f20 => extern "thiscall" fn music_volume_reset(this: u32, id: u32, volume: u32, keep: u32) -> u32 {
        b::music_volume_reset(&mut game(), this, id, volume, keep)
    }
}
