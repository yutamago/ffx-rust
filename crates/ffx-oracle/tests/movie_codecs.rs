//! The movie codecs against FFX.exe's own (`docs/formats/platform.md`,
//! "Movies"): libvpx 1.4.0's VP8 frames against `ffx_movie::vp8`,
//! libVorbis 1.3.5's headers and samples against `ffx_movie::vorbis`, and
//! the one libvpx function ported over game memory (the allocator table).
//! The movies come from `FFX_Data.vbf`. Needs `FFX_GAME_DIR`; skips
//! without it. The whole movies are compared offline
//! (`ffx-oracle`'s `movie_ref` and `ffx-movie`'s `moviecmp` examples).

#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::many_single_char_names,
    clippy::too_many_lines
)] // the C names; one walk per test

use std::path::PathBuf;

use ffx_movie::webm::{Movie, TRACK_AUDIO, TRACK_VIDEO};
use ffx_oracle::{Oracle, OracleHost};

const VPX_MEM_SET_FUNCTIONS: u32 = 0x0040_a5d0;
const VPX_MEM_UNSET_FUNCTIONS: u32 = 0x0040_a630;
const VPX_CODEC_DEC_INIT_VER: u32 = 0x0040_94c0;
const VPX_CODEC_VP8_DX: u32 = 0x0040_a430;
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

fn game_dir() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir))
}

fn oracle(dir: &std::path::Path) -> Oracle {
    let mut o = Oracle::load(dir.join("FFX.exe")).unwrap();
    o.set_fpu_control(0x027f).unwrap();
    o.set_max_instructions(1_000_000_000);
    o
}

fn movie(dir: &std::path::Path, name: &str) -> Vec<u8> {
    let archive = ffx_vbf::Archive::open(dir.join("data/FFX_Data.vbf")).unwrap();
    let path = format!("ffx_data/gamedata/ps3data/video/{name}");
    let entry = archive
        .find(&path)
        .unwrap_or_else(|| panic!("{path} not in the archive"));
    archive.read(entry).unwrap()
}

fn word(o: &mut Oracle, a: u32) -> u32 {
    u32::from_le_bytes(o.read(a, 4).unwrap().try_into().unwrap())
}

/// `vpx_mem_set_functions` and `vpx_mem_unset_functions`: results and
/// every byte written, the table allocated and freed through the CRT.
#[test]
fn vpx_mem_functions_match_the_original() {
    let Some(dir) = game_dir() else { return };
    let mut a = oracle(&dir);
    let mut b = oracle(&dir);
    let malloc = a.import_stub("malloc").unwrap();
    let free = a.import_stub("free").unwrap();
    let fns = [
        malloc,
        0x1111_0001,
        0x1111_0002,
        free,
        0x1111_0004,
        0x1111_0005,
        0x1111_0006,
    ];
    for round in 0..2 {
        // twice: the second call finds the table and only stores
        for _ in 0..=round {
            let want = a.call_cdecl(a.rebase(VPX_MEM_SET_FUNCTIONS), &fns).unwrap();
            let got = ffx_movie::api::vpx_mem_set_functions(&mut OracleHost(&mut b), fns);
            assert_eq!(got, want, "vpx_mem_set_functions");
            assert_eq!(
                a.diff(&mut b).unwrap(),
                vec![],
                "vpx_mem_set_functions memory"
            );
        }
        let want = a
            .call_cdecl(a.rebase(VPX_MEM_UNSET_FUNCTIONS), &[])
            .unwrap();
        let got = ffx_movie::api::vpx_mem_unset_functions(&mut OracleHost(&mut b));
        assert_eq!(got, want, "vpx_mem_unset_functions");
        assert_eq!(
            a.diff(&mut b).unwrap(),
            vec![],
            "vpx_mem_unset_functions memory"
        );
    }
    // unset with no table: nothing to free
    let want = a
        .call_cdecl(a.rebase(VPX_MEM_UNSET_FUNCTIONS), &[])
        .unwrap();
    let got = ffx_movie::api::vpx_mem_unset_functions(&mut OracleHost(&mut b));
    assert_eq!(got, want);
    assert_eq!(a.diff(&mut b).unwrap(), vec![]);
}

