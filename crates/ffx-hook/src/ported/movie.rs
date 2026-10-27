//! `ffx-movie`: the PS2 sample library's `FFXSamp0UnitMatrix` and
//! `FFXSamp0ViewScreenMatrix`, and (M5 stage 2) the entry points of the
//! codec libraries the movie player calls: libvpx 1.4.0 and libVorbis
//! 1.3.5 on `ffx-movie`'s decoders (`ffx_movie::api`), so the libraries'
//! own code is no longer reached (`docs/formats/platform.md`, "Movies").

use ffx_movie::api;
use ffx_movie::{ViewScreen, bridge as mv};

use super::{mem, ported};

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
