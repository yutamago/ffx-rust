//! libwebm's `mkvparser` as `PVideoPlaybackWin32` uses it, on
//! [`crate::webm`] (`docs/formats/platform.md`, "Movies"), and the
//! player's demux loop (`PVideoPlaybackWin32_DemuxLoop` `0x00a27650`),
//! whose cluster and block walk is all `mkvparser` calls.
//!
//! `PVideoPlaybackWin32_open` (`0x00a27f20`) and `_AudioInit`
//! (`0x00a273b0`) keep their calls: `Segment::CreateInstance` and `Load`
//! build the segment here, and the objects they hand out are laid out as
//! `mkvparser`'s (32-bit) for the fields the player's accessors read, so
//! the two accessors too short to hook (`Segment::GetTracks` `0x004251b0`,
//! `Track::GetType` `0x00427b10`, four bytes each) read the right values:
//!
//! | object | field |
//! |---|---|
//! | `Segment` | `+0x70` its `SegmentInfo`, `+0x74` its `Tracks` |
//! | `SegmentInfo` | `+0x28` timecode scale (i64), `+0x30` duration (f64, -1 when absent) |
//! | `Tracks` | `+0x28`..`+0x2c` the `Track*` array (begin, end) |
//! | `Track` | `+0x20` type, `+0x58` codec private, `+0x5c` its size |
//! | `VideoTrack` | `+0x90` width, `+0x98` height (i64), `+0xa0` frame rate (f64) |
//! | `AudioTrack` | `+0x90` sampling rate (f64), `+0x98` channels (i64) |
//!
//! The other accessors are hooked onto the same fields. The objects live
//! here (one 4 KiB block per segment) until `Segment` is deleted; their
//! addresses are meaningful only in the 32-bit game process. A host whose
//! game memory is not its own (`ffx-host`'s arena) gives each segment a
//! block of game memory instead ([`create_instance_at`], [`GAME_BLOCK`]
//! bytes): the objects and the codec private data are laid out there,
//! copied in by [`sync`].

#![allow(clippy::must_use_candidate)] // C-API returns the player may ignore

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use ffx_bridge::Mem;

use crate::webm::{Movie, Source, TRACK_AUDIO, TRACK_VIDEO};

/// A source the hook provides (the game's `IMkvReader`).
pub type BoxedSource = Box<dyn Source + Send>;

/// `mkvparser`'s allocator pointers (`0x022fb510` malloc, `0x022fb514`
/// free).
pub const MKV_MALLOC: u32 = 0x022f_b510;
pub const MKV_FREE: u32 = 0x022f_b514;
/// The CRT's `malloc` and `free` import slots `ResetAllocator` restores.
const IMP_MALLOC: u32 = 0x00b0_c468;
const IMP_FREE: u32 = 0x00b0_c370;

/// The fake objects' offsets in a segment's block.
const SEG: usize = 0;
const INFO: usize = 0x100;
const TRACKS: usize = 0x180;
const ARRAY: usize = 0x200;
const TRACK0: usize = 0x300;
const TRACK_SIZE: usize = 0xc0;
const MAX_TRACKS: usize = 8;
const BLOCK: usize = 0x1000;
/// The size of a segment's block in game memory ([`create_instance_at`]):
/// the objects, then the tracks' codec private data from `0x1000` on.
pub const GAME_BLOCK: u32 = 0x1_0000;

struct Segment {
    block: Vec<u8>,
    /// The block's game address, when the host gave one.
    game: Option<u32>,
    src: Option<BoxedSource>,
    movie: Option<Movie<BoxedSource>>,
    /// The tracks' codec private data, with 8 spare zero bytes (the
    /// player's header walk reads 3 past the end).
    private: Vec<Vec<u8>>,
    /// (number, type) per track.
    tracks: Vec<(u64, u64)>,
}

type Shared = Arc<Mutex<Segment>>;

static SEGMENTS: Mutex<Option<HashMap<u32, Shared>>> = Mutex::new(None);

fn segments<R>(f: impl FnOnce(&mut HashMap<u32, Shared>) -> R) -> R {
    let mut g = SEGMENTS.lock().unwrap_or_else(PoisonError::into_inner);
    f(g.get_or_insert_with(HashMap::new))
}