/// The first frames of a texture video (640 x 480): the visible I420
/// planes of every shown frame, byte for byte.
#[test]
fn vp8_frames_match_the_original() {
    let Some(dir) = game_dir() else { return };
    let mut o = oracle(&dir);
    let bytes = movie(&dir, "4_480.webm");
    let mut m = Movie::open(&bytes[..]).unwrap();
    let track = m.track(TRACK_VIDEO).unwrap().number;
    // the original, set up as PVideoPlaybackWin32_open sets it up (with the
    // CRT's heap in place of Phyre's)
    let heap: Vec<u32> = ["malloc", "calloc", "realloc", "free"]
        .iter()
        .map(|n| o.import_stub(n).unwrap())
        .collect();
    let mut set = heap;
    set.extend([0x00a2_6fb0, 0x00a2_6fd0, 0x00a2_6fc0].map(|a| o.rebase(a)));
    assert_eq!(
        o.call_cdecl(o.rebase(VPX_MEM_SET_FUNCTIONS), &set).unwrap(),
        0
    );
    let ctx = o.alloc(0x40).unwrap();
    let iter = o.alloc(4).unwrap();
    let buf = o.alloc(0x10_0000).unwrap();
    let iface = o.call_cdecl(o.rebase(VPX_CODEC_VP8_DX), &[]).unwrap();
    let r = o
        .call_cdecl(o.rebase(VPX_CODEC_DEC_INIT_VER), &[ctx, iface, 0, 0, 9])
        .unwrap();
    assert_eq!(r, 0);
    let mut port = ffx_movie::vp8::Decoder::new();
    let mut shown = 0;
    for p in m.packets().take(12) {
        let p = p.unwrap();
        if p.track != track {
            continue;
        }
        o.write(buf, &p.data).unwrap();
        let len = u32::try_from(p.data.len()).unwrap();
        o.call_cdecl(o.rebase(VPX_CODEC_DECODE), &[ctx, buf, len, 0, 0])
            .unwrap();
        o.write(iter, &[0; 4]).unwrap();
        let img = o
            .call_cdecl(o.rebase(VPX_CODEC_GET_FRAME), &[ctx, iter])
            .unwrap();
        let frame = port.decode(&p.data);
        assert_eq!(img != 0, frame.is_some(), "shown or not");
        let Some(frame) = frame else { continue };
        let (w, h) = (word(&mut o, img + 0x14), word(&mut o, img + 0x18));
        assert_eq!((w as usize, h as usize), (frame.width, frame.height));
        for (k, plane) in frame.planes.iter().enumerate() {
            let k32 = u32::try_from(k).unwrap();
            let base = word(&mut o, img + 0x24 + 4 * k32);
            let stride = word(&mut o, img + 0x34 + 4 * k32);
            let (pw, ph) = if k == 0 {
                (w, h)
            } else {
                (w.div_ceil(2), h.div_ceil(2))
            };
            for y in 0..ph {
                let row = o.read(base + y * stride, pw as usize).unwrap();
                let at = (y * pw) as usize;
                assert_eq!(
                    row,
                    plane[at..at + pw as usize],
                    "frame {shown} plane {k} row {y}"
                );
            }
        }
        o.clear_dirty();
        shown += 1;
    }
    assert!(shown >= 10, "{shown} frames compared");
}

