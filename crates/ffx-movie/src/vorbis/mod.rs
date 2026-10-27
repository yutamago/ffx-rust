//! The movie sound's Vorbis decoder: a port of the libVorbis 1.3.5
//! decoder linked into FFX.exe (`0x00a8ee40..0x00aa51e0`, with libogg's
//! `bitwise.c`), giving the same float samples bit for bit.
//!
//! libVorbis is Copyright (c) 2002-2015 Xiph.org Foundation, BSD-3-Clause;
//! the notice and licence are in `crates/ffx-movie/NOTICE`. The port
//! follows the C function by function (`synthesis.c`, `block.c`,
//! `info.c`, `codebook.c`, `sharedbook.c`, `floor1.c`, `res0.c`,
//! `mapping0.c`, `mdct.c`, `window.c`) and computes floats as the game's
//! MSVC build does (SSE2 scalar, so each float operation rounds to float,
//! and expressions with a double operand in double).
//!
//! `PhyreEngine` changed the header calls: `vorbis_synthesis_headerin`
//! returns the bytes each header used (the identification header 30, the
//! comment header 4 less than its size), which
//! `PVideoPlaybackWin32_AudioInit` (`0x00a273b0`) uses to step through the
//! `CodecPrivate`, and the identification and setup headers have no
//! framing-bit check ([`Decoder::headerin`]). Floor type 0 is not
//! implemented: no movie uses it, and the setup fails as the original does
//! for an unknown type.
//!
//! The API mirrors the C one the player calls: [`Decoder::headerin`] three
//! times, [`Decoder::synthesis_init`], then per packet
//! [`Decoder::synthesis`] and [`Decoder::blockin`], and
//! [`Decoder::pcmout`] / [`Decoder::read`] for the samples.

// The port keeps libvorbis' names (single letters, `n0`/`n1`), its float
// constants as written in `mdct.h`/`os.h`, its index loops and its
// `x * 0.5` (`HALVE`, not a midpoint: the original's rounding).
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::excessive_precision,
    clippy::approx_constant,
    clippy::manual_midpoint,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

pub mod bits;
pub mod book;
#[allow(clippy::excessive_precision, clippy::unreadable_literal)]
pub mod tables;

use bits::Reader;
use book::{Book, StaticBook, ilog};

/// libvorbis error codes.
pub const OV_EINVAL: i32 = -131;
pub const OV_ENOTVORBIS: i32 = -132;
pub const OV_EBADHEADER: i32 = -133;
pub const OV_ENOTAUDIO: i32 = -135;
pub const OV_EBADPACKET: i32 = -136;

const VIF_POSIT: usize = 63;

/// `vorbis_info_floor1`.
#[derive(Debug, Clone, Default)]
struct Floor1 {
    partitions: i32,
    partitionclass: Vec<i32>,
    class_dim: [i32; 16],
    class_subs: [i32; 16],
    class_book: [i32; 16],
    class_subbook: [[i32; 8]; 16],
    mult: i32,
    postlist: Vec<i32>,
    // floor1_look
    forward_index: Vec<usize>,
    loneighbor: Vec<usize>,
    hineighbor: Vec<usize>,
    posts: usize,
    quant_q: i32,
}

/// `vorbis_info_residue0` with its look.
#[derive(Debug, Clone, Default)]
struct Residue {
    kind: i32,
    begin: i32,
    end: i32,
    grouping: i32,
    partitions: i32,
    partvals: i32,
    groupbook: usize,
    secondstages: Vec<i32>,
    // res0_look
    partbooks: Vec<Vec<Option<usize>>>,
    stages: i32,
    decodemap: Vec<Vec<i32>>,
}

/// `vorbis_info_mapping0`.
#[derive(Debug, Clone, Default)]
struct Mapping {
    submaps: i32,
    chmuxlist: Vec<i32>,
    floorsubmap: Vec<usize>,
    residuesubmap: Vec<usize>,
    coupling_mag: Vec<usize>,
    coupling_ang: Vec<usize>,
}

/// `vorbis_info_mode`.
#[derive(Debug, Clone, Copy, Default)]
struct Mode {
    blockflag: i32,
    mapping: usize,
}

/// `mdct_lookup`.
#[derive(Debug, Clone, Default)]
struct Mdct {
    n: usize,
    log2n: i32,
    trig: Vec<f32>,
    bitrev: Vec<i32>,
}

/// The block being decoded (`vorbis_block`).
#[derive(Debug, Clone, Default)]
struct Block {
    pcm: Vec<Vec<f32>>,
    pcmend: usize,
    w: usize,
    granulepos: i64,
    sequence: i64,
    eofflag: bool,
    ready: bool,
}

/// `vorbis_info` + `codec_setup_info` + `vorbis_dsp_state` + its
/// `private_state`, and the one `vorbis_block` the player uses.
#[derive(Debug, Clone, Default)]
pub struct Decoder {
    // vorbis_info
    pub version: i32,
    pub channels: i32,
    pub rate: i32,
    pub bitrate_upper: i32,
    pub bitrate_nominal: i32,
    pub bitrate_lower: i32,
    blocksizes: [usize; 2],
    books: Vec<Book>,
    floors: Vec<Floor1>,
    residues: Vec<Residue>,
    maps: Vec<Mapping>,
    modes: Vec<Mode>,
    modebits: i32,
    headers: u32,
    // dsp
    transform: [Mdct; 2],
    window: [usize; 2],
    pcm: Vec<Vec<f32>>,
    pcm_returned: i64,
    pcm_current: i64,
    center_w: usize,
    lw: usize,
    w: usize,
    granulepos: i64,
    sequence: i64,
    sample_count: i64,
    eofflag: bool,
    block: Block,
    initialized: bool,
}

/// `icomp`-sorted order of a list of unique values.
fn sorted_order(v: &[i32]) -> Vec<usize> {
    let mut o: Vec<usize> = (0..v.len()).collect();
    o.sort_by_key(|&i| v[i]);
    o
}