fn lock(s: &Shared) -> std::sync::MutexGuard<'_, Segment> {
    s.lock().unwrap_or_else(PoisonError::into_inner)
}

fn addr<T>(p: *const T) -> u32 {
    u32::try_from(p as usize).unwrap_or(0)
}

impl Segment {
    fn base(&self) -> u32 {
        self.game.unwrap_or_else(|| addr(self.block.as_ptr()))
    }

    fn put<const N: usize>(&mut self, at: usize, bytes: [u8; N]) {
        self.block[at..at + N].copy_from_slice(&bytes);
    }

    /// Lays the loaded movie's objects out (the table above).
    fn lay_out(&mut self) {
        let Some(movie) = self.movie.as_ref() else {
            return;
        };
        let base = self.base();
        let scale = movie.timecode_scale;
        let duration = movie.duration;
        let tracks: Vec<_> = movie.tracks.iter().take(MAX_TRACKS).cloned().collect();
        let o = |at: usize| base + u32::try_from(at).unwrap_or(0);
        self.put(SEG + 0x70, o(INFO).to_le_bytes());
        self.put(SEG + 0x74, o(TRACKS).to_le_bytes());
        self.put(INFO + 0x28, scale.to_le_bytes());
        self.put(INFO + 0x30, duration.to_le_bytes());
        self.put(TRACKS + 0x28, o(ARRAY).to_le_bytes());
        self.put(TRACKS + 0x2c, o(ARRAY + 4 * tracks.len()).to_le_bytes());
        self.private.clear();
        self.tracks.clear();
        // codec private data in the game block, after the objects
        let mut next = BLOCK;
        for (i, t) in tracks.iter().enumerate() {
            let at = TRACK0 + i * TRACK_SIZE;
            self.put(ARRAY + 4 * i, o(at).to_le_bytes());
            #[allow(clippy::cast_possible_truncation)] // TrackType is small
            self.put(at + 0x20, (t.kind as u32).to_le_bytes());
            let mut private = t.codec_private.clone();
            let size = u32::try_from(private.len()).unwrap_or(0);
            private.extend([0; 8]);
            let p = if t.codec_private.is_empty() {
                0
            } else if self.game.is_some() {
                let end = next + private.len();
                if end > self.block.len() {
                    0
                } else {
                    self.block[next..end].copy_from_slice(&private);
                    let at = o(next);
                    next = end.next_multiple_of(16);
                    at
                }
            } else {
                addr(private.as_ptr())
            };
            self.private.push(private);
            self.put(at + 0x58, p.to_le_bytes());
            self.put(at + 0x5c, size.to_le_bytes());
            if t.kind == TRACK_VIDEO {
                self.put(at + 0x90, t.width.to_le_bytes());
                self.put(at + 0x98, t.height.to_le_bytes());
                self.put(at + 0xa0, t.frame_rate.to_le_bytes());
            } else if t.kind == TRACK_AUDIO {
                self.put(at + 0x90, t.sampling_rate.to_le_bytes());
                self.put(at + 0x98, t.channels.to_le_bytes());
            }
            self.tracks.push((t.number, t.kind));
        }
    }
}

/// `mkvparser::SetAllocator(malloc, free)` (`0x0041e5b0`, `PhyreEngine`'s
/// addition): stores them (nothing here allocates through them).
pub fn set_allocator(m: &mut impl Mem, malloc: u32, free: u32) {
    let (a, b) = (m.rebase(MKV_MALLOC), m.rebase(MKV_FREE));
    m.write::<u32>(a, malloc);
    m.write::<u32>(b, free);
}

/// `mkvparser::ResetAllocator()` (`0x0041e5d0`): back to the CRT's
/// (`malloc`/`free` import slots `0x00b0c468`/`0x00b0c370`).
pub fn reset_allocator(m: &mut impl Mem) {
    let malloc = m.read::<u32>(m.rebase(IMP_MALLOC));
    let free = m.read::<u32>(m.rebase(IMP_FREE));
    let (a, b) = (m.rebase(MKV_MALLOC), m.rebase(MKV_FREE));
    m.write::<u32>(a, malloc);
    m.write::<u32>(b, free);
}

