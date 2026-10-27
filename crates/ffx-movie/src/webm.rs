//! The `WebM` demuxer: what the game reads with libwebm's `mkvparser`
//! (`0x0041e5b0..0x0042e1d0`, `docs/formats/platform.md` "Movies").
//!
//! `PVideoPlaybackWin32_open` (`0x00a27f20`) loads the segment
//! (`Segment::CreateInstance`, `Segment::Load`), takes the duration
//! (`Segment::GetDuration`: the `Info` duration times the timecode scale,
//! in ns), the tracks by index (`Tracks::GetTrackByIndex`, `Track::GetType`
//! 1 video, 2 audio) and the video's size and frame rate.
//! `PVideoPlaybackWin32_AudioInit` (`0x00a273b0`) takes the audio track's
//! codec private data (the three Vorbis headers, Xiph-laced), its sampling
//! rate and channels. `PVideoPlaybackWin32_DemuxLoop` (`0x00a27650`) walks
//! the clusters (`Segment::GetFirst`/`GetNext`), each cluster's block
//! entries (`Cluster::GetFirst`/`GetNext` until `BlockEntry::EOS`), and
//! hands every frame of every block (`Block::GetFrameCount`,
//! `Block::GetFrame`) to the video or audio queue by its track's type.
//!
//! Only what those calls give is read here: the element tree is EBML with
//! the Matroska ids below. The source is read piecewise (the largest movie
//! is 618 MiB); a cluster at a time is held in memory.

use std::fmt;

/// Where the bytes come from: the file, at an offset.
pub trait Source {
    /// The size in bytes.
    fn len(&self) -> u64;
    /// Whether the source is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Fills `buf` from `pos`; fails on a short read.
    ///
    /// # Errors
    /// The source's own read errors, or [`Error::Truncated`].
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<(), Error>;
}

impl<S: Source + ?Sized> Source for Box<S> {
    fn len(&self) -> u64 {
        (**self).len()
    }

    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<(), Error> {
        (**self).read_at(pos, buf)
    }
}

impl Source for &[u8] {
    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }

    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<(), Error> {
        let start = usize::try_from(pos).map_err(|_| Error::Truncated)?;
        let end = start.checked_add(buf.len()).ok_or(Error::Truncated)?;
        buf.copy_from_slice(self.get(start..end).ok_or(Error::Truncated)?);
        Ok(())
    }
}

/// A file opened as a source.
pub struct FileSource {
    file: std::fs::File,
    len: u64,
}

impl FileSource {
    /// Opens `path`.
    ///
    /// # Errors
    /// The file system's.
    pub fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

impl Source for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<(), Error> {
        use std::io::{Read, Seek, SeekFrom};
        self.file
            .seek(SeekFrom::Start(pos))
            .and_then(|_| self.file.read_exact(buf))
            .map_err(|e| Error::Io(e.to_string()))
    }
}

/// What can go wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The source's read failed.
    Io(String),
    /// An element runs past its parent or the end of the file.
    Truncated,
    /// Not an EBML `webm`/`matroska` file, or a required element is missing.
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "read error: {e}"),
            Self::Truncated => f.write_str("truncated element"),
            Self::Invalid(what) => write!(f, "invalid WebM: {what}"),
        }
    }
}

impl std::error::Error for Error {}

const EBML: u32 = 0x1a45_dfa3;
const DOC_TYPE: u32 = 0x4282;
const SEGMENT: u32 = 0x1853_8067;
const INFO: u32 = 0x1549_a966;
const TIMECODE_SCALE: u32 = 0x002a_d7b1;
const DURATION: u32 = 0x4489;
const TRACKS: u32 = 0x1654_ae6b;
const TRACK_ENTRY: u32 = 0xae;
const TRACK_NUMBER: u32 = 0xd7;
const TRACK_TYPE: u32 = 0x83;
const CODEC_ID: u32 = 0x86;
const CODEC_PRIVATE: u32 = 0x63a2;
const VIDEO: u32 = 0xe0;
const PIXEL_WIDTH: u32 = 0xb0;
const PIXEL_HEIGHT: u32 = 0xba;
const FRAME_RATE: u32 = 0x0023_83e3;
const AUDIO: u32 = 0xe1;
const SAMPLING_FREQUENCY: u32 = 0xb5;
const CHANNELS: u32 = 0x9f;
const CLUSTER: u32 = 0x1f43_b675;
const TIMECODE: u32 = 0xe7;
const SIMPLE_BLOCK: u32 = 0xa3;
const BLOCK_GROUP: u32 = 0xa0;
const BLOCK: u32 = 0xa1;

