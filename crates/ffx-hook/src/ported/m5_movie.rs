//! M5 stage 1, list `movie` (`ffx_movie::m5_movie`): the FMV player
//! manager's teardown helpers and time queries, the PS2 movie layer's
//! debug stubs, its file opens and small glue.
//!
//! Threads: the main thread (the movie player, its PS2 layer); the shims
//! keep no state, frames are per call.

use ffx_bridge::RawMem;
use ffx_movie::m5_movie::bridge as b;

use super::{StackFrame, mem, ported};

/// The game as the bridges' host (`ffx_graphic::host::Host`).
fn game() -> RawMem {
    mem()
}

ported! {
    /// `PhyFMVPlayerManager_DestroyInstance()`.
    PhyFMVPlayerManager_DestroyInstance @ 0x006d_7370 => extern "C" fn destroy_instance() {
        b::destroy_instance(&mut game());
    }
    /// A `std::list`'s destructor (`ecx`).
    StdList_dtor_006d6c30 @ 0x006d_6c30 => extern "thiscall" fn list_dtor(list: u32) {
        b::list_dtor(&mut game(), list);
    }
    /// A `std::map`'s destructor (`ecx`): the erase's iterator in a frame
    /// word at `ebp - 4`.
    StdMap_dtor_006d6c70 @ 0x006d_6c70 => extern "thiscall" fn map_dtor(map: u32) {
        let mut f = StackFrame::<1>::new();
        b::map_dtor(&mut game(), f.at() + 4, map);
    }
    /// `user_free` of `+0x18` and `+0xc` (`ecx`).
    FmvBuffers_free_006d6cd0 @ 0x006d_6cd0 => extern "thiscall" fn free_buffers(this: u32) -> u32 {
        b::free_buffers(&mut game(), this)
    }
    /// The request queue's destructor (`ecx`).
    FmvRequestQueue_dtor_006daf30 @ 0x006d_af30 => extern "thiscall" fn request_queue_dtor(this: u32) {
        b::request_queue_dtor(&mut game(), this);
    }
    /// The request queue's constructor (`ecx`).
    FmvRequestQueue_ctor_006daee0 @ 0x006d_aee0 => extern "thiscall" fn request_queue_ctor(this: u32) -> u32 {
        b::request_queue_ctor(&mut game(), this)
    }
    /// `PhyFMVPlayerManager_getEndTimeMs(this)`.
    PhyFMVPlayerManager_getEndTimeMs @ 0x006d_91d0 => extern "thiscall" fn get_end_time_ms(this: u32) -> u32 {
        b::get_end_time_ms(&mut game(), this)
    }
    /// `PhyFMVPlayerManager_isVideoAtEndFrame(this)` -> `al`.
    PhyFMVPlayerManager_isVideoAtEndFrame @ 0x006d_a240 => extern "thiscall" fn is_video_at_end_frame(this: u32) -> bool {
        b::is_video_at_end_frame(&mut game(), this)
    }
    /// `jmp PhyFMVPlayerManager_processRequests` (`ecx`).
    PhyFMVPlayerManager_processRequests_thunk_006dadf0 @ 0x006d_adf0 => extern "thiscall" fn process_requests_thunk(this: u32) -> u32 {
        b::process_requests_thunk(&mut game(), this)
    }

    // ---- the PS2 movie layer ----
    /// `dbgPrintf` stub.
    code_0076ed80 @ 0x0076_ed80 => extern "C" fn debug_stub_0076ed80() {
        b::debug_stub(&mut game(), 0x00b5_2e28);
    }
    /// `dbgPrintf` stub.
    code_0076ed90 @ 0x0076_ed90 => extern "C" fn debug_stub_0076ed90() {
        b::debug_stub(&mut game(), 0x00b5_2f10);
    }
    /// `dbgPrintf` stub.
    code_0076eda0 @ 0x0076_eda0 => extern "C" fn debug_stub_0076eda0() {
        b::debug_stub(&mut game(), 0x00b5_2f80);
    }
    /// `movie_frame_prog()` -> 0.
    movie_frame_prog @ 0x0076_efb0 => extern "C" fn movie_frame_prog() -> u32 {
        b::debug_stub(&mut game(), 0x00b5_2810);
        0
    }
    /// `dbgPrintf` stub.
    code_0076f010 @ 0x0076_f010 => extern "C" fn debug_stub_0076f010() {
        b::debug_stub(&mut game(), 0x00b5_2410);
    }
    /// `movie_load_image_set()`.
    movie_load_image_set @ 0x0076_f030 => extern "C" fn movie_load_image_set() {
        b::debug_stub(&mut game(), 0x00b5_2e98);
    }
    /// `movie_play_prog()`.
    movie_play_prog @ 0x0076_f040 => extern "C" fn movie_play_prog() {
        b::debug_stub(&mut game(), 0x00b5_27a0);
    }
    /// `movie_start_debug_name()`.
    movie_start_debug_name @ 0x0076_f070 => extern "C" fn movie_start_debug_name() {
        b::debug_stub(&mut game(), 0x00b5_25e0);
    }
    /// `movie_sync_prog()` -> 0.
    movie_sync_prog @ 0x0076_f0a0 => extern "C" fn movie_sync_prog() -> u32 {
        b::debug_stub(&mut game(), 0x00b5_2730);
        0
    }
    /// `dbgPrintf` stub -> 0.
    code_0076f0b0 @ 0x0076_f0b0 => extern "C" fn debug_stub_0076f0b0() -> u32 {
        b::debug_stub(&mut game(), 0x00b5_26c0);
        0
    }
    /// `movie_wait()`.
    movie_wait @ 0x0077_daa0 => extern "C" fn movie_wait() -> u32 {
        b::movie_wait(&mut game())
    }
    /// `jmp TODebugGetPadPrintFlag`.
    TODebugGetPadPrintFlag_thunk_0077da90 @ 0x0077_da90 => extern "C" fn pad_print_flag_thunk() -> u32 {
        b::jmp_thunk(&mut game(), b::TO_DEBUG_PAD_PRINT_FLAG, &[])
    }
    /// `movieOpen(name)`.
    movieOpen @ 0x0093_da30 => extern "C" fn movie_open(name: u32) -> u32 {
        b::movie_open(&mut game(), name)
    }
    /// `movieClose(fd)`.
    movieClose @ 0x0093_da00 => extern "C" fn movie_close(fd: u32) -> u32 {
        b::movie_close(&mut game(), fd)
    }
    /// `0x0077dc50(convert)`: its tail calls' argument through its slot.
    MovieFrameWait_0077dc50 @ 0x0077_dc50 => extern "C" fn movie_frame_wait(convert: u32; slots: u32) -> u32 {
        b::movie_frame_wait(&mut game(), convert, slots)
    }
    /// `code_0077de40()`.
    code_0077de40 @ 0x0077_de40 => extern "C" fn movie_audio_mix() {
        b::movie_audio_mix(&mut game());
    }
    /// `movie_view()`.
    movie_view @ 0x0094_1830 => extern "C" fn movie_view() {
        b::movie_view(&mut game());
    }

    // ---- the PS2 movie layer's frame conversion and reads (b6) ----
    /// `0x0077de90()`: a 0x80-byte buffer at `ebp - 0x84`.
    MovieDebugLine_0077de90 @ 0x0077_de90 => extern "C" fn movie_debug_line() -> u32 {
        let mut f = StackFrame::<0x21>::new();
        b::movie_debug_line(&mut game(), f.at() + 0x84)
    }
    /// `movie_csc()`.
    movie_csc @ 0x0077_d5e0 => extern "C" fn movie_csc() -> u32 {
        b::movie_csc(&mut game())
    }
    /// `0x0077e110(fd, buf, size, &read, work)`.
    MovieRead_0077e110 @ 0x0077_e110 => extern "C" fn movie_read(fd: u32, buf: u32, size: u32, read: u32, work: u32) -> u32 {
        b::movie_read(&mut game(), fd, buf, size, read, work)
    }
    /// `0x0077e160(slot)`.
    MovieTakeSizes_0077e160 @ 0x0077_e160 => extern "C" fn movie_take_sizes(slot: u32) -> u32 {
        b::movie_take_sizes(&mut game(), slot)
    }
    /// `movieHardWareInit()`.
    movieHardWareInit @ 0x0077_d230 => extern "C" fn movie_hardware_init() -> u32 {
        b::movie_hardware_init(&mut game())
    }
    /// `0x00777320(sector, size, buf, mode)`.
    MovieCdRead_00777320 @ 0x0077_7320 => extern "C" fn movie_cd_read(sector: u32, size: u32, buf: u32, mode: u32) -> u32 {
        b::movie_cd_read(&mut game(), sector, size, buf, mode)
    }
}
