//! Compares the Rust movie decoders with the original's output, offline
//! (`docs/formats/platform.md`, "Movies"). The references come from
//! `ffx-oracle`'s `movie_ref` example (FFX.exe's own libvpx and libVorbis
//! in the emulator).
//!
//! ```text
//! moviecmp vorbis <movie.webm> <ref.f32> [<ref.s16>]
//! moviecmp vp8 <movie.webm> <ref.txt>
//! ```
//!
//! `vorbis` feeds the packets as `PVideoPlaybackWin32_AudioRead` does and
//! compares every interleaved f32 sample bit for bit (and the s16 the FMOD
//! stream gets). `vp8` compares the per-frame MD5 of the visible I420
//! planes with the reference's lines (as many frames as it has). Prints
//! one line, `OK` or `DIFF`, and exits 1 on a difference.

#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::format_collect,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)] // a measuring tool

use std::process::ExitCode;

use ffx_movie::vorbis::Decoder;
use ffx_movie::webm::{self, Movie, TRACK_AUDIO, TRACK_VIDEO};

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("usage: moviecmp vorbis|vp8 <movie.webm> <ref> [<ref.s16>]");
        return ExitCode::from(2);
    }
    let name = std::path::Path::new(&a[2])
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ok = match a[1].as_str() {
        "vorbis" => vorbis(&name, &a[2], &a[3], a.get(4).map(String::as_str)),
        "vp8" => vp8(&name, &a[2], &a[3]),
        _ => {
            eprintln!("unknown mode");
            return ExitCode::from(2);
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn vorbis(name: &str, path: &str, ref_f32: &str, ref_s16: Option<&str>) -> bool {
    let mut movie = Movie::open(webm::FileSource::open(path).expect("open")).expect("webm");
    let track = movie.track(TRACK_AUDIO).expect("audio track").clone();
    let headers = webm::xiph_headers(&track.codec_private).expect("headers");
    let mut d = Decoder::new();
    for (i, h) in headers.iter().enumerate() {
        let r = d.headerin(h, i32::try_from(h.len()).unwrap_or(0), true);
        assert!(r >= 0, "header {i}: {r}");
    }
    assert_eq!(d.synthesis_init(), 0);
    let channels = usize::try_from(d.channels).unwrap_or(0);
    let mut out = Vec::new();
    let mut packetno = 0i64;
    for p in movie.packets() {
        let p = p.expect("packet");
        if p.track != track.number {
            continue;
        }
        packetno += 1;
        let len = i32::try_from(p.data.len()).unwrap_or(0);
        if d.synthesis(&p.data, len, 0, packetno, false) == 0 {
            d.blockin();
        }
        let n = d.pcmout();
        for i in 0..n {
            for c in 0..channels {
                out.push(d.channel(c)[i]);
            }
        }
        d.read(n);
    }
    let reference: Vec<f32> = std::fs::read(ref_f32)
        .expect("reference")
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let n = out.len().min(reference.len());
    if let Some(Ok(at)) = std::env::var("MOVIECMP_SHOW")
        .ok()
        .map(|v| v.parse::<usize>())
    {
        for i in at..(at + 8).min(n) {
            println!("{i}: {:e} {:e}", out[i], reference[i]);
        }
    }
    let diff = (0..n)
        .filter(|&i| out[i].to_bits() != reference[i].to_bits())
        .count();
    let first = (0..n).find(|&i| out[i].to_bits() != reference[i].to_bits());
    let s16_diff = ref_s16.map(|p| {
        let r: Vec<i16> = std::fs::read(p)
            .expect("s16 reference")
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        (0..n.min(r.len()))
            .filter(|&i| ffx_movie::vorbis_to_s16(out[i]) != r[i])
            .count()
    });
    let ok = diff == 0 && out.len() == reference.len() && s16_diff.unwrap_or(0) == 0;
    println!(
        "{}\tvorbis\t{name}\tsamples={} ref={}\tf32_diff={diff}\ts16_diff={}\tfirst={first:?}",
        if ok { "OK" } else { "DIFF" },
        out.len(),
        reference.len(),
        s16_diff.map_or_else(|| "-".to_owned(), |d| d.to_string()),
    );
    ok
}

fn vp8(name: &str, path: &str, reference: &str) -> bool {
    let mut movie = Movie::open(webm::FileSource::open(path).expect("open")).expect("webm");
    let track = movie.track(TRACK_VIDEO).expect("video track").number;
    let want: Vec<String> = std::fs::read_to_string(reference)
        .expect("reference")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split_whitespace().nth(1).unwrap_or("").to_owned())
        .collect();
    let mut dec = ffx_movie::vp8::Decoder::new();
    let mut shown = 0usize;
    let mut diff = 0usize;
    let mut first = None;
    for p in movie.packets() {
        let p = p.expect("packet");
        if p.track != track {
            continue;
        }
        if shown >= want.len() {
            break;
        }
        let Some(frame) = dec.decode(&p.data) else {
            continue;
        };
        let md5 = md5_hex(&frame);
        if md5 != want[shown] {
            diff += 1;
            first.get_or_insert(shown);
        }
        shown += 1;
    }
    let ok = diff == 0 && shown == want.len();
    println!(
        "{}\tvp8\t{name}\tframes={shown} ref={}\tdiff={diff}\tfirst={first:?}",
        if ok { "OK" } else { "DIFF" },
        want.len()
    );
    ok
}

/// The MD5 of the Y, U and V planes one after the other, in hex.
fn md5_hex(f: &ffx_movie::vp8::Frame) -> String {
    use md5::{Digest, Md5};
    let mut h = Md5::new();
    for p in &f.planes {
        h.update(p);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