/// An element size of all ones: unknown (to the end of the parent).
const UNKNOWN: u64 = u64::MAX;

/// One track (`mkvparser::Track`, `VideoTrack`, `AudioTrack`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Track {
    /// `TrackNumber`, what blocks refer to.
    pub number: u64,
    /// `TrackType`: 1 video, 2 audio.
    pub kind: u64,
    /// `CodecID` (`V_VP8`, `A_VORBIS`).
    pub codec_id: String,
    /// `CodecPrivate`: for Vorbis, the three headers, Xiph-laced.
    pub codec_private: Vec<u8>,
    /// `PixelWidth`, `PixelHeight` (video).
    pub width: u64,
    pub height: u64,
    /// `FrameRate` (video; 0 when absent, as `VideoTrack::GetFrameRate`).
    pub frame_rate: f64,
    /// `SamplingFrequency` (audio; 8000 when absent, as mkvparser).
    pub sampling_rate: f64,
    /// `Channels` (audio; 1 when absent).
    pub channels: u64,
}

/// Track types.
pub const TRACK_VIDEO: u64 = 1;
pub const TRACK_AUDIO: u64 = 2;

/// One frame of a block: the payload a codec gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    /// The block's track number.
    pub track: u64,
    /// The cluster's timecode plus the block's, in timecode units.
    pub time: i64,
    /// The frame's bytes.
    pub data: Vec<u8>,
}

/// A loaded segment: its info and tracks, and where its clusters are.
pub struct Movie<S: Source> {
    src: S,
    /// `TimecodeScale` in ns (1000000 when absent).
    pub timecode_scale: u64,
    /// `Duration` in timecode units (absent: negative, as mkvparser's -1).
    pub duration: f64,
    /// The tracks in file order (`Tracks::GetTrackByIndex`).
    pub tracks: Vec<Track>,
    /// The segment's payload range.
    segment: (u64, u64),
    /// The first cluster's offset.
    first_cluster: Option<u64>,
}

/// Reads an EBML variable-length integer at `pos` of `src` (`len` bytes
/// left): (value, its size). With `id`, the length marker stays in.
fn vint(src: &mut impl Source, pos: u64, end: u64, id: bool) -> Result<(u64, u64), Error> {
    let mut first = [0u8; 1];
    if pos >= end {
        return Err(Error::Truncated);
    }
    src.read_at(pos, &mut first)?;
    let n = first[0].leading_zeros() + 1;
    if n > 8 {
        return Err(Error::Invalid("vint length"));
    }
    let n64 = u64::from(n);
    if pos + n64 > end {
        return Err(Error::Truncated);
    }
    let mut rest = [0u8; 8];
    src.read_at(pos, &mut rest[..n as usize])?;
    let mut v = 0u64;
    for &b in &rest[..n as usize] {
        v = v << 8 | u64::from(b);
    }
    if id {
        return Ok((v, n64));
    }
    let mask = if n == 8 {
        u64::MAX >> 8
    } else {
        (1u64 << (7 * n)) - 1
    };
    let v = v & mask;
    Ok((if v == mask { UNKNOWN } else { v }, n64))
}

/// An element header at `pos`: (id, payload start, payload end).
fn element(src: &mut impl Source, pos: u64, end: u64) -> Result<(u32, u64, u64), Error> {
    let (id, n) = vint(src, pos, end, true)?;
    let (size, m) = vint(src, pos + n, end, false)?;
    let start = pos + n + m;
    let stop = if size == UNKNOWN {
        end
    } else {
        start.checked_add(size).ok_or(Error::Truncated)?
    };
    if stop > end {
        return Err(Error::Truncated);
    }
    Ok((
        u32::try_from(id).map_err(|_| Error::Invalid("id"))?,
        start,
        stop,
    ))
}

fn read_bytes(src: &mut impl Source, start: u64, stop: u64) -> Result<Vec<u8>, Error> {
    let len = usize::try_from(stop - start).map_err(|_| Error::Truncated)?;
    let mut v = vec![0; len];
    src.read_at(start, &mut v)?;
    Ok(v)
}

