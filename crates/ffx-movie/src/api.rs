//! The libvpx and libVorbis C entry points `PVideoPlaybackWin32` calls,
//! over the game's memory, backed by [`crate::vp8`] and [`crate::vorbis`]
//! (`docs/formats/platform.md`, "Movies"). The hook (`ffx-hook`'s
//! `ported/movie.rs`) replaces the originals with these, so the libraries
//! linked into FFX.exe are no longer called.
//!
//! The C structs the player owns (`vpx_codec_ctx_t` at player `+0x130`,
//! `vorbis_info` `+0x180`, `vorbis_comment` `+0x1a0`, `vorbis_dsp_state`
//! `+0x1b0`, `vorbis_block` `+0x220`, the `ogg_packet` `+0x160`) stay in
//! game memory; the decoders behind them live here, keyed by the struct's
//! address. What the player reads back is written as the originals write
//! it: `vorbis_info.channels` (`+4`, `PVideoPlaybackWin32_AudioRead` reads
//! it), the `vpx_image_t` `vpx_codec_get_frame` returns (its planes and
//! strides, `+0x24`/`+0x34`), and the channel pointers of
//! `vorbis_synthesis_pcmout`.
//!
//! Pointers handed back (the image, its planes, the sample buffers) point
//! at memory these decoders own; they are addresses only in the 32-bit
//! game process. Each stays valid until the next call on the same decoder,
//! as in the originals.

// C-API status returns the player may ignore, and the C names (`m`, `t`, `v`).
#![allow(clippy::must_use_candidate, clippy::many_single_char_names)]

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use ffx_bridge::{Host, Mem};

use crate::vorbis;
use crate::vp8::{self, Frame};

/// The address of a buffer in this (32-bit) process.
fn addr<T>(p: *const T) -> u32 {
    u32::try_from(p as usize).unwrap_or(0)
}

/// `VPX_IMG_FMT_I420`.
const VPX_IMG_FMT_I420: u32 = 0x102;
/// `VPX_CODEC_ERROR` (`vpx_codec_err_t`), for a frame that does not decode.
const VPX_CODEC_ERROR: u32 = 1;
/// `VPX_CODEC_INVALID_PARAM`, for a context this did not set up.
const VPX_CODEC_INVALID_PARAM: u32 = 8;

/// libvpx's allocator table pointer (`vpx_mem.c`'s, `0x022fb508`).
pub const VPX_MEM_FUNCTIONS: u32 = 0x022f_b508;

/// `vpx_mem_set_functions(malloc, calloc, realloc, free, memcpy, memset,
/// memmove)` (`0x0040a5d0`): the table is allocated with `malloc(0x1c)`
/// (`fns[0]`, through the host) when there is none, then the seven
/// pointers stored (`malloc` at `+0`, the others from `+4`); 0, or -1
/// when the allocation fails. Only libvpx read the table; the Rust
/// decoder does not.
pub fn vpx_mem_set_functions(h: &mut impl Host, fns: [u32; 7]) -> u32 {
    let g = h.rebase(VPX_MEM_FUNCTIONS);
    let mut table = h.read::<u32>(g);
    if table == 0 {
        table = h.call(fns[0], &[0x1c]);
        h.write::<u32>(g, table);
        if table == 0 {
            return u32::MAX;
        }
    }
    for (k, f) in fns.iter().enumerate().skip(1) {
        h.write::<u32>(table + 4 * u32::try_from(k).unwrap_or(0), *f);
    }
    h.write::<u32>(table, fns[0]);
    0
}

/// `vpx_mem_unset_functions()` (`0x0040a630`): frees the table with its
/// own `free` (`+0xc`, through the host) and clears the pointer; 0.
pub fn vpx_mem_unset_functions(h: &mut impl Host) -> u32 {
    let g = h.rebase(VPX_MEM_FUNCTIONS);
    let table = h.read::<u32>(g);
    if table != 0 {
        let free = h.read::<u32>(table + 0xc);
        h.call(free, &[table]);
        h.write::<u32>(g, 0);
    }
    0
}

/// One `vpx_codec_ctx_t`'s decoder.
struct Vpx {
    dec: vp8::Decoder,
    /// The frame the last `vpx_codec_decode` produced, until
    /// `vpx_codec_get_frame` takes it.
    pending: Option<Frame>,
    /// The frame `vpx_codec_get_frame` handed out (its planes stay put).
    shown: Option<Frame>,
    /// The `user_priv` of the decode that made `pending`.
    user_priv: u32,
    /// The `vpx_image_t` handed out (0x5c bytes).
    img: Box<[u32; 23]>,
}

static VPX: Mutex<Option<HashMap<u32, Vpx>>> = Mutex::new(None);

