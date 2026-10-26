//! List p6-render-c, continuation round 4: the four full-screen meshes
//! the post effects draw (`0x00a05e70`, `0x00a06120`, `0x00a06470`,
//! `0x00a06750`). One body, read from the disassembly of all four; they
//! differ only in their streams and vertex data, which
//! [`FULLSCREEN_MESHES`] holds.
//!
//! Each is `cdecl(cluster) -> eax`:
//! 1. `PCluster::create(cluster; 1, PMesh type, 0)` (thiscall) gives the
//!    mesh; unless its segment array (`+0` count, bit 31 not owned; `+4`
//!    data) holds exactly one entry, a `PhyreMalloc(0x6c, 4)` block is
//!    constructed as one segment (`0x00434430(block, 1)`), the old owned
//!    data is destroyed (`0x00434210(data, n)`) and the block taken;
//! 2. the segment's primitive type `+0x10 = 2`; unless its stream array
//!    (`+0x28` count, `+0x2c` data) holds `n` streams, a `PhyreMalloc(0x40
//!    n, 4)` block gets `n` streams (`0x0055dc40` each) and is handed over
//!    (`0x00434e40(seg + 0x28; block, n)`);
//! 3. per stream, `0x004835d0(stream; stride, count, 1)`, the block size
//!    `[s] * [s+4]` into `+0x38` (a bound buffer released first,
//!    `0x00562da0` then `0x005646d0`), its semantic record (`[s+0xc]`:
//!    byte `+8` the semantic, `+4` the name, `+0` 0);
//! 4. per stream, `0x00434c50(stream; &slot)` (the slots in the frame,
//!    8 bytes each from `ebp - 0x28` down), the vertex data written through
//!    the pointers the slots then hold (all read before the first store);
//! 5. per stream, the next version (`+0x10` + 1, 0 at `+0x18 & 0x7fffffff`)
//!    unmapped with `0x00564cb0(stream; version, &slot)`; a zero result
//!    stores the version and clears byte `+0x14`;
//! 6. `PCluster::allocateInstanceList(cluster; 1, PMeshInstance type, 0)`:
//!    with a list and an instance at its `+0x34`, the instance's
//!    constructor `0x004f27e0(instance; mesh)` and its `eax`; else 0.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `PCluster::create` and `allocateInstanceList` (thiscall on the
/// cluster), the mesh and mesh-instance types.
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
pub const MESH_TYPE: u32 = 0x00c9_9c80;
pub const INSTANCE_TYPE: u32 = 0x00ca_7228;
/// `PhyreMalloc(n, align)` (cdecl), the segment array's construct and
/// destroy (cdecl), the stream constructor and the stream array's
/// `setBuffer` (thiscall).
pub const MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const SEGMENTS_CONSTRUCT: u32 = 0x0043_4430;
pub const SEGMENTS_DESTROY: u32 = 0x0043_4210;
pub const STREAM_CTOR: u32 = 0x0055_dc40;
pub const STREAMS_SET: u32 = 0x0043_4e40;
/// The stream's format set-up, `isBound`, release, map and unmap
/// (thiscall).
pub const STREAM_FORMAT: u32 = 0x0048_35d0;
pub const IS_BOUND: u32 = 0x0056_2da0;
pub const RELEASE: u32 = 0x0056_46d0;
pub const MAP: u32 = 0x0043_4c50;
pub const UNMAP: u32 = 0x0056_4cb0;
/// The mesh instance's constructor (thiscall).
pub const INSTANCE_CTOR: u32 = 0x004f_27e0;

/// The semantic names the streams point at (image data).
pub const NAME_POSITION: u32 = 0x00c9_8620;
pub const NAME_TEXCOORD: u32 = 0x00c9_8638;
pub const NAME_COLOR: u32 = 0x00c9_8650;

/// One vertex stream: the stride and count handed to `0x004835d0` (the
/// colour stream of `0x00a06120` says 4 for its 3 vertices), the semantic
/// byte and name, the vertex data (stored as `f32`).
#[derive(Debug, Clone, Copy)]
pub struct Stream {
    pub size: u32,
    pub count: u32,
    pub semantic: u8,
    pub name: u32,
    pub data: &'static [f32],
}

