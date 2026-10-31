//! `ffx-movie`: the PS2 sample library's `FFXSamp0UnitMatrix` and
//! `FFXSamp0ViewScreenMatrix`, and (M5 stage 2) the entry points of the
//! codec libraries the movie player calls: libvpx 1.4.0 and libVorbis
//! 1.3.5 on `ffx-movie`'s decoders (`ffx_movie::api`), so the libraries'
//! own code is no longer reached (`docs/formats/platform.md`, "Movies").

use ffx_bridge::Mem;
use ffx_movie::api;
use ffx_movie::mkv;
use ffx_movie::webm::{Error as WebmError, Source};
use ffx_movie::{ViewScreen, bridge as mv};

use super::{call_thiscall, mem, ported};

/// The game's `IMkvReader` (`VbfMkvReader` over Phyre's `PStreamFile`),
/// read through its vtable: `Read(pos, len, buf)` (slot 0) and
/// `Length(&total, &available)` (slot 1), both thiscall.
struct GameReader {
    reader: u32,
    len: u64,
}

impl GameReader {
    fn new(reader: u32) -> Self {
        let m = mem();
        let vtable = m.read::<u32>(reader);
        let length = m.read::<u32>(vtable + 4);
        let mut out = super::StackFrame::<4>::new();
        let at = out.at();
        let r = call_thiscall(length, reader, &[at, at + 8]);
        let total = m.read::<u64>(at);
        Self {
            reader,
            len: if r == 0 { total } else { 0 },
        }
    }
}

impl Source for GameReader {
    fn len(&self) -> u64 {
        self.len
    }

    /// In `ffx-host` the reader writes game memory, which a Rust buffer is
    /// not (on `x86_64` its address is a host pointer cut to 32 bits: the
    /// reads went nowhere, the segment never parsed and `PVideoPlaybackWin32_open`
    /// faulted in `Segment::GetTracks` on event-arrival's texture movie,
    /// host-gate 2026-10-31): the reads go through a frame on the game
    /// stack, [`READ_CHUNK`] bytes at a time, and are copied out.
    #[cfg(ffx_host)]
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<(), WebmError> {
        let m = mem();
        let read = m.read::<u32>(m.read::<u32>(self.reader));
        let mut frame = super::StackFrame::<{ READ_CHUNK / 4 }>::new();
        let at = frame.at();
        if at == 0 {
            return Err(WebmError::Io("IMkvReader::Read: no frame".into()));
        }
        let mut p = pos;
        for chunk in buf.chunks_mut(READ_CHUNK) {
            #[allow(clippy::cast_possible_truncation)] // halves of the i64; a chunk fits
            let (lo, hi, len) = (p as u32, (p >> 32) as u32, chunk.len() as u32);
            match call_thiscall(read, self.reader, &[lo, hi, len, at]) {
                0 => m.read_bytes(at, chunk),
                r => return Err(WebmError::Io(format!("IMkvReader::Read {r}"))),
            }
            p += u64::from(len);
        }
        Ok(())
    }

    #[cfg(not(ffx_host))]
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<(), WebmError> {
        if buf.is_empty() {
            return Ok(());
        }
        let m = mem();
        let read = m.read::<u32>(m.read::<u32>(self.reader));
        #[allow(clippy::cast_possible_truncation)] // 32-bit process; halves of the i64
        let (lo, hi, at) = (
            pos as u32,
            (pos >> 32) as u32,
            buf.as_mut_ptr() as usize as u32,
        );
        let len = u32::try_from(buf.len()).map_err(|_| WebmError::Truncated)?;
        match call_thiscall(read, self.reader, &[lo, hi, len, at]) {
            0 => Ok(()),
            r => Err(WebmError::Io(format!("IMkvReader::Read {r}"))),
        }
    }
}

/// Bytes per `IMkvReader::Read` in `ffx-host` (`GameReader::read_at`).
#[cfg(ffx_host)]
const READ_CHUNK: usize = 0x4000;

/// The player's `CDataFrameQueue`s and its done event, through the
/// original functions.
struct PlayerQueues;