fn floor1_unpack(books: usize, opb: &mut Reader) -> Option<Floor1> {
    let mut info = Floor1 {
        partitions: opb.read(5),
        ..Floor1::default()
    };
    let mut maxclass = -1;
    for _ in 0..info.partitions {
        let c = opb.read(4);
        if c < 0 {
            return None;
        }
        maxclass = maxclass.max(c);
        info.partitionclass.push(c);
    }
    let books = i32::try_from(books).ok()?;
    for j in 0..usize::try_from(maxclass + 1).ok()? {
        info.class_dim[j] = opb.read(3) + 1;
        info.class_subs[j] = opb.read(2);
        if info.class_subs[j] < 0 {
            return None;
        }
        if info.class_subs[j] != 0 {
            info.class_book[j] = opb.read(8);
        }
        if info.class_book[j] < 0 || info.class_book[j] >= books {
            return None;
        }
        for k in 0..(1usize << info.class_subs[j]) {
            info.class_subbook[j][k] = opb.read(8) - 1;
            if info.class_subbook[j][k] < -1 || info.class_subbook[j][k] >= books {
                return None;
            }
        }
    }
    info.mult = opb.read(2) + 1;
    let rangebits = opb.read(4);
    if rangebits < 0 {
        return None;
    }
    info.postlist = vec![0, 1 << rangebits];
    let mut count = 0usize;
    for &pc in &info.partitionclass {
        count += usize::try_from(info.class_dim[usize::try_from(pc).ok()?]).ok()?;
        if count > VIF_POSIT {
            return None;
        }
        while info.postlist.len() < count + 2 {
            let t = opb.read(rangebits);
            if t < 0 || t >= (1 << rangebits) {
                return None;
            }
            info.postlist.push(t);
        }
    }
    let order = sorted_order(&info.postlist);
    for w in order.windows(2) {
        if info.postlist[w[0]] == info.postlist[w[1]] {
            return None;
        }
    }
    // floor1_look
    let n = info.postlist.len();
    info.posts = n;
    info.forward_index = order;
    info.quant_q = match info.mult {
        1 => 256,
        2 => 128,
        3 => 86,
        _ => 64,
    };
    for i in 0..n - 2 {
        let (mut lo, mut hi) = (0usize, 1usize);
        let (mut lx, mut hx) = (0, info.postlist[1]);
        let currentx = info.postlist[i + 2];
        for j in 0..i + 2 {
            let x = info.postlist[j];
            if x > lx && x < currentx {
                lo = j;
                lx = x;
            }
            if x < hx && x > currentx {
                hi = j;
                hx = x;
            }
        }
        info.loneighbor.push(lo);
        info.hineighbor.push(hi);
    }
    Some(info)
}

fn icount(v: i32) -> i32 {
    v.cast_unsigned().count_ones().cast_signed()
}

fn res0_unpack(kind: i32, statics: &[StaticBook], opb: &mut Reader) -> Option<Residue> {
    let mut info = Residue {
        kind,
        begin: opb.read(24),
        end: opb.read(24),
        grouping: opb.read(24) + 1,
        partitions: opb.read(6) + 1,
        ..Residue::default()
    };
    let groupbook = opb.read(8);
    if groupbook < 0 {
        return None;
    }
    let mut acc = 0;
    for _ in 0..info.partitions {
        let mut cascade = opb.read(3);
        let cflag = opb.read(1);
        if cflag < 0 {
            return None;
        }
        if cflag != 0 {
            let c = opb.read(5);
            if c < 0 {
                return None;
            }
            cascade |= c << 3;
        }
        info.secondstages.push(cascade);
        acc += icount(cascade);
    }
    let mut booklist = Vec::new();
    for _ in 0..acc {
        let b = opb.read(8);
        if b < 0 {
            return None;
        }
        booklist.push(usize::try_from(b).ok()?);
    }
    let groupbook = usize::try_from(groupbook).ok()?;
    if groupbook >= statics.len() {
        return None;
    }
    for &b in &booklist {
        if b >= statics.len() || statics[b].maptype == 0 {
            return None;
        }
    }
    let entries = statics[groupbook].entries;
    let mut dim = statics[groupbook].dim;
    if dim < 1 {
        return None;
    }
    let mut partvals = 1i32;
    while dim > 0 {
        partvals = partvals.wrapping_mul(info.partitions);
        if partvals > entries {
            return None;
        }
        dim -= 1;
    }
    info.partvals = partvals;
    info.groupbook = groupbook;
    // res0_look
    let dim = statics[groupbook].dim;
    let mut acc = 0usize;
    let mut maxstage = 0;
    for &ss in &info.secondstages {
        let stages = ilog(ss.cast_unsigned());
        let mut parts = Vec::new();
        if stages != 0 {
            maxstage = maxstage.max(stages);
            for k in 0..stages {
                if ss & (1 << k) != 0 {
                    parts.push(Some(booklist[acc]));
                    acc += 1;
                } else {
                    parts.push(None);
                }
            }
        }
        info.partbooks.push(parts);
    }
    info.stages = maxstage;
    let mut look_partvals = 1i32;
    for _ in 0..dim {
        look_partvals = look_partvals.wrapping_mul(info.partitions);
    }
    for j in 0..look_partvals {
        let mut val = j;
        let mut mult = look_partvals / info.partitions;
        let mut map = Vec::new();
        for _ in 0..dim {
            let deco = val / mult;
            val -= deco * mult;
            mult /= info.partitions;
            map.push(deco);
        }
        info.decodemap.push(map);
    }
    Some(info)
}

fn mapping0_unpack(
    channels: i32,
    floors: usize,
    residues: usize,
    opb: &mut Reader,
) -> Option<Mapping> {
    let mut info = Mapping::default();
    if channels <= 0 {
        return None;
    }
    let b = opb.read(1);
    if b < 0 {
        return None;
    }
    info.submaps = if b != 0 {
        let s = opb.read(4) + 1;
        if s <= 0 {
            return None;
        }
        s
    } else {
        1
    };
    let b = opb.read(1);
    if b < 0 {
        return None;
    }
    if b != 0 {
        let steps = opb.read(8) + 1;
        if steps <= 0 {
            return None;
        }
        let bits = ilog((channels - 1).cast_unsigned());
        for _ in 0..steps {
            let m = opb.read(bits);
            let a = opb.read(bits);
            if m < 0 || a < 0 || m == a || m >= channels || a >= channels {
                return None;
            }
            info.coupling_mag.push(usize::try_from(m).ok()?);
            info.coupling_ang.push(usize::try_from(a).ok()?);
        }
    }
    if opb.read(2) != 0 {
        return None;
    }
    info.chmuxlist = vec![0; usize::try_from(channels).ok()?];
    if info.submaps > 1 {
        for c in &mut info.chmuxlist {
            *c = opb.read(4);
            if *c >= info.submaps || *c < 0 {
                return None;
            }
        }
    }
    for _ in 0..info.submaps {
        opb.read(8);
        let f = opb.read(8);
        if f < 0 || usize::try_from(f).ok()? >= floors {
            return None;
        }
        info.floorsubmap.push(usize::try_from(f).ok()?);
        let r = opb.read(8);
        if r < 0 || usize::try_from(r).ok()? >= residues {
            return None;
        }
        info.residuesubmap.push(usize::try_from(r).ok()?);
    }
    Some(info)
}

