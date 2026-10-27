//! The original movie decoders' output, by running FFX.exe's own libvpx
//! 1.4.0 and libVorbis 1.3.5 in the oracle (`docs/formats/platform.md`,
//! "Movies"): the reference the Rust decoders are compared with
//! (`ffx-movie`'s `moviecmp` example).
//!
//! ```text
//! movie_ref vp8 <movie.webm> <max frames> <out.txt>
//! movie_ref vorbis <movie.webm> <max packets> <out.s16> [<out.f32>]
//! ```
//!
//! `vp8` writes `index md5` per shown frame, the MD5 of the visible Y, U
//! and V planes one after the other (what `PVideoPlaybackWin32_DecodeLoop`
//! copies into its queue slot). It decodes as `PVideoPlaybackWin32_open`
//! sets the decoder up: `vpx_codec_dec_init_ver(ctx, vpx_codec_vp8_dx(),
//! NULL, 0, 9)`, then per packet `vpx_codec_decode(ctx, data, size, NULL,
//! 0)` and one `vpx_codec_get_frame`.
//!
//! `vorbis` feeds the headers and packets as `PVideoPlaybackWin32_AudioInit`
//! and `PVideoPlaybackWin32_AudioRead` do (`b_o_s` 1, `e_o_s` 0, granule
//! position 0, packet numbers 0 for the headers and 1, 2, ... for the audio)
//! and writes the interleaved samples: s16 after the game's conversion, and
//! optionally the f32 that `vorbis_synthesis_pcmout` gives.
//!
//! Needs `FFX_GAME_DIR`. A maximum of 0 means all.

#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::format_collect,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)] // a measuring tool

use std::io::Write;
use std::path::PathBuf;

use ffx_movie::webm::{self, Movie, TRACK_AUDIO, TRACK_VIDEO};
use ffx_oracle::{Oracle, Runtime};
use md5::{Digest, Md5};

const VPX_CODEC_DEC_INIT_VER: u32 = 0x0040_94c0;
const VPX_CODEC_VP8_DX: u32 = 0x0040_a430;
const VPX_MEM_SET_FUNCTIONS: u32 = 0x0040_a5d0;
const VPX_CODEC_DECODE: u32 = 0x0040_9680;
const VPX_CODEC_GET_FRAME: u32 = 0x0040_96e0;

const VORBIS_INFO_INIT: u32 = 0x00a8_ffd0;
const VORBIS_COMMENT_INIT: u32 = 0x00a9_0420;
const VORBIS_SYNTHESIS_HEADERIN: u32 = 0x00a8_f650;
const VORBIS_SYNTHESIS_INIT: u32 = 0x00a9_10e0;
const VORBIS_BLOCK_INIT: u32 = 0x00a9_21b0;
const VORBIS_SYNTHESIS: u32 = 0x00a9_2460;
const VORBIS_SYNTHESIS_BLOCKIN: u32 = 0x00a9_0a70;
const VORBIS_SYNTHESIS_PCMOUT: u32 = 0x00a9_0a10;
const VORBIS_SYNTHESIS_READ: u32 = 0x00a9_09e0;

/// The largest packet the one reused buffer holds.
const PACKET_BUF: usize = 0x40_0000;

fn word(o: &mut Oracle, a: u32) -> u32 {
    u32::from_le_bytes(o.read(a, 4).unwrap().try_into().unwrap())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() >= 5,
        "usage: movie_ref vp8|vorbis <movie.webm> <max> <out> [<out.f32>]"
    );
    let dir = PathBuf::from(std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR"));
    let mut o = Oracle::load_with(dir.join("FFX.exe"), Runtime::Dll).unwrap();
    o.set_fpu_control(0x027f).unwrap();
    o.set_max_instructions(2_000_000_000);
    let src = webm::FileSource::open(&args[2]).unwrap();
    let mut movie = Movie::open(src).unwrap();
    let max: usize = args[3].parse().unwrap();
    let max = if max == 0 { usize::MAX } else { max };
    let out = std::fs::File::create(&args[4]).unwrap();
    let mut out = std::io::BufWriter::new(out);
    match args[1].as_str() {
        "vp8" => vp8(&mut o, &mut movie, max, &mut out),
        "vorbis" => {
            let f32_out = args
                .get(5)
                .map(|p| std::io::BufWriter::new(std::fs::File::create(p).unwrap()));
            vorbis(&mut o, &mut movie, max, &mut out, f32_out);
        }
        m => panic!("unknown mode {m}"),
    }
}

