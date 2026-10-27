//! The movie video's VP8 decoder: in place of FFX.exe's libvpx 1.4.0
//! (`0x00409370..0x0041e5b0` and the DSP routines at
//! `0x00a7c920..0x00a8d6be`), the software decoder of `zvidlib-vp8`
//! (MIT, vendored in `third_party/zvidlib-vp8`, `PATCHES.md`), which
//! decodes every frame of every movie bit for bit as the original does
//! (`docs/formats/platform.md`, "Movies").
//!
//! `PVideoPlaybackWin32_DecodeLoop` (`0x00a279b0`) calls
//! `vpx_codec_decode(ctx, data, size, NULL, 0)` once per packet and
//! `vpx_codec_get_frame` once: a shown frame's visible Y, U and V planes
//! (I420, chroma halved rounding up) go into its queue; a hidden frame
//! (an alternate reference) gives none.

/// One shown frame: the visible planes, rows packed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    /// Y (`width` x `height`), U and V (`width.div_ceil(2)` x
    /// `height.div_ceil(2)`).
    pub planes: [Vec<u8>; 3],
}

/// A VP8 stream's decoder (`vpx_codec_ctx_t` set up by
/// `vpx_codec_dec_init_ver(ctx, vpx_codec_vp8_dx(), NULL, 0, 9)`).
pub struct Decoder {
    inner: zvidlib_vp8::decoder::Decoder,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    /// A decoder before the first key frame.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: zvidlib_vp8::decoder::Decoder::new(zvidlib_core::Limits::default()),
        }
    }

    /// Decodes one frame (`vpx_codec_decode` + `vpx_codec_get_frame`):
    /// the picture when the frame is shown, `None` for a hidden frame or
    /// one that does not decode.
    pub fn decode(&mut self, data: &[u8]) -> Option<Frame> {
        let p = self.inner.decode(data).ok()??;
        Some(Frame {
            width: p.width,
            height: p.height,
            planes: p.planes,
        })
    }
}
