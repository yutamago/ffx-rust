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
use ffx_sg::m5main4_pages::bridge as pages;
use ffx_sg::m5main4_sound::bridge as sound;
use ffx_sg::m5main4_viewer::bridge as viewer;

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

    // ---- batch 2 ----
    /// `(tag, src, n)`: DMA reference tags.
    GsRefTags_0077f160 @ 0x0077_f160 => extern "C" fn gs_ref_tags(tag: u32, src: u32, n: u32) {
        viewer::ref_tags(&mut mem(), tag, src, n);
    }
    /// `(i, x, y, mode)` -> `TexAnim_LoadImageRegs`'s `eax`.
    MovieViewer_BlockStep_0077c3c0 @ 0x0077_c3c0 => extern "C" fn viewer_block_step(i: u32, x: u32, y: u32, mode: u32) -> u32 {
        viewer::block_step(&mut mem(), i, x, y, mode)
    }
    /// `(tag, src, step, w, h, base, half, psm)` -> `FlushCache`'s `eax`.
    MovieViewer_TiledUpload_0077ee40 @ 0x0077_ee40 => extern "C" fn viewer_tiled_upload(tag: u32, src: u32, step: u32, w: u32, hgt: u32, base: u32, half: u32, psm: u32) -> u32 {
        viewer::tiled_upload(&mut mem(), tag, src, step, w, hgt, base, half, psm)
    }
    /// `(v)` -> the last `sceGsSyncPath`'s `eax`.
    MovieViewer_IPictureMovie_0077c4c0 @ 0x0077_c4c0 => extern "C" fn viewer_ipu_movie(v: u32) -> u32 {
        viewer::ipu_movie(&mut mem(), v)
    }
    /// `()` -> 0.
    MovieViewer_IntraDecode_0077b480 @ 0x0077_b480 => extern "C" fn viewer_intra_decode() -> u32 {
        viewer::intra_decode(&mut mem())
    }
    /// `()` -> the last `sceGsSyncPath`'s `eax`.
    MovieViewer_FrameOut_0077ca80 @ 0x0077_ca80 => extern "C" fn viewer_frame_out() -> u32 {
        viewer::frame_out(&mut mem())
    }
    /// `mpegstrendsub(v, convert)` -> the last call's `eax`.
    mpegstrendsub @ 0x0077_dab0 => extern "C" fn mpeg_str_end(v: u32, convert: u32) -> u32 {
        viewer::mpeg_str_end(&mut mem(), v, convert)
    }
    /// `()` -> the last `dbgPrintf`'s `eax`.
    MovieViewer_AllocBuffers_0077df90 @ 0x0077_df90 => extern "C" fn viewer_alloc_buffers() -> u32 {
        viewer::alloc_buffers(&mut mem())
    }
    /// `()` -> the viewer.
    MovieViewer_PictureSetup_0077ce10 @ 0x0077_ce10 => extern "C" fn viewer_picture_setup() -> u32 {
        viewer::picture_setup(&mut mem())
    }
    /// `(m, nb)` -> the chunk's memory or 0.
    dlmalloc_tmalloc_small_006eb950 @ 0x006e_b950 => extern "C" fn tmalloc_small(m: u32, nb: u32) -> u32 {
        lib::tmalloc_small(&mut mem(), m, nb)
    }
    /// `(cache)`: the cluster cache emptied; the gathered clusters in a
    /// frame `PArray`.
    ClusterCache_clear_0069a9c0 @ 0x0069_a9c0 => extern "thiscall" fn cluster_cache_clear(cache: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        chara::cluster_cache_clear(&mut mem(), cache, f.at())
    }
    /// `(path, cloth_a, cloth_b, id)` -> `graphicCharacterLoad`'s `eax`;
    /// the three name buffers in a frame.
    Ch_LoadHdRenderData @ 0x0082_a1c0 => extern "C" fn load_hd_render_data(path: u32, a: u32, b: u32, id: u32) -> u32 {
        let mut f = StackFrame::<0xa0>::new();
        chara::load_hd_render_data(&mut mem(), path, a, b, id, f.at())
    }
    /// `(key)` -> the name buffer; the table path in a frame.
    Ch_MotionName_008371f0 @ 0x0083_71f0 => extern "C" fn motion_name(key: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        chara::motion_name(&mut mem(), key, f.at())
    }
    /// `(chr, set, idx, max, dir)` -> the index set, or 0.
    Ch_SetSysMotion_ViewerAnimation @ 0x0083_7e10 => extern "C" fn set_sys_motion_viewer(chr: u32, set: u32, idx: u32, max: u32, dir: u32) -> u32 {
        chara::set_sys_motion_viewer(&mut mem(), chr, set, idx, max, dir)
    }
    /// `(window, msg, id)` -> 0; the title buffer in a frame.
    ChrAnimWin_Proc_0093e370 @ 0x0093_e370 => extern "C" fn anim_window_proc(window: u32, msg: u32, id: u32) -> u32 {
        let mut f = StackFrame::<32>::new();
        chara::anim_window_proc(&mut mem(), window, msg, id, f.at())
    }
    /// `(chr, dir, sub, name)` -> 4, 0 or `dir`; the path in a frame.
    Ch_ReadMotionViewer @ 0x0082_83c0 => extern "C" fn read_motion_viewer(chr: u32, dir: u32, sub: u32, name: u32) -> u32 {
        let mut f = StackFrame::<32>::new();
        chara::read_motion_viewer(&mut mem(), chr, dir, sub, name, f.at())
    }

    // ---- batch 3 (list main: the page heap) ----
    /// `(pool)` -> a new page or 0.
    PagePool_newPage_00943c50 @ 0x0094_3c50 => extern "thiscall" fn page_pool_new_page(pool: u32) -> u32 {
        pages::new_page(&mut mem(), pool)
    }
    /// `(pool; n)` -> the memory or 0.
    PagePool_alloc_00943a50 @ 0x0094_3a50 => extern "thiscall" fn page_pool_alloc(pool: u32, n: u32) -> u32 {
        pages::pool_alloc(&mut mem(), pool, n)
    }
    /// `(pool; p)` -> `PageRelease`'s `eax` or the page's free size.
    PagePool_free_00943e30 @ 0x0094_3e30 => extern "thiscall" fn page_pool_free(pool: u32, p: u32) -> u32 {
        pages::pool_free(&mut mem(), pool, p)
    }
    /// `(pool)`: every block freed.
    PagePool_freeAll_009439f0 @ 0x0094_39f0 => extern "thiscall" fn page_pool_free_all(pool: u32) {
        pages::pool_free_all(&mut mem(), pool);
    }
    /// `(heap)`: every allocated block freed.
    UserHeap_freeAll_00942a80 @ 0x0094_2a80 => extern "thiscall" fn user_heap_free_all(heap: u32) {
        pages::heap_free_all(&mut mem(), heap);
    }
    /// `(cs)` -> `DeleteCriticalSection`'s `eax`.
    LockedHeap_destroy_00942630 @ 0x0094_2630 => extern "thiscall" fn locked_heap_destroy(cs: u32) -> u32 {
        pages::heap_destroy(&mut mem(), cs)
    }
    /// `(cs)` -> `DeleteCriticalSection`'s `eax`.
    LockedPool_destroy_00942680 @ 0x0094_2680 => extern "thiscall" fn locked_pool_destroy(cs: u32) -> u32 {
        pages::pool_destroy(&mut mem(), cs)
    }
    /// `(alloc; size, align, pool)` -> `alloc`.
    FixedAlloc_init_00944080 @ 0x0094_4080 => extern "thiscall" fn fixed_alloc_init(a: u32, size: u32, align: u32, pool: u32) -> u32 {
        pages::alloc_init(&mut mem(), a, size, align, pool)
    }
    /// `(alloc)` -> `DeleteCriticalSection`'s `eax`.
    FixedAlloc_destroy_00944190 @ 0x0094_4190 => extern "thiscall" fn fixed_alloc_destroy(a: u32) -> u32 {
        pages::alloc_destroy(&mut mem(), a)
    }
    /// `(alloc; chunk)` -> `LeaveCriticalSection`'s `eax`.
    FixedAlloc_removeChunk_00944660 @ 0x0094_4660 => extern "thiscall" fn fixed_alloc_remove_chunk(a: u32, chunk: u32) -> u32 {
        pages::chunk_remove(&mut mem(), a, chunk)
    }
    /// `(it; table)` -> `it`.
    PageHash_iterBegin_00944b40 @ 0x0094_4b40 => extern "thiscall" fn page_hash_iter_begin(it: u32, table: u32) -> u32 {
        pages::iter_init(&mut mem(), it, table)
    }
    /// `(table)` -> the last `VirtualFree_Release`'s `eax`.
    PageHash_clear_00944f00 @ 0x0094_4f00 => extern "thiscall" fn page_hash_clear(table: u32) -> u32 {
        pages::table_clear(&mut mem(), table)
    }
    /// `(big; p)` -> found; the key in a frame.
    BigAlloc_free_00944f80 @ 0x0094_4f80 => extern "thiscall" fn big_alloc_free(big: u32, p: u32) -> bool {
        let mut f = StackFrame::<4>::new();
        pages::big_free(&mut mem(), big, p, f.at()) != 0
    }
    /// `(big)` -> `DeleteCriticalSection`'s `eax`.
    BigAlloc_destroy_00944ba0 @ 0x0094_4ba0 => extern "thiscall" fn big_alloc_destroy(big: u32) -> u32 {
        pages::big_destroy(&mut mem(), big)
    }
    /// `()`: `MemSystem_Ensure`'s slow path; the descriptor in a frame.
    MemSystem_create_009428b0 @ 0x0094_28b0 => extern "C" fn mem_system_create() {
        let mut f = StackFrame::<14>::new();
        pages::mem_system_ensure(&mut mem(), f.at());
    }
    /// `()` -> `Cam_SetPolar`'s `eax` or the index reached; the point in
    /// a frame.
    CamDebug_PointOrbit_00942520 @ 0x0094_2520 => extern "C" fn cam_point_orbit() -> u32 {
        let mut f = StackFrame::<3>::new();
        misc::point_orbit(&mut mem(), f.at())
    }
}