fn be_uint(b: &[u8]) -> u64 {
    b.iter().fold(0, |v, &x| v << 8 | u64::from(x))
}

fn be_float(b: &[u8]) -> Result<f64, Error> {
    match b.len() {
        4 => Ok(f64::from(f32::from_be_bytes([b[0], b[1], b[2], b[3]]))),
        8 => Ok(f64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ])),
        _ => Err(Error::Invalid("float size")),
    }
}

/// An EBML vint in a byte slice: (value, size).
fn vint_slice(b: &[u8], id: bool) -> Result<(u64, usize), Error> {
    let first = *b.first().ok_or(Error::Truncated)?;
    let n = first.leading_zeros() as usize + 1;
    if n > 8 || b.len() < n {
        return Err(Error::Truncated);
    }
    let v = be_uint(&b[..n]);
    if id {
        return Ok((v, n));
    }
    let mask = if n == 8 {
        u64::MAX >> 8
    } else {
        (1u64 << (7 * n)) - 1
    };
    let v = v & mask;
    Ok((if v == mask { UNKNOWN } else { v }, n))
}

/// The elements of a byte slice: (id, payload).
fn slice_children(mut b: &[u8]) -> Result<Vec<(u32, &[u8])>, Error> {
    let mut out = Vec::new();
    while !b.is_empty() {
        let (id, n) = vint_slice(b, true)?;
        let (size, m) = vint_slice(&b[n..], false)?;
        let rest = &b[n + m..];
        let size = if size == UNKNOWN {
            rest.len()
        } else {
            usize::try_from(size).map_err(|_| Error::Truncated)?
        };
        let payload = rest.get(..size).ok_or(Error::Truncated)?;
        out.push((
            u32::try_from(id).map_err(|_| Error::Invalid("id"))?,
            payload,
        ));
        b = &rest[size..];
    }
    Ok(out)
}