impl Mdct {
    /// `mdct_init`: the trig tables. libvorbis' `os.h` defines `M_PI` as
    /// the float `3.1415926536f` (MSVC's `math.h` has none), so each angle
    /// is computed in float (`(M_PI / n) * (4 * i)`), then `cos`/`sin` in
    /// double, stored as float (`0x00a9d680`). The game calls MSVCR110's
    /// `cos`/`sin`; `libm`'s give the same floats (checked against the
    /// tables the original builds, oracle test `vorbis_mdct_tables`).
    fn new(n: usize) -> Self {
        const PI: f32 = 3.141_592_653_6;
        let n2 = n >> 1;
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let log2n = (f64::from(n as f32).ln() / 2f64.ln() + 0.5).floor() as i32;
        let mut t = vec![0f32; n + n / 4];
        #[allow(clippy::cast_precision_loss)]
        let (nf, n2f) = (n as f32, (2 * n) as f32);
        let (step, half_step) = (PI / nf, PI / n2f);
        let cos = |a: f32| libm::cos(f64::from(a));
        let sin = |a: f32| libm::sin(f64::from(a));
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        for i in 0..n / 4 {
            let a = step * (4 * i) as f32;
            t[i * 2] = cos(a) as f32;
            t[i * 2 + 1] = (-sin(a)) as f32;
            let b = half_step * (2 * i + 1) as f32;
            t[n2 + i * 2] = cos(b) as f32;
            t[n2 + i * 2 + 1] = sin(b) as f32;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        for i in 0..n / 8 {
            let a = step * (4 * i + 2) as f32;
            t[n + i * 2] = (cos(a) * 0.5) as f32;
            t[n + i * 2 + 1] = (-sin(a) * 0.5) as f32;
        }
        let mask = (1 << (log2n - 1)) - 1;
        let msb = 1 << (log2n - 2);
        let mut bitrev = vec![0i32; n / 4];
        for i in 0..n / 8 {
            let mut acc = 0;
            let mut j = 0;
            while msb >> j != 0 {
                if (msb >> j) & i32::try_from(i).unwrap_or(0) != 0 {
                    acc |= 1 << j;
                }
                j += 1;
            }
            bitrev[i * 2] = ((!acc) & mask) - 1;
            bitrev[i * 2 + 1] = acc;
        }
        Self {
            n,
            log2n,
            trig: t,
            bitrev,
        }
    }
}

const PI3_8: f32 = 0.382_683_432_365_089_771_75;
const PI2_8: f32 = 0.707_106_781_186_547_524_41;
const PI1_8: f32 = 0.923_879_532_511_286_756_13;

fn butterfly_8(x: &mut [f32]) {
    let r0 = x[6] + x[2];
    let r1 = x[6] - x[2];
    let r2 = x[4] + x[0];
    let r3 = x[4] - x[0];
    x[6] = r0 + r2;
    x[4] = r0 - r2;
    let r0 = x[5] - x[1];
    let r2 = x[7] - x[3];
    x[0] = r1 + r0;
    x[2] = r1 - r0;
    let r0 = x[5] + x[1];
    let r1 = x[7] + x[3];
    x[3] = r2 + r3;
    x[1] = r2 - r3;
    x[7] = r1 + r0;
    x[5] = r1 - r0;
}

fn butterfly_16(x: &mut [f32]) {
    let r0 = x[1] - x[9];
    let r1 = x[0] - x[8];
    x[8] += x[0];
    x[9] += x[1];
    x[0] = (r0 + r1) * PI2_8;
    x[1] = (r0 - r1) * PI2_8;
    let r0 = x[3] - x[11];
    let r1 = x[10] - x[2];
    x[10] += x[2];
    x[11] += x[3];
    x[2] = r0;
    x[3] = r1;
    let r0 = x[12] - x[4];
    let r1 = x[13] - x[5];
    x[12] += x[4];
    x[13] += x[5];
    x[4] = (r0 - r1) * PI2_8;
    x[5] = (r0 + r1) * PI2_8;
    let r0 = x[14] - x[6];
    let r1 = x[15] - x[7];
    x[14] += x[6];
    x[15] += x[7];
    x[6] = r0;
    x[7] = r1;
    butterfly_8(&mut x[..8]);
    butterfly_8(&mut x[8..16]);
}

fn butterfly_32(x: &mut [f32]) {
    let r0 = x[30] - x[14];
    let r1 = x[31] - x[15];
    x[30] += x[14];
    x[31] += x[15];
    x[14] = r0;
    x[15] = r1;
    let r0 = x[28] - x[12];
    let r1 = x[29] - x[13];
    x[28] += x[12];
    x[29] += x[13];
    x[12] = r0 * PI1_8 - r1 * PI3_8;
    x[13] = r0 * PI3_8 + r1 * PI1_8;
    let r0 = x[26] - x[10];
    let r1 = x[27] - x[11];
    x[26] += x[10];
    x[27] += x[11];
    x[10] = (r0 - r1) * PI2_8;
    x[11] = (r0 + r1) * PI2_8;
    let r0 = x[24] - x[8];
    let r1 = x[25] - x[9];
    x[24] += x[8];
    x[25] += x[9];
    x[8] = r0 * PI3_8 - r1 * PI1_8;
    x[9] = r1 * PI3_8 + r0 * PI1_8;
    let r0 = x[22] - x[6];
    let r1 = x[7] - x[23];
    x[22] += x[6];
    x[23] += x[7];
    x[6] = r1;
    x[7] = r0;
    let r0 = x[4] - x[20];
    let r1 = x[5] - x[21];
    x[20] += x[4];
    x[21] += x[5];
    x[4] = r1 * PI1_8 + r0 * PI3_8;
    x[5] = r1 * PI3_8 - r0 * PI1_8;
    let r0 = x[2] - x[18];
    let r1 = x[3] - x[19];
    x[18] += x[2];
    x[19] += x[3];
    x[2] = (r1 + r0) * PI2_8;
    x[3] = (r1 - r0) * PI2_8;
    let r0 = x[0] - x[16];
    let r1 = x[1] - x[17];
    x[16] += x[0];
    x[17] += x[1];
    x[0] = r1 * PI3_8 + r0 * PI1_8;
    x[1] = r1 * PI1_8 - r0 * PI3_8;
    butterfly_16(&mut x[..16]);
    butterfly_16(&mut x[16..32]);
}

/// `mdct_butterfly_first` (trig step 4) and `mdct_butterfly_generic`
/// (step `trigint`): `x` is the `points` they work on.
fn butterfly_generic(t: &[f32], x: &mut [f32], points: usize, trigint: usize) {
    let mut x1 = points - 8;
    let mut x2 = (points >> 1) - 8;
    let mut ti = 0usize;
    loop {
        for (k, off) in [(6usize, 0usize), (4, 1), (2, 2), (0, 3)] {
            let tt = &t[ti + off * trigint..];
            let r0 = x[x1 + k] - x[x2 + k];
            let r1 = x[x1 + k + 1] - x[x2 + k + 1];
            x[x1 + k] += x[x2 + k];
            x[x1 + k + 1] += x[x2 + k + 1];
            x[x2 + k] = r1 * tt[1] + r0 * tt[0];
            x[x2 + k + 1] = r1 * tt[0] - r0 * tt[1];
        }
        ti += 4 * trigint;
        if x2 < 8 {
            break;
        }
        x1 -= 8;
        x2 -= 8;
    }
}

impl Mdct {
    fn butterflies(&self, x: &mut [f32], points: usize) {
        let t = &self.trig;
        let mut stages = self.log2n - 5;
        stages -= 1;
        if stages > 0 {
            butterfly_generic(t, x, points, 4);
        }
        let mut i = 1;
        loop {
            stages -= 1;
            if stages <= 0 {
                break;
            }
            for j in 0..(1usize << i) {
                let p = points >> i;
                butterfly_generic(t, &mut x[p * j..p * (j + 1)], p, 4 << i);
            }
            i += 1;
        }
        for j in (0..points).step_by(32) {
            butterfly_32(&mut x[j..j + 32]);
        }
    }