fn vp8(o: &mut Oracle, movie: &mut Movie<webm::FileSource>, max: usize, out: &mut impl Write) {
    let track = movie.track(TRACK_VIDEO).expect("video track").number;
    let ctx = o.alloc(0x100).unwrap();
    let iter = o.alloc(4).unwrap();
    let buf = o.alloc(PACKET_BUF).unwrap();
    // the game hands libvpx Phyre's heap (`vpx_mem_set_functions`); here
    // the CRT's, and its own copy callbacks
    let heap: Vec<u32> = ["malloc", "calloc", "realloc", "free"]
        .iter()
        .map(|n| o.import_stub(n).unwrap_or_else(|| panic!("no {n} stub")))
        .collect();
    let copies = [0x00a2_6fb0, 0x00a2_6fd0, 0x00a2_6fc0].map(|a| o.rebase(a));
    let mut set = heap;
    set.extend(copies);
    assert_eq!(
        o.call_cdecl(o.rebase(VPX_MEM_SET_FUNCTIONS), &set).unwrap(),
        0
    );
    let iface = o.call_cdecl(o.rebase(VPX_CODEC_VP8_DX), &[]).unwrap();
    let r = o
        .call_cdecl(o.rebase(VPX_CODEC_DEC_INIT_VER), &[ctx, iface, 0, 0, 9])
        .unwrap();
    assert_eq!(r, 0, "vpx_codec_dec_init_ver");
    let mut shown = 0usize;
    for p in movie.packets() {
        let p = p.unwrap();
        if p.track != track {
            continue;
        }
        assert!(p.data.len() <= PACKET_BUF);
        o.write(buf, &p.data).unwrap();
        let len = u32::try_from(p.data.len()).unwrap();
        let r = o
            .call_cdecl(o.rebase(VPX_CODEC_DECODE), &[ctx, buf, len, 0, 0])
            .unwrap();
        o.write(iter, &[0; 4]).unwrap();
        let img = o
            .call_cdecl(o.rebase(VPX_CODEC_GET_FRAME), &[ctx, iter])
            .unwrap();
        if img == 0 {
            if r != 0 {
                writeln!(out, "# packet error {r}").unwrap();
            }
            continue;
        }
        let (w, h) = (word(o, img + 0x14), word(o, img + 0x18));
        let mut md5 = Md5::new();
        for plane in 0..3 {
            let base = word(o, img + 0x24 + 4 * plane);
            let stride = word(o, img + 0x34 + 4 * plane);
            let (pw, ph) = if plane == 0 {
                (w, h)
            } else {
                (w.div_ceil(2), h.div_ceil(2))
            };
            for y in 0..ph {
                md5.update(o.read(base + y * stride, pw as usize).unwrap());
            }
        }
        let hex: String = md5.finalize().iter().map(|b| format!("{b:02x}")).collect();
        writeln!(out, "{shown} {hex}").unwrap();
        o.clear_dirty();
        shown += 1;
        if shown >= max {
            break;
        }
    }
}