fn with_vpx<R>(f: impl FnOnce(&mut HashMap<u32, Vpx>) -> R) -> R {
    let mut g = VPX.lock().unwrap_or_else(PoisonError::into_inner);
    f(g.get_or_insert_with(HashMap::new))
}

/// `vpx_codec_dec_init_ver(ctx, iface, cfg, flags, ver)` (`0x004094c0`):
/// a new decoder for `ctx`. The context is cleared and its `iface` (`+4`),
/// `init_flags` (`+0x10`) and `config.dec` (`+0x14`, the `cfg` pointer)
/// set as the original sets them; 0, or `VPX_CODEC_ABI_MISMATCH` (3) when
/// `ver` is not 9, as the original checks first. (`cfg`'s thread count
/// does not change the output; the player passes none.)
pub fn vpx_dec_init(m: &mut impl Mem, ctx: u32, iface: u32, cfg: u32, flags: u32, ver: u32) -> u32 {
    if ver != 9 {
        return 3;
    }
    if ctx == 0 || iface == 0 {
        return VPX_CODEC_INVALID_PARAM;
    }
    m.write_bytes(ctx, &[0; 0x1c]);
    m.write::<u32>(ctx + 4, iface);
    m.write::<u32>(ctx + 0x10, flags);
    m.write::<u32>(ctx + 0x14, cfg);
    with_vpx(|t| {
        t.insert(
            ctx,
            Vpx {
                dec: vp8::Decoder::new(),
                pending: None,
                shown: None,
                user_priv: 0,
                img: Box::new([0; 23]),
            },
        );
    });
    0
}

/// `vpx_codec_decode(ctx, data, size, user_priv, deadline)`
/// (`0x00409680`): decodes one frame; 0, or an error code when it does
/// not decode (the player ignores it). `user_priv` comes back in the
/// image's `user_priv` (`+0x48`). The `deadline` (0 from the player: the
/// best quality) does not change libvpx's VP8 output without
/// post-processing, which the player does not enable.
pub fn vpx_decode(
    m: &impl Mem,
    ctx: u32,
    data: u32,
    size: u32,
    user_priv: u32,
    _deadline: u32,
) -> u32 {
    let mut bytes = vec![0u8; size as usize];
    if data != 0 {
        m.read_bytes(data, &mut bytes);
    }
    with_vpx(|t| {
        let Some(v) = t.get_mut(&ctx) else {
            return VPX_CODEC_INVALID_PARAM;
        };
        if data == 0 || size == 0 {
            // a flush: VP8 has no delayed frames
            return 0;
        }
        match v.dec.decode(&bytes) {
            Some(f) => {
                v.pending = Some(f);
                v.user_priv = user_priv;
                0
            }
            None => {
                v.pending = None;
                if zvidlib_hidden(&bytes) {
                    0
                } else {
                    VPX_CODEC_ERROR
                }
            }
        }
    })
}

/// Whether a VP8 frame is coded hidden (`show_frame` 0 in its tag).
fn zvidlib_hidden(frame: &[u8]) -> bool {
    frame.first().is_some_and(|b| b & 0x10 == 0)
}

/// `vpx_codec_get_frame(ctx, iter)` (`0x004096e0`): the decoded frame
/// once (`*iter` 0 on entry, set to the image), else 0. The image is a
/// `vpx_image_t`: I420, `d_w`/`d_h` the frame's size, the planes packed
/// (strides = widths).
pub fn vpx_get_frame(m: &mut impl Mem, ctx: u32, iter: u32) -> u32 {
    if iter == 0 || m.read::<u32>(iter) != 0 {
        return 0;
    }
    let img = with_vpx(|t| {
        let v = t.get_mut(&ctx)?;
        let f = v.pending.take()?;
        let (w, h) = (u32::try_from(f.width).ok()?, u32::try_from(f.height).ok()?);
        let img = &mut v.img;
        img.fill(0);
        img[0] = VPX_IMG_FMT_I420;
        img[2] = w;
        img[3] = h;
        img[4] = 8; // bit_depth
        img[5] = w; // d_w
        img[6] = h; // d_h
        img[7] = 1; // x_chroma_shift
        img[8] = 1; // y_chroma_shift
        for (k, p) in f.planes.iter().enumerate() {
            img[9 + k] = addr(p.as_ptr());
        }
        img[13] = w;
        img[14] = w.div_ceil(2);
        img[15] = w.div_ceil(2);
        img[17] = 12; // bps
        img[18] = v.user_priv;
        v.shown = Some(f);
        Some(addr(img.as_ptr()))
    });
    match img {
        Some(a) => {
            m.write::<u32>(iter, a);
            a
        }
        None => 0,
    }
}