    fn bitreverse(&self, x: &mut [f32]) {
        let n = self.n;
        let bit = &self.bitrev;
        let t = &self.trig[n..];
        let half = n >> 1;
        let (mut w0, mut w1) = (0usize, half);
        let (mut bi, mut ti) = (0usize, 0usize);
        loop {
            let b = |k: usize| half + bit[bi + k].cast_unsigned() as usize;
            let (x0, x1) = (b(0), b(1));
            let r0 = x[x0 + 1] - x[x1 + 1];
            let r1 = x[x0] + x[x1];
            let r2 = r1 * t[ti] + r0 * t[ti + 1];
            let r3 = r1 * t[ti + 1] - r0 * t[ti];
            w1 -= 4;
            let r0 = (x[x0 + 1] + x[x1 + 1]) * 0.5;
            let r1 = (x[x0] - x[x1]) * 0.5;
            x[w0] = r0 + r2;
            x[w1 + 2] = r0 - r2;
            x[w0 + 1] = r1 + r3;
            x[w1 + 3] = r3 - r1;
            let (x0, x1) = (b(2), b(3));
            let r0 = x[x0 + 1] - x[x1 + 1];
            let r1 = x[x0] + x[x1];
            let r2 = r1 * t[ti + 2] + r0 * t[ti + 3];
            let r3 = r1 * t[ti + 3] - r0 * t[ti + 2];
            let r0 = (x[x0 + 1] + x[x1 + 1]) * 0.5;
            let r1 = (x[x0] - x[x1]) * 0.5;
            x[w0 + 2] = r0 + r2;
            x[w1] = r0 - r2;
            x[w0 + 3] = r1 + r3;
            x[w1 + 1] = r3 - r1;
            ti += 4;
            bi += 4;
            w0 += 4;
            if w0 >= w1 {
                break;
            }
        }
    }

    /// `mdct_backward(init, in, out)` with `in == out` (as
    /// `mapping0_inverse` calls it): reads the first `n/2`, writes `n`.
    fn backward(&self, out: &mut [f32]) {
        let n = self.n;
        let n2 = n >> 1;
        let n4 = n >> 2;
        let input: Vec<f32> = out[..n2].to_vec();
        let t = &self.trig;
        // rotate
        let mut ix = n2 - 7;
        let mut ox = n2 + n4;
        let mut ti = n4;
        loop {
            ox -= 4;
            out[ox] = -input[ix + 2] * t[ti + 3] - input[ix] * t[ti + 2];
            out[ox + 1] = input[ix] * t[ti + 3] - input[ix + 2] * t[ti + 2];
            out[ox + 2] = -input[ix + 6] * t[ti + 1] - input[ix + 4] * t[ti];
            out[ox + 3] = input[ix + 4] * t[ti + 1] - input[ix + 6] * t[ti];
            ti += 4;
            if ix < 8 {
                break;
            }
            ix -= 8;
        }
        let mut ix = n2 - 8;
        let mut ox = n2 + n4;
        let mut ti = n4;
        loop {
            ti -= 4;
            out[ox] = input[ix + 4] * t[ti + 3] + input[ix + 6] * t[ti + 2];
            out[ox + 1] = input[ix + 4] * t[ti + 2] - input[ix + 6] * t[ti + 3];
            out[ox + 2] = input[ix] * t[ti + 1] + input[ix + 2] * t[ti];
            out[ox + 3] = input[ix] * t[ti] - input[ix + 2] * t[ti + 1];
            ox += 4;
            if ix < 8 {
                break;
            }
            ix -= 8;
        }
        self.butterflies(&mut out[n2..n], n2);
        self.bitreverse(out);
        // rotate + window
        let mut ox1 = n2 + n4;
        let mut ox2 = n2 + n4;
        let mut ix = 0usize;
        let mut ti = n2;
        loop {
            ox1 -= 4;
            out[ox1 + 3] = out[ix] * t[ti + 1] - out[ix + 1] * t[ti];
            out[ox2] = -(out[ix] * t[ti] + out[ix + 1] * t[ti + 1]);
            out[ox1 + 2] = out[ix + 2] * t[ti + 3] - out[ix + 3] * t[ti + 2];
            out[ox2 + 1] = -(out[ix + 2] * t[ti + 2] + out[ix + 3] * t[ti + 3]);
            out[ox1 + 1] = out[ix + 4] * t[ti + 5] - out[ix + 5] * t[ti + 4];
            out[ox2 + 2] = -(out[ix + 4] * t[ti + 4] + out[ix + 5] * t[ti + 5]);
            out[ox1] = out[ix + 6] * t[ti + 7] - out[ix + 7] * t[ti + 6];
            out[ox2 + 3] = -(out[ix + 6] * t[ti + 6] + out[ix + 7] * t[ti + 7]);
            ox2 += 4;
            ix += 8;
            ti += 8;
            if ix >= ox1 {
                break;
            }
        }
        let mut ix = n2 + n4;
        let mut ox1 = n4;
        let mut ox2 = ox1;
        loop {
            ox1 -= 4;
            ix -= 4;
            out[ox1 + 3] = out[ix + 3];
            out[ox2] = -out[ox1 + 3];
            out[ox1 + 2] = out[ix + 2];
            out[ox2 + 1] = -out[ox1 + 2];
            out[ox1 + 1] = out[ix + 1];
            out[ox2 + 2] = -out[ox1 + 1];
            out[ox1] = out[ix];
            out[ox2 + 3] = -out[ox1];
            ox2 += 4;
            if ox2 >= ix {
                break;
            }
        }
        let mut ix = n2 + n4;
        let mut ox1 = n2 + n4;
        let ox2 = n2;
        loop {
            ox1 -= 4;
            out[ox1] = out[ix + 3];
            out[ox1 + 1] = out[ix + 2];
            out[ox1 + 2] = out[ix + 1];
            out[ox1 + 3] = out[ix];
            ix += 4;
            if ox1 <= ox2 {
                break;
            }
        }
    }
}

/// `_vorbis_window_get(n)`.
fn window(n: usize) -> &'static [f32] {
    use tables::{VWIN64, VWIN128, VWIN256, VWIN512, VWIN1024, VWIN2048, VWIN4096, VWIN8192};
    match n {
        0 => &VWIN64,
        1 => &VWIN128,
        2 => &VWIN256,
        3 => &VWIN512,
        4 => &VWIN1024,
        5 => &VWIN2048,
        6 => &VWIN4096,
        _ => &VWIN8192,
    }
}