fn vorbis(
    o: &mut Oracle,
    movie: &mut Movie<webm::FileSource>,
    max: usize,
    out: &mut impl Write,
    mut f32_out: Option<impl Write>,
) {
    let track = movie.track(TRACK_AUDIO).expect("audio track").clone();
    let headers = webm::xiph_headers(&track.codec_private).unwrap();
    let channels = u32::try_from(track.channels).unwrap();
    let vi = o.alloc(0x40).unwrap();
    let vc = o.alloc(0x20).unwrap();
    let vd = o.alloc(0x100).unwrap();
    let vb = o.alloc(0x100).unwrap();
    let op = o.alloc(0x20).unwrap();
    let pcm = o.alloc(4).unwrap();
    let buf = o.alloc(PACKET_BUF).unwrap();
    o.call_cdecl(o.rebase(VORBIS_INFO_INIT), &[vi]).unwrap();
    o.call_cdecl(o.rebase(VORBIS_COMMENT_INIT), &[vc]).unwrap();
    let set_packet = |o: &mut Oracle, data: &[u8], packetno: u32| {
        o.write(buf, data).unwrap();
        let mut p = Vec::with_capacity(0x20);
        p.extend(buf.to_le_bytes());
        p.extend(u32::try_from(data.len()).unwrap().to_le_bytes());
        p.extend(1u32.to_le_bytes()); // b_o_s
        p.extend(0u32.to_le_bytes()); // e_o_s
        p.extend(0u64.to_le_bytes()); // granulepos
        p.extend(u64::from(packetno).to_le_bytes());
        o.write(op, &p).unwrap();
    };
    // PVideoPlaybackWin32_AudioInit's walk: Phyre's vorbis_synthesis_headerin
    // returns the bytes it read; the first header starts at +3 (one-byte
    // lace sizes), the third 4 bytes after the end the second reports
    let private = &track.codec_private;
    let mut at = 3usize;
    let mut left = private.len();
    for (i, h) in headers.iter().enumerate() {
        let xiph = h.as_ptr() as usize - private.as_ptr() as usize;
        assert_eq!(
            at, xiph,
            "header {i}: the game's offset differs from the Xiph lacing"
        );
        let mut bytes = private[at..].to_vec();
        bytes.resize(left, 0);
        set_packet(o, &bytes, 0);
        let r = o
            .call_cdecl(o.rebase(VORBIS_SYNTHESIS_HEADERIN), &[vi, vc, op])
            .unwrap();
        assert!(r < 0x8000_0000, "vorbis_synthesis_headerin {i}: {r:#x}");
        let step = r as usize + if i == 1 { 4 } else { 0 };
        at += step;
        left -= step;
    }
    assert_eq!(
        o.call_cdecl(o.rebase(VORBIS_SYNTHESIS_INIT), &[vd, vi])
            .unwrap(),
        0
    );
    assert_eq!(
        o.call_cdecl(o.rebase(VORBIS_BLOCK_INIT), &[vd, vb])
            .unwrap(),
        0
    );
    let mut packetno = 0u32;
    let mut total = 0u64;
    for p in movie.packets() {
        let p = p.unwrap();
        if p.track != track.number {
            continue;
        }
        packetno += 1;
        set_packet(o, &p.data, packetno);
        if o.call_cdecl(o.rebase(VORBIS_SYNTHESIS), &[vb, op]).unwrap() == 0 {
            o.call_cdecl(o.rebase(VORBIS_SYNTHESIS_BLOCKIN), &[vd, vb])
                .unwrap();
        }
        loop {
            let n = o
                .call_cdecl(o.rebase(VORBIS_SYNTHESIS_PCMOUT), &[vd, pcm])
                .unwrap();
            if n == 0 || n > 0x10_0000 {
                break;
            }
            let chans = word(o, pcm);
            let ptrs: Vec<u32> = (0..channels).map(|c| word(o, chans + 4 * c)).collect();
            let planes: Vec<Vec<f32>> = ptrs
                .iter()
                .map(|&a| {
                    o.read(a, 4 * n as usize)
                        .unwrap()
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                        .collect()
                })
                .collect();
            let mut s16 = Vec::with_capacity(2 * planes.len() * n as usize);
            let mut fl = Vec::with_capacity(4 * planes.len() * n as usize);
            for i in 0..n as usize {
                for pl in &planes {
                    s16.extend(ffx_movie::vorbis_to_s16(pl[i]).to_le_bytes());
                    fl.extend(pl[i].to_le_bytes());
                }
            }
            out.write_all(&s16).unwrap();
            if let Some(f) = f32_out.as_mut() {
                f.write_all(&fl).unwrap();
            }
            total += u64::from(n);
            o.call_cdecl(o.rebase(VORBIS_SYNTHESIS_READ), &[vd, n])
                .unwrap();
        }
        o.clear_dirty();
        if packetno as usize >= max {
            break;
        }
    }
    eprintln!("{packetno} packets, {total} samples per channel");
}
