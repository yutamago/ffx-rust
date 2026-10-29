//! M5 stage 1, lists `small` and `main`, round 4: the FMOD glue left over
//! (`ffx_sg::m5main4_sound`), the movie player manager and the PS2 movie
//! helpers (`ffx_sg::m5main4_movie`), libogg's bit packer
//! (`ffx_sg::m5main4_ogg`), libwebm and heap helpers
//! (`ffx_sg::m5main4_lib`) and debug leftovers (`ffx_sg::m5main4_misc`).
//!
//! Threads: the FMOD glue and the movie manager run on the main thread,
//! the video task callback and the Vorbis bit packer on the video
//! player's threads. The shims keep no state; frames the originals hand
//! their callees are `StackFrame`s or the caller's own argument slots.

use super::{StackFrame, mem, ported};
use ffx_sg::m5main4_chara::bridge as chara;
use ffx_sg::m5main4_lib::bridge as lib;
use ffx_sg::m5main4_misc::bridge as misc;
use ffx_sg::m5main4_movie::bridge as movie;
use ffx_sg::m5main4_ogg::bridge as ogg;
use ffx_sg::m5main4_sound::bridge as sound;

ported! {
    // ---- batch 1 ----
    /// `(sfx; seq)`: the event list in a frame, the bank through the
    /// caller's `seq` slot.
    FmodSfx_unloadProjectBySeq @ 0x0070_dbb0 => extern "thiscall" fn unload_project_by_seq(sfx: u32, seq: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        sound::unload_project_by_seq(&mut mem(), sfx, seq, f.at(), slots)
    }
    /// `(shout; id, _, _, system, mute, &status0, &status1, blob)` -> 0.
    FmodShout_start @ 0x0070_e480 => extern "thiscall" fn shout_start(shout: u32, id: u32, _b: u32, _c: u32, system: u32, mute: u32, status0: u32, status1: u32, blob: u32) -> u32 {
        let mut f = StackFrame::<64>::new();
        let a = sound::ShoutStart { id, system, mute, status0, status1, blob };
        sound::shout_start(&mut mem(), shout, a, f.at())
    }
    /// `(mgr)` -> the last call's `eax`.
    PhyFMVPlayerManager_dtor @ 0x006d_6d00 => extern "thiscall" fn fmv_manager_dtor(mgr: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        movie::dtor(&mut mem(), mgr, f.at())
    }
    /// `(queue)`: the movie requests.
    PhyFMVPlayerManager_processRequests @ 0x006d_8cb0 => extern "thiscall" fn fmv_process_requests(q: u32) {
        movie::process_requests(&mut mem(), q);
    }
    /// `(task)` -> `operator delete`'s `eax`.
    PhyFMVPlayerManager_updateVideoCallback @ 0x006d_ae00 => extern "C" fn fmv_update_video_callback(task: u32) -> u32 {
        movie::update_video_callback(&mut mem(), task)
    }
    /// `(fd)` -> 0 or the failing call's result.
    HddResourceWrite_00779d20 @ 0x0077_9d20 => extern "C" fn hdd_resource_write(fd: u32) -> u32 {
        let mut f = StackFrame::<128>::new();
        movie::resource_write(&mut mem(), fd, f.at())
    }
    /// `movieOpenCd(viewer)` -> 1 or the descriptor.
    movieOpenCd @ 0x0077_7240 => extern "C" fn movie_open_cd(viewer: u32) -> u32 {
        movie::open_cd(&mut mem(), viewer)
    }
    /// `movie_hdd_file_open(n, sector)` -> the descriptor or -1; the busy
    /// count goes through the caller's `n` slot.
    movie_hdd_file_open @ 0x0077_9be0 => extern "C" fn movie_hdd_file_open(n: u32, sector: u32; slots: u32) -> u32 {
        movie::hdd_file_open(&mut mem(), n, sector, slots)
    }
    /// `readSizeFile(name, buf)` -> 1 or 0.
    readSizeFile @ 0x0093_da60 => extern "C" fn read_size_file(name: u32, buf: u32) -> u32 {
        movie::read_size_file(&mut mem(), name, buf)
    }
    /// `()`: the camera list's default points.
    CamDebug_DefaultPoints_00941980 @ 0x0094_1980 => extern "C" fn cam_default_points() {
        misc::default_points(&mut mem());
    }
    /// `(b, value, bits)`.
    libogg_oggpack_write @ 0x0042_e350 => extern "C" fn oggpack_write(b: u32, value: u32, bits: u32) {
        ogg::write(&mut mem(), b, value, bits);
    }
    /// `(b, value, bits)`: big-endian.
    libogg_oggpackB_write_0042e480 @ 0x0042_e480 => extern "C" fn oggpack_b_write(b: u32, value: u32, bits: u32) {
        ogg::write_b(&mut mem(), b, value, bits);
    }
    /// `(b, bits)` -> the bits or -1.
    libogg_oggpack_look @ 0x0042_e840 => extern "fastcall" fn oggpack_look(b: u32, bits: u32) -> u32 {
        ogg::look(&mem(), b, bits)
    }
    /// `(b, bits)`.
    libogg_oggpack_adv @ 0x0042_ea30 => extern "fastcall" fn oggpack_adv(b: u32, bits: u32) {
        ogg::adv(&mut mem(), b, bits);
    }
    /// `(b, bits)` -> the bits or -1.
    libogg_oggpack_read @ 0x0042_eb10 => extern "fastcall" fn oggpack_read(b: u32, bits: u32) -> u32 {
        ogg::read(&mut mem(), b, bits)
    }
    /// `(block; cluster)` -> the time (`edx:eax`); `Cluster::Load`'s
    /// length goes through the caller's `cluster` slot.
    libwebm_mkvparser_Block_GetTime_0042df70 @ 0x0042_df70 => extern "thiscall" fn block_get_time(block: u32, cluster: u32; slots: u32) -> u64 {
        let mut f = StackFrame::<2>::new();
        lib::block_get_time(&mut mem(), block, cluster, f.at(), slots)
    }
    /// `(block; idx)` -> the frame.
    libwebm_mkvparser_Block_GetFrame @ 0x0042_e0d0 => extern "thiscall" fn block_get_frame(block: u32, idx: u32) -> u32 {
        lib::block_get_frame(&mut mem(), block, idx)
    }
    /// `(m)` -> the bytes released.
    MSpace_ReleaseSegments_006ebda0 @ 0x006e_bda0 => extern "C" fn release_segments(m: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        lib::release_segments(&mut mem(), m, f.at())
    }
    /// `(q, src, x, y, w, h, width, base)` -> the packet's end.
    setLoadImageTagsTexAnim @ 0x0077_eff0 => extern "C" fn set_load_image_tags(q: u32, src: u32, x: u32, y: u32, w: u32, hgt: u32, width: u32, base: u32) -> u32 {
        chara::set_load_image_tags(&mut mem(), q, src, x, y, w, hgt, width, base)
    }
    /// `(i, q)` -> the packet's end.
    chr_texanim_draw_sub_old @ 0x0077_fe50 => extern "C" fn texanim_draw_sub_old(i: u32, q: u32) -> u32 {
        chara::draw_sub_old(&mut mem(), i, q)
    }
    /// `(i, q)` -> the packet's end.
    chr_texanim_draw_sub_new @ 0x0077_f210 => extern "C" fn texanim_draw_sub_new(i: u32, q: u32) -> u32 {
        chara::draw_sub_new(&mut mem(), i, q)
    }
    /// `(i, q)` -> the packet's end.
    chr_texanim_draw @ 0x0077_c830 => extern "C" fn texanim_draw(i: u32, q: u32) -> u32 {
        chara::draw(&mut mem(), i, q)
    }
    /// `(id, n)` -> the group's `i16`, or -1.
    ChN_ReadSystemMGRP_ViewerAnimation @ 0x0083_69a0 => extern "C" fn read_mgrp_viewer(id: u32, n: u32) -> u32 {
        chara::read_mgrp_viewer(&mut mem(), id, n)
    }
    /// `(task)` -> the second load's `eax`.
    Ch_LoadClothFileTask @ 0x0082_a140 => extern "C" fn load_cloth_file_task(task: u32) -> u32 {
        chara::load_cloth_file_task(&mut mem(), task)
    }
    /// `(name, dir)` -> the viewer key.
    Ch_IdGetFromName @ 0x0082_6dc0 => extern "C" fn id_get_from_name(name: u32, dir: u32) -> u32 {
        chara::id_get_from_name(&mut mem(), name, dir)
    }
}