/// One full-screen mesh builder.
#[derive(Debug, Clone, Copy)]
pub struct QuadMesh {
    /// The original's address.
    pub addr: u32,
    pub streams: &'static [Stream],
}

const POSITION_TRIANGLE: [f32; 9] = [-1.0, -1.0, 1.0, 3.0, -1.0, 1.0, -1.0, 3.0, 1.0];
const TEXCOORD_TRIANGLE: [f32; 6] = [0.0, 0.0, 2.0, 0.0, 0.0, 2.0];
const COLOR_TRIANGLE: [f32; 12] = [1.0; 12];
const POSITION_QUAD: [f32; 18] = [
    -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, 1.0, -1.0, 1.0,
];
const TEXCOORD_QUAD: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0];
const POSITION_QUAD_FLIPPED: [f32; 18] = [
    -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0,
];
const TEXCOORD_QUAD_FLIPPED: [f32; 12] =
    [0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0];

const fn position(count: u32, data: &'static [f32]) -> Stream {
    Stream {
        size: 0xc,
        count,
        semantic: 2,
        name: NAME_POSITION,
        data,
    }
}

const fn texcoord(count: u32, data: &'static [f32]) -> Stream {
    Stream {
        size: 8,
        count,
        semantic: 1,
        name: NAME_TEXCOORD,
        data,
    }
}

/// The four builders: a triangle over the screen (texture coordinates to
/// 2), the same with a white colour stream, and two six-vertex quads
/// (texture coordinates 0..1, the second with the triangles' order and v
/// mirrored).
pub const FULLSCREEN_MESHES: [QuadMesh; 4] = [
    QuadMesh {
        addr: 0x00a0_5e70,
        streams: &[
            position(3, &POSITION_TRIANGLE),
            texcoord(3, &TEXCOORD_TRIANGLE),
        ],
    },
    QuadMesh {
        addr: 0x00a0_6120,
        streams: &[
            position(3, &POSITION_TRIANGLE),
            texcoord(3, &TEXCOORD_TRIANGLE),
            Stream {
                size: 0x10,
                count: 4,
                semantic: 3,
                name: NAME_COLOR,
                data: &COLOR_TRIANGLE,
            },
        ],
    },
    QuadMesh {
        addr: 0x00a0_6470,
        streams: &[position(6, &POSITION_QUAD), texcoord(6, &TEXCOORD_QUAD)],
    },
    QuadMesh {
        addr: 0x00a0_6750,
        streams: &[
            position(6, &POSITION_QUAD_FLIPPED),
            texcoord(6, &TEXCOORD_QUAD_FLIPPED),
        ],
    },
];

/// `0x00a06e60(cluster, u, v)`'s streams (18 vertices: six triangles,
/// the screen split at the texture coordinates `u` and `v`): positions
/// (`0x00c985b4`), texture coordinates (`0x00c98638`), and a third
/// three-float stream (`0x00c9862c`, semantic 2, ones and zeros). The
/// data comes from [`split_vertices`].
pub const SPLIT_STREAMS: [Stream; 3] = [
    Stream {
        size: 0xc,
        count: 0x12,
        semantic: 2,
        name: 0x00c9_85b4,
        data: &[],
    },
    Stream {
        size: 8,
        count: 0x12,
        semantic: 1,
        name: 0x00c9_8638,
        data: &[],
    },
    Stream {
        size: 0xc,
        count: 0x12,
        semantic: 2,
        name: 0x00c9_862c,
        data: &[],
    },
];

/// `0x00a06e60`'s constants in the image: -1.0 (`f32`, `0x00b92340`) and
/// 2.0 (`f64`, `0x00b92270`).
pub const MINUS_ONE: u32 = 0x00b9_2340;
pub const TWO: u32 = 0x00b9_2270;

/// One word of the split mesh's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum W {
    /// -1.0 (the image's).
    M,
    /// 1.0, 0.0.
    One,
    Zero,
    /// `2 u - 1`, `2 v - 1` (rounded to `f32`).
    X,
    Y,
    /// `u`, `v` (through `fld`/`fst`).
    U,
    V,
}

use W::{M, One, U, V, X, Y, Zero};