/// `render_point`.
fn render_point(x0: i32, x1: i32, y0: i32, y1: i32, x: i32) -> i32 {
    let y0 = y0 & 0x7fff;
    let y1 = y1 & 0x7fff;
    let dy = y1 - y0;
    let adx = x1 - x0;
    let ady = dy.abs();
    let err = ady * (x - x0);
    let off = if adx == 0 { 0 } else { err / adx };
    if dy < 0 { y0 - off } else { y0 + off }
}

/// `render_line`: `d[x] *= FLOOR1_fromdB_LOOKUP[y]` along the line.
fn render_line(n: i32, x0: i32, x1: i32, y0: i32, y1: i32, d: &mut [f32]) {
    let lookup = &tables::FLOOR1_FROM_DB;
    let dy = y1 - y0;
    let adx = x1 - x0;
    if adx == 0 {
        return;
    }
    let mut ady = dy.abs();
    let base = dy / adx;
    let sy = if dy < 0 { base - 1 } else { base + 1 };
    let mut x = x0;
    let mut y = y0;
    let mut err = 0;
    ady -= (base * adx).abs();
    let n = n.min(x1);
    let mut mul = |x: i32, y: i32| {
        if let (Some(v), Some(&f)) = (
            d.get_mut(x.cast_unsigned() as usize),
            lookup.get(y.cast_unsigned() as usize),
        ) {
            *v *= f;
        }
    };
    if x < n {
        mul(x, y);
    }
    loop {
        x += 1;
        if x >= n {
            break;
        }
        err += ady;
        if err >= adx {
            err -= adx;
            y += sy;
        } else {
            y += base;
        }
        mul(x, y);
    }
}

impl Floor1 {
    /// `floor1_inverse1`: the decoded posts, `None` when the floor is
    /// unused or the packet ends.
    fn inverse1(&self, books: &[Book], opb: &mut Reader) -> Option<Vec<i32>> {
        if opb.read(1) != 1 {
            return None;
        }
        let mut fit = vec![0i32; self.posts];
        let bits = ilog((self.quant_q - 1).cast_unsigned());
        fit[0] = opb.read(bits);
        fit[1] = opb.read(bits);
        let mut j = 2usize;
        for &class in &self.partitionclass {
            let class = class.cast_unsigned() as usize;
            let cdim = self.class_dim[class].cast_unsigned() as usize;
            let csubbits = self.class_subs[class];
            let csub = 1 << csubbits;
            let mut cval = 0;
            if csubbits != 0 {
                cval = books[self.class_book[class].cast_unsigned() as usize].decode(opb);
                if cval == -1 {
                    return None;
                }
            }
            for k in 0..cdim {
                let book = self.class_subbook[class][(cval & (csub - 1)).cast_unsigned() as usize];
                cval >>= csubbits;
                if book >= 0 {
                    fit[j + k] = books[book.cast_unsigned() as usize].decode(opb);
                    if fit[j + k] == -1 {
                        return None;
                    }
                } else {
                    fit[j + k] = 0;
                }
            }
            j += cdim;
        }
        for i in 2..self.posts {
            let lo = self.loneighbor[i - 2];
            let hi = self.hineighbor[i - 2];
            let predicted = render_point(
                self.postlist[lo],
                self.postlist[hi],
                fit[lo],
                fit[hi],
                self.postlist[i],
            );
            let hiroom = self.quant_q - predicted;
            let loroom = predicted;
            let room = hiroom.min(loroom) << 1;
            let mut val = fit[i];
            if val != 0 {
                if val >= room {
                    if hiroom > loroom {
                        val -= loroom;
                    } else {
                        val = -1 - (val - hiroom);
                    }
                } else if val & 1 != 0 {
                    val = -((val + 1) >> 1);
                } else {
                    val >>= 1;
                }
                fit[i] = (val + predicted) & 0x7fff;
                fit[lo] &= 0x7fff;
                fit[hi] &= 0x7fff;
            } else {
                fit[i] = predicted | 0x8000;
            }
        }
        Some(fit)
    }

    /// `floor1_inverse2`: the curve applied to the residue in `out`.
    fn inverse2(&self, n: usize, memo: Option<&[i32]>, out: &mut [f32]) {
        let Some(fit) = memo else {
            out[..n].fill(0.0);
            return;
        };
        let n = i32::try_from(n).unwrap_or(0);
        let mut hx = 0;
        let mut lx = 0;
        let mut ly = (fit[0] * self.mult).clamp(0, 255);
        for j in 1..self.posts {
            let current = self.forward_index[j];
            let mut hy = fit[current] & 0x7fff;
            if hy == fit[current] {
                hx = self.postlist[current];
                hy = (hy * self.mult).clamp(0, 255);
                render_line(n, lx, hx, ly, hy, out);
                lx = hx;
                ly = hy;
            }
        }
        let f = tables::FLOOR1_FROM_DB[ly.cast_unsigned() as usize];
        for j in hx..n {
            if let Some(v) = out.get_mut(j.cast_unsigned() as usize) {
                *v *= f;
            }
        }
    }
}