/// `vpx_codec_destroy(ctx)` (`0x00409370`): drops the decoder; 0, or
/// `VPX_CODEC_INVALID_PARAM` for an unknown context.
pub fn vpx_destroy(m: &mut impl Mem, ctx: u32) -> u32 {
    let had = with_vpx(|t| t.remove(&ctx).is_some());
    if !had {
        return VPX_CODEC_INVALID_PARAM;
    }
    m.write_bytes(ctx, &[0; 0x1c]);
    0
}

/// One movie's Vorbis decoder, from `vorbis_info_init` to the clears.
struct Vorbis {
    dec: vorbis::Decoder,
    /// `vorbis_dsp_state.pcmret`: the channel pointers `pcmout` hands out.
    pcmret: Vec<u32>,
}

/// The decoders by their `vorbis_info`, and which `vorbis_info` a
/// `vorbis_dsp_state` and a `vorbis_block` belong to.
#[derive(Default)]
struct VorbisTable {
    by_info: HashMap<u32, Vorbis>,
    dsp: HashMap<u32, u32>,
    block: HashMap<u32, u32>,
}

static VORBIS: Mutex<Option<VorbisTable>> = Mutex::new(None);

fn with_vorbis<R>(f: impl FnOnce(&mut VorbisTable) -> R) -> R {
    let mut g = VORBIS.lock().unwrap_or_else(PoisonError::into_inner);
    f(g.get_or_insert_with(VorbisTable::default))
}

/// `vorbis_info` is 0x20 bytes; `vorbis_comment` 0x10.
const INFO_SIZE: usize = 0x20;
const COMMENT_SIZE: usize = 0x10;

/// `vorbis_info_init(vi)` (`0x00a8ffd0`): a cleared `vorbis_info` and a
/// new decoder for it.
pub fn vorbis_info_init(m: &mut impl Mem, vi: u32) {
    m.write_bytes(vi, &[0; INFO_SIZE]);
    with_vorbis(|t| {
        t.by_info.insert(
            vi,
            Vorbis {
                dec: vorbis::Decoder::new(),
                pcmret: Vec::new(),
            },
        );
    });
}

/// `vorbis_comment_init(vc)` (`0x00a90420`): cleared; the comments are
/// not kept (nothing reads them).
pub fn vorbis_comment_init(m: &mut impl Mem, vc: u32) {
    m.write_bytes(vc, &[0; COMMENT_SIZE]);
}

/// An `ogg_packet` (`packet`, `bytes`, `b_o_s`, `e_o_s`, `granulepos`,
/// `packetno`).
struct Packet {
    data: Vec<u8>,
    bytes: i32,
    b_o_s: bool,
    e_o_s: bool,
    granulepos: i64,
    packetno: i64,
}

fn read_packet(m: &impl Mem, op: u32) -> Packet {
    let ptr = m.read::<u32>(op);
    let bytes = m.read::<i32>(op + 4);
    let mut data = vec![0u8; usize::try_from(bytes).unwrap_or(0)];
    if ptr != 0 {
        m.read_bytes(ptr, &mut data);
    }
    Packet {
        data,
        bytes,
        b_o_s: m.read::<i32>(op + 8) != 0,
        e_o_s: m.read::<i32>(op + 0xc) != 0,
        granulepos: m.read::<i64>(op + 0x10),
        packetno: m.read::<i64>(op + 0x18),
    }
}

/// `vorbis_synthesis_headerin(vi, vc, op)` (`0x00a8f650`, `PhyreEngine`'s
/// version, [`vorbis::Decoder::headerin`]): after the identification
/// header the `vorbis_info` fields are filled as the original fills them.
pub fn vorbis_headerin(m: &mut impl Mem, vi: u32, vc: u32, op: u32) -> u32 {
    if op == 0 || vc == 0 {
        return vorbis::OV_EBADHEADER.cast_unsigned();
    }
    let p = read_packet(m, op);
    let r = with_vorbis(|t| {
        let v = t.by_info.get_mut(&vi)?;
        let r = v.dec.headerin(&p.data, p.bytes, p.b_o_s);
        Some((
            r,
            [
                v.dec.version,
                v.dec.channels,
                v.dec.rate,
                v.dec.bitrate_upper,
                v.dec.bitrate_nominal,
                v.dec.bitrate_lower,
            ],
        ))
    });
    let Some((r, fields)) = r else {
        return vorbis::OV_EBADHEADER.cast_unsigned();
    };
    for (k, f) in fields.iter().enumerate() {
        m.write::<i32>(vi + 4 * u32::try_from(k).unwrap_or(0), *f);
    }
    r.cast_unsigned()
}