const SPLIT_POSITIONS: [W; 54] = [
    M, M, One, One, M, One, M, X, One, One, X, One, M, X, One, One, M, One, M, X, One, One, X, One,
    M, Y, One, One, Y, One, M, Y, One, One, X, One, M, Y, One, One, Y, One, M, One, One, One, One,
    One, M, One, One, One, Y, One,
];
const SPLIT_TEXCOORDS: [W; 36] = [
    Zero, Zero, One, Zero, Zero, U, One, U, Zero, U, One, Zero, Zero, U, One, U, Zero, V, One, V,
    Zero, V, One, U, Zero, V, One, V, Zero, One, One, One, Zero, One, One, V,
];
/// The third stream: 1.0 but for these words.
const SPLIT_THIRD_ZEROS: [usize; 6] = [1, 4, 16, 43, 46, 49];

/// The split mesh's three streams (`f32` bits) for `u` and `v` (`f32`
/// bits), with the image's -1.0 and 2.0: `2 u - 1` is `u * 2.0 - 1.0`,
/// `2 v - 1` is `2.0 * v - 1.0` (x87, rounded to `f32` once each).
#[must_use]
pub fn split_vertices(
    minus_one: u32,
    two: f64,
    u: u32,
    v: u32,
) -> ([u32; 54], [u32; 36], [u32; 54]) {
    let one = 1.0f32.to_bits();
    let zero = 0u32;
    let (fu, fv) = (x87::load(u), x87::load(v));
    let x = x87::store(x87::sub(x87::mul(fu, two), 1.0));
    let y = x87::store(x87::sub(x87::mul(two, fv), 1.0));
    let (u, v) = (x87::store(fu), x87::store(fv));
    let minus_one = x87::store(x87::load(minus_one));
    let word = |w: W| match w {
        M => minus_one,
        One => one,
        Zero => zero,
        X => x,
        Y => y,
        U => u,
        V => v,
    };
    let mut third = [one; 54];
    for i in SPLIT_THIRD_ZEROS {
        third[i] = zero;
    }
    (SPLIT_POSITIONS.map(word), SPLIT_TEXCOORDS.map(word), third)
}

/// The frame the builders hand out: 8 bytes per stream, stream `k`'s at
/// `ebp - 0x28 - 8 k`. `frame` is the lowest, `ebp - 0x20 - 8 n`.
#[must_use]
pub fn slot(frame: u32, n: u32, k: u32) -> u32 {
    frame + 8 * (n - 1 - k)
}

pub mod bridge {
    use super::{
        ALLOCATE_INSTANCE_LIST, CLUSTER_CREATE, Conv, Host, INSTANCE_CTOR, INSTANCE_TYPE, IS_BOUND,
        MALLOC_ALIGNED, MAP, MESH_TYPE, QuadMesh, RELEASE, SEGMENTS_CONSTRUCT, SEGMENTS_DESTROY,
        STREAM_CTOR, STREAM_FORMAT, STREAMS_SET, UNMAP, slot,
    };

    /// One of [`super::FULLSCREEN_MESHES`] (`cdecl(cluster) -> eax`):
    /// `frame` holds the map slots (see [`super::slot`]).
    pub fn build(h: &mut impl Host, cluster: u32, frame: u32, q: &QuadMesh) -> u32 {
        build_streams(h, cluster, frame, q.streams, |h, k, p| {
            for (i, &v) in q.streams[k].data.iter().enumerate() {
                h.write(p.wrapping_add(4 * i as u32), v);
            }
        })
    }