impl mkv::Queues for PlayerQueues {
    fn reserve(&mut self, queue: u32, len: u32) -> u32 {
        call_thiscall(rebased(0x00a3_dec0), queue, &[len])
    }

    fn commit(&mut self, queue: u32) {
        call_thiscall(rebased(0x00a3_df00), queue, &[]);
    }

    fn close(&mut self, queue: u32) {
        call_thiscall(rebased(0x00a3_dea0), queue, &[]);
    }

    fn signal(&mut self, event: u32) {
        call_thiscall(rebased(0x0043_0200), event, &[]);
    }
}

/// `Segment::CreateInstance`: in the hooked game the segment's objects
/// live in Rust memory, which is the game's; in `ffx-host` (whose arena is
/// not the host's memory) in a block from `mkvparser`'s allocator (the
/// game's `malloc` that `SetAllocator` stored), freed with the segment.
#[cfg(not(ffx_host))]
fn create_segment(src: mkv::BoxedSource, pos: i64, out: u32) -> i64 {
    mkv::create_instance(&mut mem(), src, pos, out)
}

#[cfg(ffx_host)]
fn create_segment(src: mkv::BoxedSource, pos: i64, out: u32) -> i64 {
    let mut m = mem();
    let malloc = m.read::<u32>(m.rebase(mkv::MKV_MALLOC));
    let block = if malloc == 0 {
        0
    } else {
        super::call_cdecl(malloc, &[mkv::GAME_BLOCK])
    };
    if block == 0 {
        if out != 0 {
            m.write::<u32>(out, 0);
        }
        return -1;
    }
    mkv::create_instance_at(&mut m, src, pos, out, block)
}

#[cfg(not(ffx_host))]
fn free_segment_block(_: Option<u32>) {}

#[cfg(ffx_host)]
fn free_segment_block(block: Option<u32>) {
    let m = mem();
    let free = m.read::<u32>(m.rebase(mkv::MKV_FREE));
    if let Some(b) = block
        && free != 0
    {
        super::call_cdecl(free, &[b]);
    }
}

fn rebased(sym: u32) -> u32 {
    u32::try_from(crate::rebase(sym)).unwrap_or(0)
}