/// `Segment::CreateInstance(reader, pos, &segment)` (`0x0041fd50`): a new
/// segment over `src` (the reader), its address into `*out`; 0. The
/// headers are read by [`load`]. The player passes position 0; another
/// start is not supported (-2, `E_FILE_FORMAT_INVALID`, `*out` 0).
pub fn create_instance(m: &mut impl Mem, src: BoxedSource, pos: i64, out: u32) -> i64 {
    create(m, src, pos, out, None)
}

/// [`create_instance`] with the segment's objects in game memory at
/// `game` ([`GAME_BLOCK`] bytes the host allocated; [`sync`] fills them,
/// [`delete`] gives the address back to free).
pub fn create_instance_at(
    m: &mut impl Mem,
    src: BoxedSource,
    pos: i64,
    out: u32,
    game: u32,
) -> i64 {
    create(m, src, pos, out, Some(game))
}

fn create(m: &mut impl Mem, src: BoxedSource, pos: i64, out: u32, game: Option<u32>) -> i64 {
    if pos != 0 {
        if out != 0 {
            m.write::<u32>(out, 0);
        }
        return -2;
    }
    let seg = Segment {
        block: vec![
            0;
            if game.is_some() {
                GAME_BLOCK as usize
            } else {
                BLOCK
            }
        ],
        game,
        src: Some(src),
        movie: None,
        private: Vec::new(),
        tracks: Vec::new(),
    };
    let base = seg.base();
    segments(|t| t.insert(base, Arc::new(Mutex::new(seg))));
    if out != 0 {
        m.write::<u32>(out, base);
    }
    0
}

fn find(seg: u32) -> Option<Shared> {
    segments(|t| t.get(&seg).cloned())
}

/// `Segment::Load()` (`0x00421450`): reads the EBML header, `Info` and
/// `Tracks`; 0, or -2 (`E_FILE_FORMAT_INVALID`) when they do not parse.
pub fn load(seg: u32) -> i64 {
    let Some(s) = find(seg) else {
        return -2;
    };
    let mut s = lock(&s);
    let Some(src) = s.src.take() else {
        return 0;
    };
    match Movie::open(src) {
        Ok(movie) => {
            s.movie = Some(movie);
            s.lay_out();
            0
        }
        Err(_) => -2,
    }
}

/// Copies the segment's objects into its game block ([`create_instance_at`];
/// nothing for a segment without one). After [`load`].
pub fn sync(m: &mut impl Mem, seg: u32) {
    if let Some(s) = find(seg) {
        let s = lock(&s);
        if let Some(at) = s.game {
            m.write_bytes(at, &s.block);
        }
    }
}

/// `Segment::~Segment` + free (`0x0041fd10`, `delete segment`); the game
/// block to free, if the host gave one.
pub fn delete(seg: u32) -> Option<u32> {
    segments(|t| t.remove(&seg)).and_then(|s| lock(&s).game)
}

/// `Segment::GetDuration()` (`0x00425200`): the duration times the
/// timecode scale in ns, truncated; -1 without one.
pub fn duration(m: &impl Mem, seg: u32) -> i64 {
    let info = m.read::<u32>(seg + 0x70);
    if info == 0 {
        return -1;
    }
    let d = m.read::<f64>(info + 0x30);
    if d < 0.0 {
        return -1;
    }
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let ns = (d * m.read::<i64>(info + 0x28) as f64) as i64;
    ns
}

/// `Tracks::GetTracksCount()` (`0x004294c0`).
pub fn tracks_count(m: &impl Mem, tracks: u32) -> u32 {
    let (b, e) = (m.read::<u32>(tracks + 0x28), m.read::<u32>(tracks + 0x2c));
    e.wrapping_sub(b) / 4
}

/// `Tracks::GetTrackByIndex(i)` (`0x00429de0`): 0 past the end.
pub fn track_by_index(m: &impl Mem, tracks: u32, i: u32) -> u32 {
    if i >= tracks_count(m, tracks) {
        return 0;
    }
    m.read::<u32>(m.read::<u32>(tracks + 0x28) + 4 * i)
}

/// `Track::GetCodecPrivate(size&)` (`0x00427b80`): the data, its size into
/// `*size`.
pub fn codec_private(m: &mut impl Mem, track: u32, size: u32) -> u32 {
    let n = m.read::<u32>(track + 0x5c);
    m.write::<u32>(size, n);
    m.read::<u32>(track + 0x58)
}