    /// The body for `streams` (at most 3), `data(h, k, p)` writing stream
    /// `k`'s vertices at `p` (the pointer its map slot holds; streams in
    /// order, after every map).
    pub fn build_streams<H: Host>(
        h: &mut H,
        cluster: u32,
        frame: u32,
        streams: &[super::Stream],
        mut data: impl FnMut(&mut H, usize, u32),
    ) -> u32 {
        let n = streams.len().min(3) as u32;
        let mesh_type = h.rebase(MESH_TYPE);
        let mesh = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, mesh_type, 0]);
        if h.read::<u32>(mesh) != 1 {
            let block = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x6c, 4]);
            if block != 0 {
                h.call_sym(SEGMENTS_CONSTRUCT, Conv::Cdecl, &[block, 1]);
                let head: u32 = h.read(mesh);
                let data: u32 = h.read(mesh + 4);
                if data != block && head >> 31 == 0 {
                    h.call_sym(SEGMENTS_DESTROY, Conv::Cdecl, &[data, head & 0x7fff_ffff]);
                }
                h.write(mesh + 4, block);
                h.write(mesh, 1u32);
            }
        }
        let seg: u32 = h.read(mesh + 4);
        h.write(seg + 0x10, 2u32);
        if h.read::<u32>(seg + 0x28) != n {
            let block = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x40 * n, 4]);
            if block != 0 {
                for k in 0..n {
                    let s = block.wrapping_add(0x40 * k);
                    if s != 0 {
                        h.call_sym(STREAM_CTOR, Conv::Thiscall(s), &[]);
                    }
                }
                h.call_sym(STREAMS_SET, Conv::Thiscall(seg + 0x28), &[block, n]);
            }
        }
        let specs = streams;
        let mut streams = [0u32; 3];
        for (k, st) in specs.iter().enumerate().take(n as usize) {
            let s = h.read::<u32>(seg + 0x2c).wrapping_add(0x40 * k as u32);
            h.call_sym(STREAM_FORMAT, Conv::Thiscall(s), &[st.size, st.count, 1]);
            let bytes = h.read::<u32>(s).wrapping_mul(h.read(s + 4));
            if h.read::<u32>(s + 0x38) != bytes {
                if h.call_sym(IS_BOUND, Conv::Thiscall(s), &[]) & 0xff != 0 {
                    h.call_sym(RELEASE, Conv::Thiscall(s), &[]);
                }
                h.write(s + 0x38, bytes);
            }
            let record: u32 = h.read(s + 0xc);
            h.write(record + 8, st.semantic);
            let name = h.rebase(st.name);
            h.write(record + 4, name);
            h.write(record, 0u32);
            streams[k] = s;
        }
        for (k, &s) in streams.iter().enumerate().take(n as usize) {
            h.call_sym(MAP, Conv::Thiscall(s), &[slot(frame, n, k as u32)]);
        }
        let mut out = [0u32; 3];
        for (k, p) in out.iter_mut().enumerate().take(n as usize) {
            *p = h.read(slot(frame, n, k as u32));
        }
        for (k, &p) in out.iter().enumerate().take(n as usize) {
            data(h, k, p);
        }
        for (k, &s) in streams.iter().enumerate().take(n as usize) {
            let mut version = h.read::<u32>(s + 0x10).wrapping_add(1);
            if version == h.read::<u32>(s + 0x18) & 0x7fff_ffff {
                version = 0;
            }
            if h.call_sym(
                UNMAP,
                Conv::Thiscall(s),
                &[version, slot(frame, n, k as u32)],
            ) == 0
            {
                h.write(s + 0x10, version);
                h.write(s + 0x14, 0u8);
            }
        }
        let instance_type = h.rebase(INSTANCE_TYPE);
        let list = h.call_sym(
            ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(cluster),
            &[1, instance_type, 0],
        );
        if list != 0 {
            let instance: u32 = h.read(list + 0x34);
            if instance != 0 {
                return h.call_sym(INSTANCE_CTOR, Conv::Thiscall(instance), &[mesh]);
            }
        }
        0
    }

    /// `0x00a06e60(cluster, u, v)` (cdecl, `u` and `v` floats): the
    /// split mesh ([`super::SPLIT_STREAMS`]); `frame` as [`build`]'s.
    pub fn split(h: &mut impl Host, cluster: u32, frame: u32, u: u32, v: u32) -> u32 {
        let minus_one: u32 = h.read(h.rebase(super::MINUS_ONE));
        let two = f64::from_bits(h.read(h.rebase(super::TWO)));
        let (pos, tex, third) = super::split_vertices(minus_one, two, u, v);
        build_streams(h, cluster, frame, &super::SPLIT_STREAMS, |h, k, p| {
            let words: &[u32] = match k {
                0 => &pos,
                1 => &tex,
                _ => &third,
            };
            for (i, &w) in words.iter().enumerate() {
                h.write(p.wrapping_add(4 * i as u32), w);
            }
        })
    }
}
