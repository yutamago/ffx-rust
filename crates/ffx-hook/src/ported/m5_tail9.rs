//! M5 stage 1, round 9 (`tail9`): the movie player around the replaced
//! libraries (`ffx_pcui::l9_video`, list `phyre-media`) and the ability
//! map's job kicks (`ffx_system::l9_jobs`).
//!
//! Threads: the reading and decoding threads run their own start
//! routines here (`PVideoPlaybackWin32__ReadingThread`,
//! `__VideoDecodingThread`), FMOD's mixer thread the read callback; the
//! rest runs on the main thread. Frames are a few words, per call.

use super::{StackFrame, mem, ported};
use ffx_pcui::l9_video::bridge as v;
use ffx_system::l9_jobs::bridge as jobs;

ported! {
    /// `(player)`: the reading thread's start routine; 0.
    PVideoPlaybackWin32__ReadingThread @ 0x00a2_7880 => extern "C" fn reading_thread(player: u32) -> u32 {
        v::reading_thread(&mut mem(), player)
    }
    /// `(player)`: the open-state word in a frame.
    PVideoPlaybackWin32_AudioShutdown @ 0x00a2_78f0 => extern "thiscall" fn audio_shutdown(player: u32) -> u32 {
        let mut state = StackFrame::<1>::new();
        let at = state.at();
        v::audio_shutdown(&mut mem(), player, at)
    }
    /// `(player)`: the decoder's iterator in a frame.
    PVideoPlaybackWin32_DecodeLoop @ 0x00a2_79b0 => extern "thiscall" fn decode_loop(player: u32) -> u32 {
        let mut iter = StackFrame::<1>::new();
        let at = iter.at();
        v::decode_loop(&mut mem(), player, at)
    }
    /// `(player)`: the decoding thread's start routine; 0.
    PVideoPlaybackWin32__VideoDecodingThread @ 0x00a2_7b90 => extern "C" fn decoding_thread(player: u32) -> u32 {
        v::decoding_thread(&mut mem(), player)
    }
    /// `(player)` -> ms (`edx:eax`).
    Phyre_PVideo_PVideoPlaybackWin32_getCurrentTime @ 0x00a2_7bd0 => extern "thiscall" fn get_current_time(player: u32) -> u64 {
        v::get_current_time(&mut mem(), player)
    }
    /// `(player)` -> the end time (`edx:eax`).
    Phyre_PVideo_PVideoPlaybackWin32_getEndTime @ 0x00a2_7c40 => extern "thiscall" fn get_end_time(player: u32) -> u64 {
        v::get_end_time(&mem(), player)
    }
    /// `(player)` -> `al`: the open-state word in a frame.
    PVideoPlaybackWin32_isPlaying @ 0x00a2_7c50 => extern "thiscall" fn is_playing(player: u32) -> u8 {
        let mut state = StackFrame::<1>::new();
        let at = state.at();
        v::is_playing(&mut mem(), player, at)
    }
    /// `(player; unused, frames, system)`: the thread's
    /// `PThreadInitializationWin32` in a frame at the original's
    /// `ebp - 0x18`.
    PVideoPlaybackWin32_initialize @ 0x00a2_7cf0 => extern "thiscall" fn initialize(player: u32, _unused: u32, frames: u32, system: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        let ebp = frame.at() + 0x18;
        v::initialize(&mut mem(), ebp, player, frames, system)
    }
    /// `(player; time, flags)` -> `al` 1: seeking does nothing.
    PVideoPlaybackWin32_seek @ 0x00a2_7e10 => extern "thiscall" fn seek(_player: u32, _time: u32, _flags: u32) -> u8 {
        1
    }
    /// `(texture; out)`.
    PVideoPlaybackWin32_MapTexture @ 0x00a2_7e20 => extern "thiscall" fn map_texture(tex: u32, out: u32) -> u32 {
        v::map_texture(&mut mem(), tex, out)
    }
    /// `(player)` -> `al` 1.
    PVideoPlaybackWin32_togglePause @ 0x00a2_7e90 => extern "thiscall" fn toggle_pause(player: u32) -> u8 {
        v::toggle_pause(&mut mem(), player)
    }
    /// `(sound, data, len)` -> 0: the user-data word in a frame.
    PVideoPlaybackWin32_FmodReadCallback_00a27ee0 @ 0x00a2_7ee0 => extern "stdcall" fn fmod_read_callback(sound: u32, data: u32, len: u32) -> u32 {
        let mut slot = StackFrame::<1>::new();
        let at = slot.at();
        v::fmod_read_callback(&mut mem(), at, sound, data, len)
    }
    /// `(player)`: the thread's exit code in a frame.
    PVideoPlaybackWin32_terminate @ 0x00a2_8430 => extern "thiscall" fn terminate(player: u32) -> u32 {
        let mut code = StackFrame::<1>::new();
        let at = code.at();
        v::terminate(&mut mem(), player, at)
    }
    /// `(player; path, looping)` -> 0, -1, -2, -3: the decoding thread's
    /// `PThreadInitializationWin32` in a frame at the original's
    /// `ebp - 0x28`.
    PVideoPlaybackWin32_open @ 0x00a2_7f20 => extern "thiscall" fn open(player: u32, path: u32, looping: u32) -> u32 {
        let mut frame = StackFrame::<{ (v::OPEN_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + v::OPEN_FRAME;
        v::open(&mut mem(), ebp, player, path, looping)
    }
    /// `(player; y, u, v)` -> `al`: the mapping object in a frame.
    Phyre_PVideo_PVideoPlaybackWin32_updateTexture @ 0x00a2_84a0 => extern "thiscall" fn update_texture(player: u32, y: u32, u: u32, w: u32) -> u8 {
        let mut mapping = StackFrame::<{ (v::MAPPING_SIZE / 4) as usize }>::new();
        let at = mapping.at();
        v::update_texture(&mut mem(), at, player, y, u, w)
    }
    /// `(texture; data)`.
    PTexture2DD3D11_unmapLevel_005a8550 @ 0x005a_8550 => extern "thiscall" fn unmap_level(tex: u32, data: u32) -> u32 {
        v::unmap_level(&mut mem(), tex, data)
    }
    /// `(host, cmd)` -> the next command: the opus colour handler (table
    /// entry 0x90); the CLUT green pass's two vectors in a frame.
    OpusColour_00809f90 @ 0x0080_9f90 => extern "C" fn opus_colour(host: u32, cmd: u32) -> u32 {
        let mut vectors = StackFrame::<{ (ffx_glue_other::l9_vfx::bridge::VECTORS / 4) as usize }>::new();
        let at = vectors.at();
        ffx_glue_other::l9_vfx::bridge::opus_colour(&mut mem(), at, host, cmd)
    }
    /// `(entry)` -> `entry`: the input devices' entry constructor.
    PInputDevice_entryCtor_00620540 @ 0x0062_0540 => extern "thiscall" fn input_entry_ctor(entry: u32) -> u32 {
        ffx_pother_b::l9_input::bridge::entry_ctor(&mut mem(), entry)
    }
    /// `()`: the ability map's job wait (queue 0 of `0x00cdede4`).
    AbmapJobWait_00a8ecf0 @ 0x00a8_ecf0 => extern "C" fn abmap_job_wait() {
        ffx_system::m5a_platform::bridge::wait_cloth(&mut mem());
    }
    /// `()`: the queue word `pushJob` writes in a frame.
    AbmapJobKick_00a8ed00 @ 0x00a8_ed00 => extern "C" fn abmap_kick_ed00() -> u32 {
        let mut slot = StackFrame::<1>::new();
        let at = slot.at();
        jobs::kick(&mut mem(), jobs::KICKS[0].1, at)
    }
    /// `()`.
    AbmapJobKick_00a8ed20 @ 0x00a8_ed20 => extern "C" fn abmap_kick_ed20() -> u32 {
        let mut slot = StackFrame::<1>::new();
        let at = slot.at();
        jobs::kick(&mut mem(), jobs::KICKS[1].1, at)
    }
    /// `()`.
    AbmapJobKick_00a8ed40 @ 0x00a8_ed40 => extern "C" fn abmap_kick_ed40() -> u32 {
        let mut slot = StackFrame::<1>::new();
        let at = slot.at();
        jobs::kick(&mut mem(), jobs::KICKS[2].1, at)
    }
    /// `()`.
    AbmapJobKick_00a8ed60 @ 0x00a8_ed60 => extern "C" fn abmap_kick_ed60() -> u32 {
        let mut slot = StackFrame::<1>::new();
        let at = slot.at();
        jobs::kick(&mut mem(), jobs::KICKS[3].1, at)
    }
}