/// `VideoTrack::GetWidth`/`GetHeight` (`0x00428db0`/`0x00428dc0`) and
/// `AudioTrack::GetChannels` (`0x00429180`): an i64 field.
pub fn field_i64(m: &impl Mem, obj: u32, off: u32) -> i64 {
    m.read::<i64>(obj + off)
}

/// `VideoTrack::GetFrameRate` (`0x00428dd0`, `+0xa0`) and
/// `AudioTrack::GetSamplingRate` (`0x00429170`, `+0x90`): a double, in
/// `ST0`.
pub fn field_f64(m: &impl Mem, obj: u32, off: u32) -> f64 {
    m.read::<f64>(obj + off)
}

/// The player's frame queues (`CDataFrameQueue`), the calls the demux
/// loop makes on them.
pub trait Queues {
    /// `CDataFrameQueue_Reserve(len)` (`0x00a3dec0`): a slot (its first
    /// word holds `len`, the data follows), waiting while the queue is
    /// full; 0 when it is closed.
    fn reserve(&mut self, queue: u32, len: u32) -> u32;
    /// `CDataFrameQueue_Commit()` (`0x00a3df00`).
    fn commit(&mut self, queue: u32);
    /// `CDataFrameQueue_Close()` (`0x00a3dea0`).
    fn close(&mut self, queue: u32);
    /// `Phyre_PEventFlagWin32_signal()` (`0x00430200`).
    fn signal(&mut self, event: u32);
}

/// `PVideoPlaybackWin32_DemuxLoop(player)` (`0x00a27650`, the reading
/// thread): demux state (`+0xe8`) 1; every frame of every block, cluster
/// by cluster, into the video queue (`player`, track type 1) or the audio
/// queue (`+0x80`, type 2, when the player has an audio track `+0x294`);
/// from the first cluster again while looping (`+0xf0`); stops when the
/// player's state (`+0xe4`) is 3. Then both queues closed, demux state 2,
/// the done event (`+0x2b4`) signalled. A cluster that does not parse ends
/// the loop at once, as `Cluster::GetNext`'s error does (the original
/// prints "Error parsing next block of cluster" and leaves the queues
/// open). Frames of unknown tracks, and audio without an audio track, are
/// skipped (the original reads the latter into a scratch buffer).
pub fn demux_loop(m: &mut impl Mem, player: u32, q: &mut impl Queues) {
    m.write::<u32>(player + 0xe8, 1);
    let seg = m.read::<u32>(player + 0x110);
    let state = find(seg);
    'outer: loop {
        if let Some(s) = &state {
            let (mut pos, tracks) = {
                let g = lock(s);
                (
                    g.movie.as_ref().and_then(Movie::first_cluster),
                    g.tracks.clone(),
                )
            };
            while let Some(p) = pos {
                if m.read::<u32>(player + 0xe4) == 3 {
                    break;
                }
                let read = lock(s).movie.as_mut().map(|mv| mv.read_cluster(p));
                let Some(Ok((packets, next))) = read else {
                    return;
                };
                for pk in packets {
                    if m.read::<u32>(player + 0xe4) == 3 {
                        break;
                    }
                    let Some(&(_, kind)) = tracks.iter().find(|(n, _)| *n == pk.track) else {
                        continue;
                    };
                    let queue = match kind {
                        TRACK_VIDEO => player,
                        TRACK_AUDIO if m.read::<u32>(player + 0x294) != 0 => player + 0x80,
                        _ => continue,
                    };
                    let len = u32::try_from(pk.data.len()).unwrap_or(0);
                    let slot = q.reserve(queue, len);
                    if slot == 0 {
                        continue;
                    }
                    m.write_bytes(slot + 4, &pk.data);
                    q.commit(queue);
                }
                pos = next;
            }
        }
        if m.read::<u8>(player + 0xf0) == 0 || m.read::<u32>(player + 0xe4) == 3 || state.is_none()
        {
            break 'outer;
        }
    }
    q.close(player);
    q.close(player + 0x80);
    m.write::<u32>(player + 0xe8, 2);
    q.signal(player + 0x2b4);
}