/// `vorbis_synthesis_init(v, vi)` (`0x00a910e0`): 0, or 1 without a
/// complete setup.
pub fn vorbis_synthesis_init(m: &mut impl Mem, v: u32, vi: u32) -> u32 {
    m.write_bytes(v, &[0; 0x70]);
    m.write::<u32>(v + 4, vi);
    with_vorbis(|t| {
        let Some(d) = t.by_info.get_mut(&vi) else {
            return 1;
        };
        if d.dec.synthesis_init() != 0 {
            return 1;
        }
        t.dsp.insert(v, vi);
        0
    })
}

/// `vorbis_block_init(v, vb)` (`0x00a921b0`): 0.
pub fn vorbis_block_init(m: &mut impl Mem, v: u32, vb: u32) -> u32 {
    m.write_bytes(vb, &[0; 0x70]);
    with_vorbis(|t| {
        t.block.insert(vb, v);
    });
    0
}

/// `vorbis_synthesis(vb, op)` (`0x00a92460`): 0, `OV_ENOTAUDIO` or
/// `OV_EBADPACKET`.
pub fn vorbis_synthesis(m: &impl Mem, vb: u32, op: u32) -> u32 {
    if op == 0 {
        return vorbis::OV_EBADPACKET.cast_unsigned();
    }
    let p = read_packet(m, op);
    with_vorbis(|t| {
        let Some(d) = t
            .block
            .get(&vb)
            .and_then(|v| t.dsp.get(v))
            .and_then(|vi| t.by_info.get_mut(vi))
        else {
            return vorbis::OV_EBADPACKET;
        };
        d.dec
            .synthesis(&p.data, p.bytes, p.granulepos, p.packetno, p.e_o_s)
    })
    .cast_unsigned()
}

/// `vorbis_synthesis_blockin(v, vb)` (`0x00a90a70`): 0 or `OV_EINVAL`
/// (also for a block `v`'s `vorbis_block_init` did not set up).
pub fn vorbis_blockin(v: u32, vb: u32) -> u32 {
    with_vorbis(|t| {
        if t.block.get(&vb) != Some(&v) {
            return vorbis::OV_EINVAL;
        }
        let Some(d) = t.dsp.get(&v).and_then(|vi| t.by_info.get_mut(vi)) else {
            return vorbis::OV_EINVAL;
        };
        d.dec.blockin()
    })
    .cast_unsigned()
}

/// `vorbis_synthesis_pcmout(v, pcm)` (`0x00a90a10`): the samples ready
/// per channel; with `pcm`, `*pcm` = the channel pointers.
pub fn vorbis_pcmout(m: &mut impl Mem, v: u32, pcm: u32) -> u32 {
    let r = with_vorbis(|t| {
        let d = t.dsp.get(&v).and_then(|vi| t.by_info.get_mut(vi))?;
        let n = d.dec.pcmout();
        if n == 0 {
            return Some((0, 0));
        }
        let ch = usize::try_from(d.dec.channels).unwrap_or(0);
        d.pcmret.clear();
        for c in 0..ch {
            d.pcmret.push(addr(d.dec.channel(c).as_ptr()));
        }
        Some((u32::try_from(n).unwrap_or(0), addr(d.pcmret.as_ptr())))
    });
    let Some((n, ret)) = r else {
        return 0;
    };
    if n != 0 && pcm != 0 {
        m.write::<u32>(pcm, ret);
    }
    n
}

/// `vorbis_synthesis_read(v, n)` (`0x00a909e0`): 0 or `OV_EINVAL`.
pub fn vorbis_read(v: u32, n: u32) -> u32 {
    with_vorbis(|t| {
        let Some(d) = t.dsp.get(&v).and_then(|vi| t.by_info.get_mut(vi)) else {
            return vorbis::OV_EINVAL;
        };
        d.dec.read(n as usize)
    })
    .cast_unsigned()
}

/// `vorbis_block_clear(vb)` (`0x00a92040`).
pub fn vorbis_block_clear(m: &mut impl Mem, vb: u32) {
    with_vorbis(|t| t.block.remove(&vb));
    m.write_bytes(vb, &[0; 0x70]);
}

/// `vorbis_dsp_clear(v)` (`0x00a917a0`).
pub fn vorbis_dsp_clear(m: &mut impl Mem, v: u32) {
    with_vorbis(|t| t.dsp.remove(&v));
    m.write_bytes(v, &[0; 0x70]);
}

/// `vorbis_comment_clear(vc)` (`0x00a90030`).
pub fn vorbis_comment_clear(m: &mut impl Mem, vc: u32) {
    m.write_bytes(vc, &[0; COMMENT_SIZE]);
}

/// `vorbis_info_clear(vi)` (`0x00a8fd80`): drops the decoder.
pub fn vorbis_info_clear(m: &mut impl Mem, vi: u32) {
    with_vorbis(|t| {
        t.by_info.remove(&vi);
        t.dsp.retain(|_, i| *i != vi);
    });
    m.write_bytes(vi, &[0; INFO_SIZE]);
}