/// A movie's sound: the header calls (`PhyreEngine`'s byte counts, the
/// `vorbis_info` fields the player reads) and the samples of its first
/// packets, bit for bit.
#[test]
fn vorbis_matches_the_original() {
    let Some(dir) = game_dir() else { return };
    let mut o = oracle(&dir);
    let bytes = movie(&dir, "bsc_jp.webm");
    let mut m = Movie::open(&bytes[..]).unwrap();
    let track = m.track(TRACK_AUDIO).unwrap().clone();
    let private = track.codec_private.clone();
    let (vi, vc, vd, vb) = (
        o.alloc(0x40).unwrap(),
        o.alloc(0x20).unwrap(),
        o.alloc(0x100).unwrap(),
        o.alloc(0x100).unwrap(),
    );
    let (op, pcm, buf) = (
        o.alloc(0x20).unwrap(),
        o.alloc(4).unwrap(),
        o.alloc(0x10_0000).unwrap(),
    );
    o.call_cdecl(o.rebase(VORBIS_INFO_INIT), &[vi]).unwrap();
    o.call_cdecl(o.rebase(VORBIS_COMMENT_INIT), &[vc]).unwrap();
    let set_packet = |o: &mut Oracle, data: &[u8], bytes: usize, packetno: u32| {
        o.write(buf, data).unwrap();
        let mut p = Vec::new();
        p.extend(buf.to_le_bytes());
        p.extend(u32::try_from(bytes).unwrap().to_le_bytes());
        p.extend(1u32.to_le_bytes());
        p.extend(0u32.to_le_bytes());
        p.extend(0u64.to_le_bytes());
        p.extend(u64::from(packetno).to_le_bytes());
        o.write(op, &p).unwrap();
    };
    // PVideoPlaybackWin32_AudioInit's walk over the CodecPrivate
    let mut port = ffx_movie::vorbis::Decoder::new();
    let (mut at, mut left) = (3usize, private.len());
    for i in 0..3 {
        let mut data = private[at..].to_vec();
        data.resize(left, 0);
        set_packet(&mut o, &data, left, 0);
        let want = o
            .call_cdecl(o.rebase(VORBIS_SYNTHESIS_HEADERIN), &[vi, vc, op])
            .unwrap();
        let got = port.headerin(&data, i32::try_from(left).unwrap(), true);
        assert_eq!(got.cast_unsigned(), want, "header {i}");
        let step = want as usize + if i == 1 { 4 } else { 0 };
        at += step;
        left -= step;
    }
    let info: Vec<u32> = (0..6).map(|k| word(&mut o, vi + 4 * k)).collect();
    let fields = [
        port.version,
        port.channels,
        port.rate,
        port.bitrate_upper,
        port.bitrate_nominal,
        port.bitrate_lower,
    ];
    assert_eq!(info, fields.map(i32::cast_unsigned), "vorbis_info");
    assert_eq!(
        o.call_cdecl(o.rebase(VORBIS_SYNTHESIS_INIT), &[vd, vi])
            .unwrap(),
        0
    );
    assert_eq!(port.synthesis_init(), 0);
    assert_eq!(
        o.call_cdecl(o.rebase(VORBIS_BLOCK_INIT), &[vd, vb])
            .unwrap(),
        0
    );
    let channels = info[1];
    let mut packetno = 0u32;
    let mut samples = 0;
    for p in m.packets() {
        let p = p.unwrap();
        if p.track != track.number {
            continue;
        }
        packetno += 1;
        if packetno > 100_000 {
            break;
        }
        set_packet(&mut o, &p.data, p.data.len(), packetno);
        let want = o.call_cdecl(o.rebase(VORBIS_SYNTHESIS), &[vb, op]).unwrap();
        let len = i32::try_from(p.data.len()).unwrap();
        let got = port.synthesis(&p.data, len, 0, i64::from(packetno), false);
        assert_eq!(got.cast_unsigned(), want, "vorbis_synthesis {packetno}");
        if want == 0 {
            let want = o
                .call_cdecl(o.rebase(VORBIS_SYNTHESIS_BLOCKIN), &[vd, vb])
                .unwrap();
            assert_eq!(port.blockin().cast_unsigned(), want, "blockin {packetno}");
        }
        let n = o
            .call_cdecl(o.rebase(VORBIS_SYNTHESIS_PCMOUT), &[vd, pcm])
            .unwrap();
        assert_eq!(port.pcmout(), n as usize, "pcmout {packetno}");
        if n != 0 {
            let chans = word(&mut o, pcm);
            for c in 0..channels {
                let at = word(&mut o, chans + 4 * c);
                let want: Vec<u32> = o
                    .read(at, 4 * n as usize)
                    .unwrap()
                    .chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                let got: Vec<u32> = port
                    .channel(c as usize)
                    .iter()
                    .map(|f| f.to_bits())
                    .collect();
                assert_eq!(got, want, "packet {packetno} channel {c}");
            }
            samples += n;
        }
        assert_eq!(
            o.call_cdecl(o.rebase(VORBIS_SYNTHESIS_READ), &[vd, n])
                .unwrap(),
            0
        );
        assert_eq!(port.read(n as usize), 0);
        o.clear_dirty();
    }
    assert!(samples > 20_000, "{samples} samples compared");
}