fn parse_track(b: &[u8]) -> Result<Track, Error> {
    let mut t = Track {
        sampling_rate: 8000.0,
        channels: 1,
        ..Track::default()
    };
    for (id, p) in slice_children(b)? {
        match id {
            TRACK_NUMBER => t.number = be_uint(p),
            TRACK_TYPE => t.kind = be_uint(p),
            CODEC_ID => String::from_utf8_lossy(p)
                .trim_end_matches('\0')
                .clone_into(&mut t.codec_id),
            CODEC_PRIVATE => t.codec_private = p.to_vec(),
            VIDEO => {
                for (id, p) in slice_children(p)? {
                    match id {
                        PIXEL_WIDTH => t.width = be_uint(p),
                        PIXEL_HEIGHT => t.height = be_uint(p),
                        FRAME_RATE => t.frame_rate = be_float(p)?,
                        _ => {}
                    }
                }
            }
            AUDIO => {
                for (id, p) in slice_children(p)? {
                    match id {
                        SAMPLING_FREQUENCY => t.sampling_rate = be_float(p)?,
                        CHANNELS => t.channels = be_uint(p),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(t)
}

/// The frames of one `Block`/`SimpleBlock` payload, with lacing undone.
fn block_frames(b: &[u8], cluster_time: i64, out: &mut Vec<Packet>) -> Result<(), Error> {
    let (track, n) = vint_slice(b, false)?;
    let rest = b.get(n..).ok_or(Error::Truncated)?;
    if rest.len() < 3 {
        return Err(Error::Truncated);
    }
    let rel = i64::from(i16::from_be_bytes([rest[0], rest[1]]));
    let flags = rest[2];
    let time = cluster_time + rel;
    let mut body = &rest[3..];
    let lacing = (flags >> 1) & 3;
    if lacing == 0 {
        out.push(Packet {
            track,
            time,
            data: body.to_vec(),
        });
        return Ok(());
    }
    let count = usize::from(*body.first().ok_or(Error::Truncated)?) + 1;
    body = &body[1..];
    let mut sizes = Vec::with_capacity(count);
    match lacing {
        1 => {
            // Xiph: each size but the last as a run of 255s and a final byte.
            for _ in 0..count - 1 {
                let mut s = 0usize;
                loop {
                    let x = *body.first().ok_or(Error::Truncated)?;
                    body = &body[1..];
                    s += usize::from(x);
                    if x != 255 {
                        break;
                    }
                }
                sizes.push(s);
            }
        }
        3 => {
            // EBML: the first size, then signed differences.
            let (first, k) = vint_slice(body, false)?;
            body = &body[k..];
            let mut s = usize::try_from(first).map_err(|_| Error::Truncated)?;
            sizes.push(s);
            for _ in 1..count - 1 {
                let (raw, k) = vint_slice(body, false)?;
                body = &body[k..];
                let bias = (1i64 << (7 * k - 1)) - 1;
                let diff = i64::try_from(raw).map_err(|_| Error::Truncated)? - bias;
                s = usize::try_from(i64::try_from(s).map_err(|_| Error::Truncated)? + diff)
                    .map_err(|_| Error::Invalid("EBML lace size"))?;
                sizes.push(s);
            }
        }
        _ => {
            // fixed
            let each = body.len() / count;
            sizes.resize(count - 1, each);
        }
    }
    let used: usize = sizes.iter().sum();
    sizes.push(body.len().checked_sub(used).ok_or(Error::Truncated)?);
    for s in sizes {
        let (frame, rest) = body.split_at_checked(s).ok_or(Error::Truncated)?;
        out.push(Packet {
            track,
            time,
            data: frame.to_vec(),
        });
        body = rest;
    }
    Ok(())
}

impl<S: Source> Movie<S> {
    /// Reads the EBML header, the segment's `Info` and `Tracks`, and finds
    /// the first cluster (`Segment::CreateInstance` + `Segment::Load`).
    ///
    /// # Errors
    /// Read errors, or a file that is not `WebM`.
    pub fn open(mut src: S) -> Result<Self, Error> {
        let len = src.len();
        let (id, s, e) = element(&mut src, 0, len)?;
        if id != EBML {
            return Err(Error::Invalid("no EBML header"));
        }
        let header = read_bytes(&mut src, s, e)?;
        let doc = slice_children(&header)?
            .into_iter()
            .find(|(id, _)| *id == DOC_TYPE)
            .map(|(_, p)| p.to_vec())
            .unwrap_or_default();
        if doc != b"webm" && doc != b"matroska" {
            return Err(Error::Invalid("doc type"));
        }
        let (id, seg_start, seg_end) = element(&mut src, e, len)?;
        if id != SEGMENT {
            return Err(Error::Invalid("no segment"));
        }
        let mut movie = Self {
            src,
            timecode_scale: 1_000_000,
            duration: -1.0,
            tracks: Vec::new(),
            segment: (seg_start, seg_end),
            first_cluster: None,
        };
        let mut pos = seg_start;
        while pos < seg_end {
            let (id, s, e) = element(&mut movie.src, pos, seg_end)?;
            match id {
                INFO => {
                    let b = read_bytes(&mut movie.src, s, e)?;
                    for (id, p) in slice_children(&b)? {
                        match id {
                            TIMECODE_SCALE => movie.timecode_scale = be_uint(p),
                            DURATION => movie.duration = be_float(p)?,
                            _ => {}
                        }
                    }
                }
                TRACKS => {
                    let b = read_bytes(&mut movie.src, s, e)?;
                    for (id, p) in slice_children(&b)? {
                        if id == TRACK_ENTRY {
                            movie.tracks.push(parse_track(p)?);
                        }
                    }
                }
                CLUSTER => {
                    movie.first_cluster = Some(pos);
                    break;
                }
                _ => {}
            }
            pos = e;
        }
        Ok(movie)
    }

    /// `Segment::GetDuration`: the duration in ns (`duration` x
    /// `timecode_scale`, truncated), -1 when there is none.
    #[must_use]
    pub fn duration_ns(&self) -> i64 {
        if self.duration < 0.0 {
            return -1;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        {
            (self.duration * self.timecode_scale as f64) as i64
        }
    }

    /// The first track of a type.
    #[must_use]
    pub fn track(&self, kind: u64) -> Option<&Track> {
        self.tracks.iter().find(|t| t.kind == kind)
    }

    /// Every packet of every cluster, in file order.
    #[must_use]
    pub fn packets(&mut self) -> Packets<'_, S> {
        Packets {
            pos: self.first_cluster,
            pending: std::collections::VecDeque::new(),
            movie: self,
        }
    }

    /// The first cluster's offset (`Segment::GetFirst`), `None` without
    /// clusters.
    #[must_use]
    pub fn first_cluster(&self) -> Option<u64> {
        self.first_cluster
    }

    /// The packets of the element at `pos` (none unless it is a cluster)
    /// and the next element's offset, `None` at the segment's end
    /// (`Segment::GetNext`).
    ///
    /// # Errors
    /// Read errors and malformed elements.
    pub fn read_cluster(&mut self, pos: u64) -> Result<(Vec<Packet>, Option<u64>), Error> {
        let mut out = Vec::new();
        let next = self.cluster(pos, &mut out)?;
        Ok((out, (next < self.segment.1).then_some(next)))
    }

    /// The packets of the cluster at `pos`, and the next element's offset.
    fn cluster(&mut self, pos: u64, out: &mut Vec<Packet>) -> Result<u64, Error> {
        let end = self.segment.1;
        let (id, s, e) = element(&mut self.src, pos, end)?;
        if id != CLUSTER {
            return Ok(e);
        }
        let b = read_bytes(&mut self.src, s, e)?;
        let mut time = 0i64;
        for (id, p) in slice_children(&b)? {
            match id {
                TIMECODE => {
                    time = i64::try_from(be_uint(p)).map_err(|_| Error::Invalid("timecode"))?;
                }
                SIMPLE_BLOCK => block_frames(p, time, out)?,
                BLOCK_GROUP => {
                    for (id, q) in slice_children(p)? {
                        if id == BLOCK {
                            block_frames(q, time, out)?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(e)
    }
}

/// The packet iterator ([`Movie::packets`]).
pub struct Packets<'a, S: Source> {
    movie: &'a mut Movie<S>,
    pos: Option<u64>,
    pending: std::collections::VecDeque<Packet>,
}

impl<S: Source> Iterator for Packets<'_, S> {
    type Item = Result<Packet, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(p) = self.pending.pop_front() {
                return Some(Ok(p));
            }
            let pos = self.pos?;
            if pos >= self.movie.segment.1 {
                self.pos = None;
                return None;
            }
            let mut out = Vec::new();
            match self.movie.cluster(pos, &mut out) {
                Ok(next) => {
                    self.pos = Some(next);
                    self.pending.extend(out);
                }
                Err(e) => {
                    self.pos = None;
                    return Some(Err(e));
                }
            }
        }
    }
}

/// The three Vorbis headers of a Xiph-laced `CodecPrivate`.
///
/// # Errors
/// [`Error::Invalid`] when the lacing is not two sizes and three headers.
pub fn xiph_headers(private: &[u8]) -> Result<[&[u8]; 3], Error> {
    let (&count, mut b) = private.split_first().ok_or(Error::Truncated)?;
    if count != 2 {
        return Err(Error::Invalid("Vorbis CodecPrivate lacing"));
    }
    let mut sizes = [0usize; 2];
    for s in &mut sizes {
        loop {
            let (&x, rest) = b.split_first().ok_or(Error::Truncated)?;
            b = rest;
            *s += usize::from(x);
            if x != 255 {
                break;
            }
        }
    }
    let (a, b) = b.split_at_checked(sizes[0]).ok_or(Error::Truncated)?;
    let (c, d) = b.split_at_checked(sizes[1]).ok_or(Error::Truncated)?;
    Ok([a, c, d])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xiph_lacing() {
        let mut v = vec![2u8, 3, 255, 1];
        v.extend([1, 2, 3]);
        v.extend(std::iter::repeat_n(9, 256));
        v.extend([7, 7]);
        let [a, b, c] = xiph_headers(&v).unwrap();
        assert_eq!(a, [1, 2, 3]);
        assert_eq!(b.len(), 256);
        assert_eq!(c, [7, 7]);
    }

    #[test]
    fn simple_block_lacing() {
        // track 1, time +2, Xiph lacing with three frames of 2, 1, 3 bytes
        let b = [0x81, 0, 2, 0x02, 2, 2, 1, 1, 1, 2, 3, 3, 3];
        let mut out = Vec::new();
        block_frames(&b, 10, &mut out).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].data, [1, 1]);
        assert_eq!(out[1].data, [2]);
        assert_eq!(out[2].data, [3, 3, 3]);
        assert_eq!(out[2].time, 12);
    }
}