ported! {
    /// `FFXSamp0UnitMatrix(m)`: returns the 4 its loop leaves in `eax`.
    FFXSamp0UnitMatrix @ 0x0093_d900 => extern "C" fn ffx_samp0_unit_matrix(m: u32) -> u32 {
        mv::samp0_unit_matrix(&mut mem(), m)
    }
    /// `FFXSamp0ViewScreenMatrix(m, scrz, ax, ay, cx, cy, zmin, zmax,
    /// nearz, farz)`: the screen matrix in this call's own frame, as the
    /// original keeps it.
    FFXSamp0ViewScreenMatrix @ 0x0093_d960 => extern "C" fn ffx_samp0_view_screen_matrix(
        m: u32, scrz: u32, ax: u32, ay: u32, cx: u32, cy: u32, zmin: u32, zmax: u32, nearz: u32, farz: u32
    ) {
        let f = f32::from_bits;
        let v = ViewScreen {
            scrz: f(scrz),
            ax: f(ax),
            ay: f(ay),
            cx: f(cx),
            cy: f(cy),
            zmin: f(zmin),
            zmax: f(zmax),
            nearz: f(nearz),
            farz: f(farz),
        };
        let mut tmp = [0u32; 16];
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = tmp.as_mut_ptr() as usize as u32;
        mv::samp0_view_screen_matrix(&mut mem(), m, at, &v);
    }
    /// `MovieCameraFrame_0076edc0()`: the camera record goes into this
    /// call's own frame, as the original keeps it; without records the
    /// port uses 0 for the original's never-written frame (brief
    /// 2026-10-24). Nobody reads its `eax` (`Sg_MainLoop`,
    /// `Native_b009_MovieCameraWait_Result`).
    MovieCameraFrame_0076edc0 @ 0x0076_edc0 => extern "C" fn movie_camera_frame() -> u32 {
        let mut work = super::StackFrame::<{ (mv::CAMERA_WORK / 4) as usize }>::new();
        let at = work.at();
        mv::movie_camera_frame(&mut mem(), at)
    }

    // ---- libvpx 1.4.0 (PVideoPlaybackWin32_open, _DecodeLoop, _close) ----
    /// `vpx_mem_set_functions(malloc, calloc, realloc, free, memcpy,
    /// memset, memmove)`: libvpx's allocator table (`0x022fb508`), kept as
    /// the original keeps it (allocated once with `malloc`, the seven
    /// pointers stored); the Rust decoder allocates on its own. 0, or -1
    /// when `malloc` gives nothing.
    vpx_mem_set_functions @ 0x0040_a5d0 => extern "C" fn vpx_mem_set_functions(
        malloc: u32, calloc: u32, realloc: u32, free: u32, memcpy: u32, memset: u32, memmove: u32
    ) -> u32 {
        api::vpx_mem_set_functions(&mut mem(), [malloc, calloc, realloc, free, memcpy, memset, memmove])
    }
    /// `vpx_mem_unset_functions()`: frees the table with its own `free`
    /// and clears it; 0.
    vpx_mem_unset_functions @ 0x0040_a630 => extern "C" fn vpx_mem_unset_functions() -> u32 {
        api::vpx_mem_unset_functions(&mut mem())
    }
    /// `vpx_codec_vp8_dx()`: `&vpx_codec_vp8_dx_algo` (`0x00b7f728`), as
    /// the original returns it.
    vpx_codec_vp8_dx @ 0x0040_a430 => extern "C" fn vpx_codec_vp8_dx() -> u32 {
        u32::try_from(crate::rebase(0x00b7_f728)).unwrap_or(0)
    }
    /// `vpx_codec_dec_init_ver(ctx, iface, cfg, flags, ver)`.
    vpx_codec_dec_init_ver @ 0x0040_94c0 => extern "C" fn vpx_codec_dec_init_ver(
        ctx: u32, iface: u32, cfg: u32, flags: u32, ver: u32
    ) -> u32 {
        api::vpx_dec_init(&mut mem(), ctx, iface, cfg, flags, ver)
    }
    /// `vpx_codec_decode(ctx, data, size, user_priv, deadline)`.
    vpx_codec_decode @ 0x0040_9680 => extern "C" fn vpx_codec_decode(
        ctx: u32, data: u32, size: u32, user_priv: u32, deadline: u32
    ) -> u32 {
        api::vpx_decode(&mem(), ctx, data, size, user_priv, deadline)
    }
    /// `vpx_codec_get_frame(ctx, iter)`.
    vpx_codec_get_frame @ 0x0040_96e0 => extern "C" fn vpx_codec_get_frame(ctx: u32, iter: u32) -> u32 {
        api::vpx_get_frame(&mut mem(), ctx, iter)
    }
    /// `vpx_codec_destroy(ctx)`.
    vpx_codec_destroy @ 0x0040_9370 => extern "C" fn vpx_codec_destroy(ctx: u32) -> u32 {
        api::vpx_destroy(&mut mem(), ctx)
    }

    // ---- libwebm's mkvparser (PVideoPlaybackWin32_open, _AudioInit,
    // _close) and the demux loop; Segment::GetTracks (0x004251b0) and
    // Track::GetType (0x00427b10) are four bytes, too short to hook: they
    // run on mkv's objects, laid out for them ----
    /// `mkvparser::SetAllocator(malloc, free)`; `eax` is `free`, as the
    /// original leaves it.
    mkvparser_SetAllocator @ 0x0041_e5b0 => extern "C" fn mkvparser_set_allocator(malloc: u32, free: u32) -> u32 {
        mkv::set_allocator(&mut mem(), malloc, free);
        free
    }
    /// `mkvparser::ResetAllocator()`; `eax` is the CRT's `free`, as the
    /// original leaves it.
    mkvparser_ResetAllocator @ 0x0041_e5d0 => extern "C" fn mkvparser_reset_allocator() -> u32 {
        let mut m = mem();
        mkv::reset_allocator(&mut m);
        m.read::<u32>(m.rebase(mkv::MKV_FREE))
    }
    /// `Segment::CreateInstance(reader, pos, &segment)`: over the game's
    /// reader.
    mkvparser_Segment_CreateInstance @ 0x0041_fd50 => extern "C" fn mkvparser_segment_create_instance(
        reader: u32, pos_lo: u32, pos_hi: u32, out: u32
    ) -> u64 {
        let pos = (u64::from(pos_hi) << 32 | u64::from(pos_lo)).cast_signed();
        create_segment(Box::new(GameReader::new(reader)), pos, out).cast_unsigned()
    }
    /// `Segment::Load()`.
    mkvparser_Segment_Load @ 0x0042_1450 => extern "thiscall" fn mkvparser_segment_load(seg: u32) -> u64 {
        let r = mkv::load(seg);
        mkv::sync(&mut mem(), seg);
        r.cast_unsigned()
    }
    /// `delete segment`.
    mkvparser_Segment_delete @ 0x0041_fd10 => extern "C" fn mkvparser_segment_delete(seg: u32) {
        free_segment_block(mkv::delete(seg));
    }
    /// `Segment::GetDuration()`.
    mkvparser_Segment_GetDuration @ 0x0042_5200 => extern "thiscall" fn mkvparser_segment_get_duration(seg: u32) -> u64 {
        mkv::duration(&mem(), seg).cast_unsigned()
    }
    /// `Tracks::GetTracksCount()`.
    mkvparser_Tracks_GetTracksCount @ 0x0042_94c0 => extern "thiscall" fn mkvparser_tracks_count(tracks: u32) -> u32 {
        mkv::tracks_count(&mem(), tracks)
    }
    /// `Tracks::GetTrackByIndex(i)`.
    mkvparser_Tracks_GetTrackByIndex @ 0x0042_9de0 => extern "thiscall" fn mkvparser_track_by_index(tracks: u32, i: u32) -> u32 {
        mkv::track_by_index(&mem(), tracks, i)
    }
    /// `Track::GetCodecPrivate(size&)`.
    mkvparser_Track_GetCodecPrivate @ 0x0042_7b80 => extern "thiscall" fn mkvparser_codec_private(track: u32, size: u32) -> u32 {
        mkv::codec_private(&mut mem(), track, size)
    }
    /// `VideoTrack::GetWidth()`.
    mkvparser_VideoTrack_GetWidth @ 0x0042_8db0 => extern "thiscall" fn mkvparser_width(track: u32) -> u64 {
        mkv::field_i64(&mem(), track, 0x90).cast_unsigned()
    }
    /// `VideoTrack::GetHeight()`.
    mkvparser_VideoTrack_GetHeight @ 0x0042_8dc0 => extern "thiscall" fn mkvparser_height(track: u32) -> u64 {
        mkv::field_i64(&mem(), track, 0x98).cast_unsigned()
    }
    /// `VideoTrack::GetFrameRate()` (`ST0`).
    mkvparser_VideoTrack_GetFrameRate @ 0x0042_8dd0 => extern "thiscall" fn mkvparser_frame_rate(track: u32) -> f64 {
        mkv::field_f64(&mem(), track, 0xa0)
    }
    /// `AudioTrack::GetSamplingRate()` (`ST0`).
    mkvparser_AudioTrack_GetSamplingRate @ 0x0042_9170 => extern "thiscall" fn mkvparser_sampling_rate(track: u32) -> f64 {
        mkv::field_f64(&mem(), track, 0x90)
    }
    /// `AudioTrack::GetChannels()`.
    mkvparser_AudioTrack_GetChannels @ 0x0042_9180 => extern "thiscall" fn mkvparser_channels(track: u32) -> u64 {
        mkv::field_i64(&mem(), track, 0x98).cast_unsigned()
    }
    /// `PVideoPlaybackWin32_DemuxLoop(this)` (the reading thread): the
    /// cluster and block walk on `ffx_movie::webm`, into the original
    /// queues.
    PVideoPlaybackWin32_DemuxLoop @ 0x00a2_7650 => extern "thiscall" fn pvideo_demux_loop(player: u32) {
        mkv::demux_loop(&mut mem(), player, &mut PlayerQueues);
    }

    // ---- libVorbis 1.3.5 (PVideoPlaybackWin32_AudioInit, _AudioRead,
    // _AudioShutdown) ----
    /// `vorbis_info_init(vi)`.
    vorbis_info_init @ 0x00a8_ffd0 => extern "C" fn vorbis_info_init(vi: u32) {
        api::vorbis_info_init(&mut mem(), vi);
    }
    /// `vorbis_comment_init(vc)`.
    vorbis_comment_init @ 0x00a9_0420 => extern "C" fn vorbis_comment_init(vc: u32) {
        api::vorbis_comment_init(&mut mem(), vc);
    }
    /// `vorbis_synthesis_headerin(vi, vc, op)` (`PhyreEngine`'s: returns
    /// the bytes a header used).
    vorbis_synthesis_headerin @ 0x00a8_f650 => extern "C" fn vorbis_synthesis_headerin(vi: u32, vc: u32, op: u32) -> u32 {
        api::vorbis_headerin(&mut mem(), vi, vc, op)
    }
    /// `vorbis_synthesis_init(v, vi)`.
    vorbis_synthesis_init @ 0x00a9_10e0 => extern "C" fn vorbis_synthesis_init(v: u32, vi: u32) -> u32 {
        api::vorbis_synthesis_init(&mut mem(), v, vi)
    }
    /// `vorbis_block_init(v, vb)`.
    vorbis_block_init @ 0x00a9_21b0 => extern "C" fn vorbis_block_init(v: u32, vb: u32) -> u32 {
        api::vorbis_block_init(&mut mem(), v, vb)
    }
    /// `vorbis_synthesis(vb, op)`.
    vorbis_synthesis @ 0x00a9_2460 => extern "C" fn vorbis_synthesis(vb: u32, op: u32) -> u32 {
        api::vorbis_synthesis(&mem(), vb, op)
    }
    /// `vorbis_synthesis_blockin(v, vb)`.
    vorbis_synthesis_blockin @ 0x00a9_0a70 => extern "C" fn vorbis_synthesis_blockin(v: u32, vb: u32) -> u32 {
        api::vorbis_blockin(v, vb)
    }
    /// `vorbis_synthesis_pcmout(v, pcm)`.
    vorbis_synthesis_pcmout @ 0x00a9_0a10 => extern "C" fn vorbis_synthesis_pcmout(v: u32, pcm: u32) -> u32 {
        api::vorbis_pcmout(&mut mem(), v, pcm)
    }
    /// `vorbis_synthesis_read(v, n)`.
    vorbis_synthesis_read @ 0x00a9_09e0 => extern "C" fn vorbis_synthesis_read(v: u32, n: u32) -> u32 {
        api::vorbis_read(v, n)
    }
    /// `vorbis_block_clear(vb)`.
    vorbis_block_clear @ 0x00a9_2040 => extern "C" fn vorbis_block_clear(vb: u32) {
        api::vorbis_block_clear(&mut mem(), vb);
    }
    /// `vorbis_dsp_clear(v)`.
    vorbis_dsp_clear @ 0x00a9_17a0 => extern "C" fn vorbis_dsp_clear(v: u32) {
        api::vorbis_dsp_clear(&mut mem(), v);
    }
    /// `vorbis_comment_clear(vc)`.
    vorbis_comment_clear @ 0x00a9_0030 => extern "C" fn vorbis_comment_clear(vc: u32) {
        api::vorbis_comment_clear(&mut mem(), vc);
    }
    /// `vorbis_info_clear(vi)`.
    vorbis_info_clear @ 0x00a8_fd80 => extern "C" fn vorbis_info_clear(vi: u32) {
        api::vorbis_info_clear(&mut mem(), vi);
    }
}