impl Residue {
    /// `res0_inverse` / `res1_inverse` (`_01inverse`) over the channels
    /// in use.
    fn inverse01(&self, books: &[Book], pcmend: usize, opb: &mut Reader, chans: &mut [&mut [f32]]) {
        let ch = chans.len();
        let spp = self.grouping;
        let ppw = books[self.groupbook].dim;
        let max = i32::try_from(pcmend >> 1).unwrap_or(0);
        let end = self.end.min(max);
        let n = end - self.begin;
        if n <= 0 || spp <= 0 || ppw <= 0 {
            return;
        }
        let partvals = n / spp;
        let partwords = usize::try_from((partvals + ppw - 1) / ppw).unwrap_or(0);
        let mut partword: Vec<Vec<usize>> = vec![vec![0; partwords]; ch];
        for s in 0..self.stages {
            let mut i = 0;
            let mut l = 0usize;
            while i < partvals {
                if s == 0 {
                    for pw in &mut partword {
                        let temp = books[self.groupbook].decode(opb);
                        if temp == -1 || temp >= self.partvals {
                            return;
                        }
                        pw[l] = temp.cast_unsigned() as usize;
                    }
                }
                let mut k = 0;
                while k < ppw && i < partvals {
                    for (j, chan) in chans.iter_mut().enumerate() {
                        let offset = (self.begin + i * spp).cast_unsigned() as usize;
                        let part = self.decodemap[partword[j][l]][k.cast_unsigned() as usize];
                        let part = part.cast_unsigned() as usize;
                        if self.secondstages[part] & (1 << s) != 0
                            && let Some(Some(b)) =
                                self.partbooks[part].get(s.cast_unsigned() as usize)
                        {
                            let target = chan.get_mut(offset..).unwrap_or(&mut []);
                            let r = if self.kind == 0 {
                                books[*b].decodevs_add(target, opb, spp)
                            } else {
                                books[*b].decodev_add(target, opb, spp)
                            };
                            if r == -1 {
                                return;
                            }
                        }
                    }
                    k += 1;
                    i += 1;
                }
                l += 1;
            }
        }
    }

    /// `res2_inverse`.
    fn inverse2(
        &self,
        books: &[Book],
        pcmend: usize,
        opb: &mut Reader,
        chans: &mut [&mut [f32]],
        nonzero: &[bool],
    ) {
        let ch = i32::try_from(chans.len()).unwrap_or(0);
        let spp = self.grouping;
        let ppw = books[self.groupbook].dim;
        let max = i32::try_from((pcmend * chans.len()) >> 1).unwrap_or(0);
        let end = self.end.min(max);
        let n = end - self.begin;
        if n <= 0 || spp <= 0 || ppw <= 0 {
            return;
        }
        let partvals = n / spp;
        let partwords = usize::try_from((partvals + ppw - 1) / ppw).unwrap_or(0);
        let mut partword = vec![0usize; partwords];
        if !nonzero.iter().any(|&z| z) {
            return;
        }
        for s in 0..self.stages {
            let mut i = 0;
            let mut l = 0usize;
            while i < partvals {
                if s == 0 {
                    let temp = books[self.groupbook].decode(opb);
                    if temp == -1 || temp >= self.partvals {
                        return;
                    }
                    partword[l] = temp.cast_unsigned() as usize;
                }
                let mut k = 0;
                while k < ppw && i < partvals {
                    let part = self.decodemap[partword[l]][k.cast_unsigned() as usize];
                    let part = part.cast_unsigned() as usize;
                    if self.secondstages[part] & (1 << s) != 0
                        && let Some(Some(b)) = self.partbooks[part].get(s.cast_unsigned() as usize)
                        && books[*b].decodevv_add(chans, i * spp + self.begin, ch, opb, spp) == -1
                    {
                        return;
                    }
                    k += 1;
                    i += 1;
                }
                l += 1;
            }
        }
    }
}

impl Decoder {
    /// `vorbis_info_init` + `vorbis_comment_init`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `vorbis_synthesis_headerin` as `PhyreEngine` built it: the
    /// identification header returns 30, the comment header the bytes of
    /// its vendor string and comments plus 12, the setup header 0; errors
    /// are `OV_*`. `bytes` is the packet size the caller claims.
    pub fn headerin(&mut self, packet: &[u8], bytes: i32, b_o_s: bool) -> i32 {
        let mut opb = Reader::new(packet, bytes);
        let kind = opb.read(8);
        let mut magic = [0u8; 6];
        for m in &mut magic {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                *m = opb.read(8) as u8;
            }
        }
        if &magic != b"vorbis" {
            return OV_ENOTVORBIS;
        }
        match kind {
            1 => {
                if !b_o_s || self.rate != 0 {
                    return OV_EBADHEADER;
                }
                self.unpack_info(&mut opb)
            }
            3 => {
                if self.rate == 0 || self.headers != 1 {
                    return OV_EBADHEADER;
                }
                self.unpack_comment(&mut opb)
            }
            5 => {
                if self.rate == 0 || self.headers != 2 || !self.books.is_empty() {
                    return OV_EBADHEADER;
                }
                self.unpack_books(&mut opb)
            }
            _ => OV_EBADHEADER,
        }
    }

    fn unpack_info(&mut self, opb: &mut Reader) -> i32 {
        self.version = opb.read(32);
        if self.version != 0 {
            return -134; // OV_EVERSION
        }
        self.channels = opb.read(8);
        self.rate = opb.read(32);
        self.bitrate_upper = opb.read(32);
        self.bitrate_nominal = opb.read(32);
        self.bitrate_lower = opb.read(32);
        let b0 = opb.read(4);
        let b1 = opb.read(4);
        let bs = |b: i32| if (0..31).contains(&b) { 1usize << b } else { 0 };
        self.blocksizes = [bs(b0), bs(b1)];
        if self.rate < 1
            || self.channels < 1
            || self.blocksizes[0] < 64
            || self.blocksizes[1] < self.blocksizes[0]
            || self.blocksizes[1] > 8192
        {
            *self = Self::default();
            return OV_EBADHEADER;
        }
        self.headers = 1;
        0x17 + 7
    }

    fn unpack_comment(&mut self, opb: &mut Reader) -> i32 {
        let vendorlen = opb.read(32);
        if vendorlen < 0 || vendorlen > opb.storage() - 8 {
            return OV_EBADHEADER;
        }
        for _ in 0..vendorlen {
            opb.read(8);
        }
        let comments = opb.read(32);
        if comments < 0 || comments > (opb.storage() - opb.bytes()) >> 2 {
            return OV_EBADHEADER;
        }
        let mut sum = 0i32;
        for _ in 0..comments {
            let len = opb.read(32);
            if len < 0 || len > opb.storage() - opb.bytes() {
                return OV_EBADHEADER;
            }
            for _ in 0..len {
                opb.read(8);
            }
            sum += len + 4;
        }
        self.headers = 2;
        vendorlen + 0xc + sum
    }

    fn unpack_books(&mut self, opb: &mut Reader) -> i32 {
        let r = self.unpack_books_inner(opb);
        if r.is_none() {
            let (rate, channels) = (self.rate, self.channels);
            *self = Self::default();
            // vorbis_info_clear wipes the info too
            let _ = (rate, channels);
            return OV_EBADHEADER;
        }
        self.headers = 3;
        0
    }

    fn unpack_books_inner(&mut self, opb: &mut Reader) -> Option<()> {
        let nbooks = opb.read(8) + 1;
        if nbooks <= 0 {
            return None;
        }
        let mut statics = Vec::new();
        for _ in 0..nbooks {
            statics.push(book::unpack(opb)?);
        }
        let times = opb.read(6) + 1;
        if times <= 0 {
            return None;
        }
        for _ in 0..times {
            let t = opb.read(16);
            if t != 0 {
                return None;
            }
        }
        let floors = opb.read(6) + 1;
        if floors <= 0 {
            return None;
        }
        for _ in 0..floors {
            let kind = opb.read(16);
            if kind != 1 {
                // floor 0 is not ported (no movie uses it); others are invalid
                return None;
            }
            self.floors.push(floor1_unpack(statics.len(), opb)?);
        }
        let residues = opb.read(6) + 1;
        if residues <= 0 {
            return None;
        }
        for _ in 0..residues {
            let kind = opb.read(16);
            if !(0..3).contains(&kind) {
                return None;
            }
            self.residues.push(res0_unpack(kind, &statics, opb)?);
        }
        let maps = opb.read(6) + 1;
        if maps <= 0 {
            return None;
        }
        for _ in 0..maps {
            if opb.read(16) != 0 {
                return None;
            }
            self.maps.push(mapping0_unpack(
                self.channels,
                self.floors.len(),
                self.residues.len(),
                opb,
            )?);
        }
        let modes = opb.read(6) + 1;
        if modes <= 0 {
            return None;
        }
        for _ in 0..modes {
            let blockflag = opb.read(1);
            let windowtype = opb.read(16);
            let transformtype = opb.read(16);
            let mapping = opb.read(8);
            if windowtype >= 1 || transformtype >= 1 || mapping >= maps || mapping < 0 {
                return None;
            }
            self.modes.push(Mode {
                blockflag,
                mapping: usize::try_from(mapping).ok()?,
            });
        }
        // vorbis_book_init_decode for each (in _vds_shared_init)
        self.books = statics
            .iter()
            .map(Book::init_decode)
            .collect::<Option<_>>()?;
        Some(())
    }

    /// `vorbis_synthesis_init` (`_vds_shared_init` + `restart`): 0, or 1
    /// without a complete setup.
    pub fn synthesis_init(&mut self) -> i32 {
        if self.modes.is_empty()
            || self.blocksizes[0] < 64
            || self.blocksizes[1] < self.blocksizes[0]
        {
            return 1;
        }
        self.modebits = ilog(u32::try_from(self.modes.len() - 1).unwrap_or(0));
        self.transform = [Mdct::new(self.blocksizes[0]), Mdct::new(self.blocksizes[1])];
        let ilog_n = |n: usize| usize::try_from(ilog(u32::try_from(n).unwrap_or(0))).unwrap_or(7);
        self.window = [
            ilog_n(self.blocksizes[0]) - 7,
            ilog_n(self.blocksizes[1]) - 7,
        ];
        let ch = usize::try_from(self.channels).unwrap_or(0);
        self.pcm = vec![vec![0.0; self.blocksizes[1]]; ch];
        self.lw = 0;
        self.w = 0;
        // restart
        self.center_w = self.blocksizes[1] >> 1;
        self.pcm_current = i64::try_from(self.center_w).unwrap_or(0);
        self.pcm_returned = -1;
        self.granulepos = -1;
        self.sequence = -1;
        self.eofflag = false;
        self.sample_count = -1;
        self.initialized = true;
        0
    }

    /// `vorbis_synthesis(vb, op)`: decodes the packet into the block; 0,
    /// or `OV_ENOTAUDIO` / `OV_EBADPACKET`.
    pub fn synthesis(
        &mut self,
        packet: &[u8],
        bytes: i32,
        granulepos: i64,
        packetno: i64,
        e_o_s: bool,
    ) -> i32 {
        if !self.initialized {
            return OV_EBADPACKET;
        }
        self.block.ready = false;
        let mut opb = Reader::new(packet, bytes);
        if opb.read(1) != 0 {
            return OV_ENOTAUDIO;
        }
        let mode = opb.read(self.modebits);
        if mode == -1 {
            return OV_EBADPACKET;
        }
        let Some(&m) = self.modes.get(mode.cast_unsigned() as usize) else {
            return OV_EBADPACKET;
        };
        let w = usize::from(m.blockflag != 0);
        if w != 0 {
            opb.read(1); // lW
            let nw = opb.read(1);
            if nw == -1 {
                return OV_EBADPACKET;
            }
        }
        let n = self.blocksizes[w];
        let ch = usize::try_from(self.channels).unwrap_or(0);
        let mut block = std::mem::take(&mut self.block);
        block.w = w;
        block.granulepos = granulepos;
        block.sequence = packetno;
        block.eofflag = e_o_s;
        block.pcmend = n;
        block.pcm.resize(ch, Vec::new());
        for p in &mut block.pcm {
            p.clear();
            p.resize(n, 0.0);
        }
        self.mapping0_inverse(&mut block, m.mapping, &mut opb);
        block.ready = true;
        self.block = block;
        0
    }

    fn mapping0_inverse(&self, vb: &mut Block, mapping: usize, opb: &mut Reader) {
        let info = &self.maps[mapping];
        let n = vb.pcmend;
        let ch = vb.pcm.len();
        let mut memo: Vec<Option<Vec<i32>>> = Vec::with_capacity(ch);
        let mut nonzero = vec![false; ch];
        for i in 0..ch {
            let submap = info.chmuxlist[i].cast_unsigned() as usize;
            let f = &self.floors[info.floorsubmap[submap]];
            let m = f.inverse1(&self.books, opb);
            nonzero[i] = m.is_some();
            memo.push(m);
            vb.pcm[i][..n / 2].fill(0.0);
        }
        for (&m, &a) in info.coupling_mag.iter().zip(&info.coupling_ang) {
            if nonzero[m] || nonzero[a] {
                nonzero[m] = true;
                nonzero[a] = true;
            }
        }
        for i in 0..info.submaps.cast_unsigned() as usize {
            let res = &self.residues[info.residuesubmap[i]];
            let members: Vec<usize> = (0..ch)
                .filter(|&j| info.chmuxlist[j].cast_unsigned() as usize == i)
                .collect();
            let zero: Vec<bool> = members.iter().map(|&j| nonzero[j]).collect();
            // borrow the member channels
            let mut refs: Vec<&mut [f32]> = Vec::new();
            let mut rest: &mut [Vec<f32>] = &mut vb.pcm;
            let mut base = 0usize;
            for &j in &members {
                let (_, tail) = rest.split_at_mut(j - base);
                let (one, tail) = tail.split_at_mut(1);
                refs.push(&mut one[0][..]);
                rest = tail;
                base = j + 1;
            }
            if res.kind == 2 {
                res.inverse2(&self.books, vb.pcmend, opb, &mut refs, &zero);
            } else {
                let mut used: Vec<&mut [f32]> = refs
                    .into_iter()
                    .zip(&zero)
                    .filter_map(|(r, &z)| z.then_some(r))
                    .collect();
                if !used.is_empty() {
                    res.inverse01(&self.books, vb.pcmend, opb, &mut used);
                }
            }
        }
        for (&m, &a) in info.coupling_mag.iter().zip(&info.coupling_ang).rev() {
            for j in 0..n / 2 {
                let mag = vb.pcm[m][j];
                let ang = vb.pcm[a][j];
                let (pm, pa) = if mag > 0.0 {
                    if ang > 0.0 {
                        (mag, mag - ang)
                    } else {
                        (mag + ang, mag)
                    }
                } else if ang > 0.0 {
                    (mag, mag + ang)
                } else {
                    (mag - ang, mag)
                };
                vb.pcm[m][j] = pm;
                vb.pcm[a][j] = pa;
            }
        }
        for i in 0..ch {
            let submap = info.chmuxlist[i].cast_unsigned() as usize;
            let f = &self.floors[info.floorsubmap[submap]];
            f.inverse2(n / 2, memo[i].as_deref(), &mut vb.pcm[i]);
        }
        for p in &mut vb.pcm {
            self.transform[vb.w].backward(p);
        }
    }

    /// `vorbis_synthesis_blockin(v, vb)`: overlap-adds the decoded block.
    pub fn blockin(&mut self) -> i32 {
        if !self.block.ready {
            return OV_EINVAL;
        }
        if self.pcm_current > self.pcm_returned && self.pcm_returned != -1 {
            return OV_EINVAL;
        }
        let vb = std::mem::take(&mut self.block);
        self.lw = self.w;
        self.w = vb.w;
        if self.sequence == -1 || self.sequence + 1 != vb.sequence {
            self.granulepos = -1;
            self.sample_count = -1;
        }
        self.sequence = vb.sequence;
        let n = self.blocksizes[self.w] >> 1;
        let n0 = self.blocksizes[0] >> 1;
        let n1 = self.blocksizes[1] >> 1;
        let (this_center, prev_center) = if self.center_w != 0 { (n1, 0) } else { (0, n1) };
        for (pcm, p) in self.pcm.iter_mut().zip(&vb.pcm) {
            if self.lw != 0 {
                if self.w != 0 {
                    let w = window(self.window[1]);
                    let pcm = &mut pcm[prev_center..];
                    for i in 0..n1 {
                        pcm[i] = pcm[i] * w[n1 - i - 1] + p[i] * w[i];
                    }
                } else {
                    let w = window(self.window[0]);
                    let pcm = &mut pcm[prev_center + n1 / 2 - n0 / 2..];
                    for i in 0..n0 {
                        pcm[i] = pcm[i] * w[n0 - i - 1] + p[i] * w[i];
                    }
                }
            } else if self.w != 0 {
                let w = window(self.window[0]);
                let pcm = &mut pcm[prev_center..];
                let p = &p[n1 / 2 - n0 / 2..];
                let mut i = 0;
                while i < n0 {
                    pcm[i] = pcm[i] * w[n0 - i - 1] + p[i] * w[i];
                    i += 1;
                }
                while i < n1 / 2 + n0 / 2 {
                    pcm[i] = p[i];
                    i += 1;
                }
            } else {
                let w = window(self.window[0]);
                let pcm = &mut pcm[prev_center..];
                for i in 0..n0 {
                    pcm[i] = pcm[i] * w[n0 - i - 1] + p[i] * w[i];
                }
            }
            pcm[this_center..this_center + n].copy_from_slice(&p[n..2 * n]);
        }
        self.center_w = if self.center_w != 0 { 0 } else { n1 };
        let to_i64 = |v: usize| i64::try_from(v).unwrap_or(0);
        if self.pcm_returned == -1 {
            self.pcm_returned = to_i64(this_center);
            self.pcm_current = to_i64(this_center);
        } else {
            self.pcm_returned = to_i64(prev_center);
            self.pcm_current =
                to_i64(prev_center + self.blocksizes[self.lw] / 4 + self.blocksizes[self.w] / 4);
        }
        let step = to_i64(self.blocksizes[self.lw] / 4 + self.blocksizes[self.w] / 4);
        if self.sample_count == -1 {
            self.sample_count = 0;
        } else {
            self.sample_count += step;
        }
        if self.granulepos == -1 {
            if vb.granulepos != -1 {
                self.granulepos = vb.granulepos;
                if self.sample_count > self.granulepos {
                    // C long: 32 bits
                    #[allow(clippy::cast_possible_truncation)] // C long
                    let mut extra = i64::from(self.sample_count.wrapping_sub(vb.granulepos) as i32);
                    if extra < 0 {
                        extra = 0;
                    }
                    if vb.eofflag {
                        extra = extra.min(self.pcm_current - self.pcm_returned);
                        self.pcm_current -= extra;
                    } else {
                        self.pcm_returned = (self.pcm_returned + extra).min(self.pcm_current);
                    }
                }
            }
        } else {
            self.granulepos += step;
            if vb.granulepos != -1 && self.granulepos != vb.granulepos {
                if self.granulepos > vb.granulepos {
                    #[allow(clippy::cast_possible_truncation)] // C long
                    let mut extra = i64::from(self.granulepos.wrapping_sub(vb.granulepos) as i32);
                    if extra != 0 && vb.eofflag {
                        extra = extra.min(self.pcm_current - self.pcm_returned);
                        if extra < 0 {
                            extra = 0;
                        }
                        self.pcm_current -= extra;
                    }
                }
                self.granulepos = vb.granulepos;
            }
        }
        if vb.eofflag {
            self.eofflag = true;
        }
        self.block = vb;
        self.block.ready = false;
        0
    }

    /// `vorbis_synthesis_pcmout`: the samples ready per channel.
    #[must_use]
    pub fn pcmout(&self) -> usize {
        if self.pcm_returned > -1 && self.pcm_returned < self.pcm_current {
            usize::try_from(self.pcm_current - self.pcm_returned).unwrap_or(0)
        } else {
            0
        }
    }

    /// The ready samples of channel `c` (what `pcmout`'s `pcm[c]` points
    /// at, [`Decoder::pcmout`] long).
    #[must_use]
    pub fn channel(&self, c: usize) -> &[f32] {
        let n = self.pcmout();
        let start = usize::try_from(self.pcm_returned.max(0)).unwrap_or(0);
        self.pcm.get(c).map_or(&[], |p| &p[start..start + n])
    }

    /// `vorbis_synthesis_read`: marks `n` samples taken; `OV_EINVAL` when
    /// fewer are ready.
    pub fn read(&mut self, n: usize) -> i32 {
        let n = i64::try_from(n).unwrap_or(i64::MAX);
        if n != 0 && self.pcm_returned + n > self.pcm_current {
            return OV_EINVAL;
        }
        self.pcm_returned += n;
        0
    }
}
