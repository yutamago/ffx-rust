//! `KeDtt`: the Ke distortion sprites, drawn by sampling the frame buffer.
//!
//! [`ke_dtt_spr_mk_gif_pk`] builds one sprite's mesh in its PC render
//! data: a fan of eight triangles around the projected centre, textured
//! from the screen under it (the render data's `"FrameBuffer"` buffer),
//! so the effect shows the scene shifted by the sprite's UV offsets (heat
//! haze, shock rings).
//!
//! The geometry still lives in the PS2 code's emulated VU0 registers
//! (`0x00c8f830`.., [`vf`]): the effect program fills the rim offsets and
//! UV offsets there before the call, and the function leaves its last
//! vertex in them, as the original does.

use ffx_bridge::Mem;

use crate::camera::FArgs;
use crate::x87;

/// The emulated VU0 registers the sprite is built in.
pub mod vf {
    /// `g_Vu0_Q`: `vf00.w / w` of the projected centre.
    pub const Q: u32 = 0x00c8_f788;
    /// `vf01`: its x, y (times Q) move the UV origin.
    pub const VF01: u32 = 0x00c8_f7a0;
    /// The vertex being built (x, y; z, w the centre's).
    pub const POS: u32 = 0x00c8_f830;
    /// The UV origin: subtracted from a vertex's x, y.
    pub const UV_ORIGIN: u32 = 0x00c8_f840;
    /// The projected centre (x, y, z times Q, w).
    pub const CENTER: u32 = 0x00c8_f850;
    /// The three rim offsets (x, y): the first and third are the axes,
    /// the second the diagonal. Of the six words the function scales by
    /// Q, `+0x00` and `0x00c8f8a4` are left out (the original's).
    pub const RIM_A: u32 = 0x00c8_f880;
    pub const RIM_B: u32 = 0x00c8_f890;
    pub const RIM_C: u32 = 0x00c8_f8a0;
    /// The UV offsets before and after scaling by Q: the axes' (x for
    /// [`RIM_C`]'s vertices, y for [`RIM_A`]'s) and the diagonal's.
    pub const UV_AXES_IN: u32 = 0x00c8_f8b0;
    pub const UV_DIAG_IN: u32 = 0x00c8_f8c0;
    pub const UV_AXES: u32 = 0x00c8_f8f0;
    pub const UV_DIAG: u32 = 0x00c8_f900;
    /// The scale from screen units to the frame buffer's (x).
    pub const UV_SCALE: u32 = 0x00c8_f910;
    /// A vertex's UV: scaled, and before scaling.
    pub const UV_SCALED: u32 = 0x00c8_f930;
    pub const UV: u32 = 0x00c8_f970;
    /// `g_Vu0_VfMove`: gets the sprite's colour.
    pub const MOVE: u32 = 0x00c8_f980;
    /// `vf00.w` (1.0 unless something changed it).
    pub const VF00_W: u32 = 0x00c0_a010;
}

/// The globals and functions it calls.
mod ext {
    /// `g_pppCurrentProgram`: the low word of every VFX instance key.
    pub const PROGRAM: u32 = 0x0230_fd20;
    /// The view matrix the vertices go through.
    pub const VIEW: u32 = 0x0230_fc60;
    /// `g_pppFillMatrix`.
    pub const FILL_MATRIX: u32 = 0x0230_fca0;
    /// `g_MagicFile_MagicId`.
    pub const MAGIC_ID: u32 = 0x00c6_4ca0;
    /// `g_VfxPass`.
    pub const VFX_PASS: u32 = 0x0113_335c;

    pub const GET_DYN_GEOMETRY: u32 = 0x0064_5380;
    pub const GET_RENDER_DATA_BY_GEOMETRY: u32 = 0x0064_53b0;
    pub const IS_IN_CUT_2D_SCENE: u32 = 0x0064_1c00;
    pub const CUT_2D_SCENE_BIAS: u32 = 0x0064_1130;
    pub const VFX_CREATE: u32 = 0x0064_4de0;
    pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;
    pub const GET_RENDER_DATA_BY_INSTANCE: u32 = 0x0064_53d0;
    pub const VFX_SETUP: u32 = 0x0064_5780;
    pub const GET_BUFFER_FOR_SEGMENT: u32 = 0x0063_fdb0;
    pub const VFX_SET_VISIBLE: u32 = 0x0064_5710;
    pub const USE_OPTIMIZED_SHADER: u32 = 0x0064_4ac0;
    pub const VFX_SET_Z_CORRECTION: u32 = 0x0064_5750;
    pub const VFX_UPDATE: u32 = 0x0064_58a0;
    pub const VFX_DTT_UPDATE: u32 = 0x0064_5330;

    pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
    pub const COPY_VECTOR: u32 = 0x006e_d370;
    pub const SCALE_VEC_XYZ: u32 = 0x0075_fb50;
    pub const SUB_XY_FLAGS: u32 = 0x0075_e870;
    pub const RESTORE_VF00: u32 = 0x0042_ef00;
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
}

/// The scratch [`ke_dtt_spr_mk_gif_pk`] needs: a vector, the colour, and
/// the data buffer's description.
pub const WORK: u32 = 0x40;

mod work {
    pub const VEC: u32 = 0;
    pub const COLOR: u32 = 0x10;
    pub const NAME: u32 = 0x20;
    pub const COUNT: u32 = 0x30;
    pub const STRIDE: u32 = 0x34;
    pub const ONE: u32 = 0x38;
}

/// The sprite object's fields (`p`).
mod obj {
    /// The colour, 0..255 per channel (x, y, z, w as f32).
    pub const COLOR: u32 = 0x10;
    /// The local-to-view matrix the position goes through.
    pub const MATRIX: u32 = 0x28;
    /// Bit 0: the rim keeps the colour's alpha (else 0).
    pub const FLAGS: u32 = 0x3c;
    /// The VFX instance (the key's high word).
    pub const INSTANCE: u32 = 0x60;
}

/// The render data's description (`rd + 0x94`).
mod desc {
    pub const SEGMENT: u32 = 0x4;
    pub const VERTS: u32 = 0xc;
    pub const COLORS: u32 = 0x14;
    pub const UVS: u32 = 0x18;
    pub const INDICES: u32 = 0x1c;
    pub const KIND: u32 = 0x24;
}

/// The magic whose sprites draw in the Z-corrected pass.
const MAGIC_Z_CORRECTED: u32 = 0x92;

/// The window (projected x, y) the centre must lie in.
const WINDOW_X: (f64, f64) = (1707.0, 2389.0);
const WINDOW_Y: (f64, f64) = (1840.0, 2256.0);

fn ld(bits: u32) -> f64 {
    x87::load(bits)
}

fn st(x: f64) -> u32 {
    x87::store(x)
}

/// Rebased access to the globals.
struct G;

impl G {
    fn get<M: Mem>(mem: &M, at: u32) -> f64 {
        ld(mem.read::<u32>(mem.rebase(at)))
    }

    fn put<M: Mem>(mem: &mut M, at: u32, x: f64) -> u32 {
        let bits = st(x);
        let a = mem.rebase(at);
        mem.write(a, bits);
        bits
    }
}

/// The VFX instance key of the sprite: the current program and the
/// instance (with the program's sign carried, a 64-bit sum).
fn key<M: Mem>(mem: &M, p: u32) -> [u32; 2] {
    let lo = mem.read::<u32>(mem.rebase(ext::PROGRAM));
    let hi = mem.read::<u32>(p + obj::INSTANCE);
    let carry = if lo.cast_signed() < 0 { u32::MAX } else { 0 };
    [lo, hi.wrapping_add(carry)]
}

/// The screen mapping: 682 units across (512 and the cut scene's ratio
/// and bias while a 2D cut scene draws).
struct Screen {
    width: f64,
    ratio: f64,
    bias: f64,
}

/// One coordinate of a rim vertex or its UV offset: the register word
/// added to or subtracted from the centre's (or the UV's), or nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Plus(u32),
    Minus(u32),
    Keep,
}

/// A rim vertex: its x, y from the centre and its UV offset.
struct Rim {
    pos: [Step; 2],
    uv: [Step; 2],
}

/// The rim, in vertex order 1..=8: the axes, the diagonal, around.
const RIMS: [Rim; 8] = {
    use Step::{Keep, Minus as M, Plus as P};
    use vf::{RIM_A as A, RIM_B as B, RIM_C as C, UV_AXES as UA, UV_DIAG as UD};
    [
        Rim {
            pos: [P(A), P(A + 4)],
            uv: [Keep, P(UA + 4)],
        },
        Rim {
            pos: [P(B), P(B + 4)],
            uv: [P(UD), P(UD + 4)],
        },
        Rim {
            pos: [P(C), P(C + 4)],
            uv: [P(UA), Keep],
        },
        Rim {
            pos: [P(B), M(B + 4)],
            uv: [P(UD), M(UD + 4)],
        },
        Rim {
            pos: [M(A), M(A + 4)],
            uv: [Keep, M(UA + 4)],
        },
        Rim {
            pos: [M(B), M(B + 4)],
            uv: [M(UD), M(UD + 4)],
        },
        Rim {
            pos: [M(C), M(C + 4)],
            uv: [M(UA), Keep],
        },
        Rim {
            pos: [M(B), P(B + 4)],
            uv: [M(UD), P(UD + 4)],
        },
    ]
};

/// One draw of the distortion sprite `p` (`KeDttSpr_MkGifPk(p)`,
/// `0x00767790`).
///
/// The position `p + 0` goes through the sprite's matrix; the centre is
/// that times Q = `vf00.w / w`, as are the rim and UV offsets. A centre
/// off the window hides the instance. Otherwise the instance gets its
/// `"FrameBuffer"` data buffer (9 vertices of 0x18 bytes) the first time,
/// and the buffer segment is filled: the colour (over 255) for all nine,
/// the rim's alpha 0 unless `p + 0x3c` bit 0; the vertices (back over Q,
/// through the view matrix); the eight triangles `(0, i + 2, i + 1)` and
/// `(0, 1, 8)`; and each vertex's UV, `(x, y) - origin` plus its offset,
/// scaled into the frame buffer. The render data's matrix becomes the
/// fill matrix (translation cleared), its bounds ±100000, and the instance
/// is updated (Z-corrected for magic 0x92 in the VFX pass). Returns 0.
#[allow(clippy::too_many_lines)] // one pass, as the original
pub fn ke_dtt_spr_mk_gif_pk<M, C, F>(
    mem: &mut M,
    p: u32,
    work: u32,
    call: &mut C,
    fcall: &mut F,
) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
    F: FnMut(&mut M, u32, FArgs<'_>) -> f64,
{
    let r = |mem: &M, a: u32| mem.rebase(a);
    let geometry = call(mem, r(mem, ext::GET_DYN_GEOMETRY), &key(mem, p));
    let mut rd = if geometry == 0 {
        0
    } else {
        call(mem, r(mem, ext::GET_RENDER_DATA_BY_GEOMETRY), &[geometry])
    };
    let mut screen = Screen {
        width: 682.0,
        ratio: 1.0,
        bias: 0.0,
    };
    if call(mem, r(mem, ext::IS_IN_CUT_2D_SCENE), &[]) != 0 {
        screen.width = 512.0;
        let f = r(mem, ext::CUT_2D_SCENE_BIAS);
        screen.bias = ld(st(fcall(mem, f, FArgs::Stack(&[]))));
        screen.ratio = ld(0x3faa_aaaa); // 1.3333333
    }
    let first_draw = rd == 0;

    // the centre, projected
    let m = mem.read::<u32>(p + obj::MATRIX);
    call(mem, r(mem, ext::APPLY_MATRIX), &[p, m, p]);
    let [x, y, z, w] = mem.read::<[u32; 4]>(p);
    let center = r(mem, vf::CENTER);
    mem.write(center, [x, y, z, w]);
    let q = G::put(mem, vf::Q, G::get(mem, vf::VF00_W) / ld(w));
    let (q, inv_q) = (ld(q), st(1.0 / ld(q)));
    for at in [
        vf::CENTER,
        vf::CENTER + 4,
        vf::CENTER + 8,
        vf::RIM_A + 4,
        vf::RIM_B,
        vf::RIM_B + 4,
        vf::RIM_C,
    ] {
        G::put(mem, at, G::get(mem, at) * q);
    }
    let zw = [x87::copy(mem.read::<u32>(center + 8)), x87::copy(w)];
    let pos = r(mem, vf::POS);
    mem.write(pos + 8, zw);
    G::put(
        mem,
        vf::POS,
        G::get(mem, vf::CENTER) - G::get(mem, vf::RIM_C),
    );
    call(mem, r(mem, ext::RESTORE_VF00), &[]);
    G::put(
        mem,
        vf::POS + 4,
        G::get(mem, vf::CENTER + 4) - G::get(mem, vf::RIM_A + 4),
    );
    call(mem, r(mem, ext::RESTORE_VF00), &[]);

    // only a centre inside the window draws (the original also tests
    // 0..4096 for magic 0x6a and 0x1bd, but a second test with the window
    // follows for every magic, so the window decides; NaN passes)
    let (cx, cy) = (G::get(mem, vf::CENTER), G::get(mem, vf::CENTER + 4));
    if cx < WINDOW_X.0 || WINDOW_X.1 < cx || cy < WINDOW_Y.0 || WINDOW_Y.1 < cy {
        let [lo, hi] = key(mem, p);
        call(mem, r(mem, ext::VFX_SET_VISIBLE), &[lo, hi, 0]);
        return 0;
    }

    if first_draw {
        let name = work + work::NAME;
        mem.write(name, *b"FrameBuffer\0");
        mem.write(work + work::COUNT, 9u32);
        mem.write(work + work::STRIDE, 0x18u32);
        mem.write(work + work::ONE, 1u32);
        let [lo, hi] = key(mem, p);
        call(mem, r(mem, ext::VFX_CREATE), &[lo, hi, 0x1000]);
        let [lo, hi] = key(mem, p);
        call(
            mem,
            r(mem, ext::VFX_CREATE_DATA_BUFFER),
            &[
                lo,
                hi,
                1,
                work + work::COUNT,
                work + work::STRIDE,
                name,
                work + work::ONE,
            ],
        );
        rd = call(mem, r(mem, ext::GET_RENDER_DATA_BY_INSTANCE), &key(mem, p));
        if rd == 0 {
            return 0;
        }
        let d = mem.read::<u32>(rd + 0x94);
        mem.write(d + desc::KIND, 2u32);
        call(mem, r(mem, ext::VFX_SETUP), &key(mem, p));
    }
    let d = mem.read::<u32>(rd + 0x94);
    let [seg_a, seg_b] = mem.read::<[u32; 2]>(d + desc::SEGMENT);
    if call(
        mem,
        r(mem, ext::GET_BUFFER_FOR_SEGMENT),
        &[rd, 0, seg_a, seg_b],
    ) == 0
    {
        return 0;
    }

    // the UV offsets and origin follow Q
    for (dst, src) in [
        (vf::UV_AXES, vf::UV_AXES_IN),
        (vf::UV_AXES + 4, vf::UV_AXES_IN + 4),
        (vf::UV_DIAG, vf::UV_DIAG_IN),
        (vf::UV_DIAG + 4, vf::UV_DIAG_IN + 4),
    ] {
        G::put(mem, dst, G::get(mem, src) * q);
    }
    for i in 0..2 {
        let shift = G::put(mem, vf::MOVE + 4 * i, G::get(mem, vf::VF01 + 4 * i) * q);
        let at = vf::UV_ORIGIN + 4 * i;
        G::put(mem, at, G::get(mem, at) + ld(shift));
    }

    // the colour, over 255: the centre's, then the rim's
    let rgba = mem.read::<[u32; 4]>(p + obj::COLOR);
    let mv = r(mem, vf::MOVE);
    mem.write(mv, rgba);
    let color = work + work::COLOR;
    call(mem, r(mem, ext::COPY_VECTOR), &[color, mv]);
    let rgba = mem.read::<[u32; 4]>(color).map(|c| st(ld(c) / 255.0));
    let d = mem.read::<u32>(rd + 0x94);
    let colors = mem.read::<u32>(d + desc::COLORS);
    mem.write(colors, rgba);
    let mut rim_rgba = rgba;
    if mem.read::<u8>(p + obj::FLAGS) & 1 == 0 {
        rim_rgba[3] = 0;
    }
    for i in 1..9 {
        mem.write(colors + 0x10 * i, rim_rgba);
    }

    let d = mem.read::<u32>(rd + 0x94);
    let verts = mem.read::<u32>(d + desc::VERTS);
    let uvs = mem.read::<u32>(d + desc::UVS);
    let indices = mem.read::<u32>(d + desc::INDICES);
    let vec = work + work::VEC;

    // the centre
    project(mem, call, vec, center, inv_q);
    store_vertex(mem, verts, vec);
    store_triangle(mem, indices, 0);
    uv_from_origin(mem, call, center);
    store_uv(mem, uvs, &screen);

    // the rim
    for (n, rim) in (1u32..).zip(&RIMS) {
        rim_position(mem, call, rim);
        project(mem, call, vec, pos, inv_q);
        store_vertex(mem, verts + 0xc * n, vec);
        if n < 8 {
            store_triangle(mem, indices + 6 * n, n);
        }
        uv_from_origin(mem, call, pos);
        rim_uv(mem, call, rim);
        store_uv(mem, uvs + 8 * n, &screen);
    }

    // the render data: fill matrix without translation, wide bounds
    call(mem, r(mem, ext::UNIT_MATRIX), &[rd]);
    call(
        mem,
        r(mem, ext::MUL_MATRIX),
        &[rd, r(mem, ext::FILL_MATRIX), rd],
    );
    for at in [0xc, 0x1c, 0x2c] {
        mem.write(rd + at, 0u32);
    }
    mem.write(rd + 0x3c, 0x3f80_0000u32);
    mem.write(rd + 0x98, [0xc7c3_5000u32; 3]); // -100000
    mem.write(rd + 0xa4, [0x47c3_5000u32; 3]); // 100000
    let [lo, hi] = key(mem, p);
    call(mem, r(mem, ext::VFX_SET_VISIBLE), &[lo, hi, 1]);
    call(mem, r(mem, ext::USE_OPTIMIZED_SHADER), &[geometry, 0]);
    let magic = mem.read::<u32>(r(mem, ext::MAGIC_ID));
    if magic == MAGIC_Z_CORRECTED && mem.read::<u32>(r(mem, ext::VFX_PASS)) != 0 {
        let z = x87::copy(mem.read::<u32>(center + 8));
        let [lo, hi] = key(mem, p);
        call(mem, r(mem, ext::VFX_SET_Z_CORRECTION), &[lo, hi, z]);
        call(mem, r(mem, ext::VFX_UPDATE), &key(mem, p));
    } else {
        call(mem, r(mem, ext::VFX_DTT_UPDATE), &key(mem, p));
    }
    0
}

/// `vec = view * (src.xyz / Q, src.w)`: copied, scaled by `inv_q`
/// (`KeVu0ScaleVecXYZ` takes its scale as a vector; the original's other
/// three words are its caller's saved registers, the port's are 0), and
/// through the view matrix.
fn project<M, C>(mem: &mut M, call: &mut C, vec: u32, src: u32, inv_q: u32)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let r = |mem: &M, a: u32| mem.rebase(a);
    call(mem, r(mem, ext::COPY_VECTOR), &[vec, src]);
    call(mem, r(mem, ext::SCALE_VEC_XYZ), &[vec, vec, inv_q, 0, 0, 0]);
    call(
        mem,
        r(mem, ext::APPLY_MATRIX),
        &[vec, r(mem, ext::VIEW), vec],
    );
}

fn store_vertex<M: Mem>(mem: &mut M, at: u32, vec: u32) {
    let v = mem.read::<[u32; 3]>(vec).map(x87::copy);
    mem.write(at, v);
}

/// Triangle `n` of the fan: `(0, n + 2, n + 1)`, the last `(0, 1, 8)`.
fn store_triangle<M: Mem>(mem: &mut M, at: u32, n: u32) {
    let next = if n == 7 { 1 } else { n + 2 };
    #[allow(clippy::cast_possible_truncation)] // vertex numbers 0..9
    mem.write(at, [0u16, next as u16, (n + 1) as u16]);
}

/// [`vf::UV`] = `src.xy - origin.xy` (`Vec2SubScalarsFlags`, the origin
/// passed by value).
fn uv_from_origin<M, C>(mem: &mut M, call: &mut C, src: u32)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let r = |mem: &M, a: u32| mem.rebase(a);
    let o = mem.read::<[u32; 4]>(r(mem, vf::UV_ORIGIN));
    let uv = r(mem, vf::UV);
    call(
        mem,
        r(mem, ext::SUB_XY_FLAGS),
        &[uv, src, o[0], o[1], o[2], o[3]],
    );
}

/// `dst.xy = base.xy` stepped by `steps`. The original subtracts both
/// through `Vec2SubScalarsFlags` (the vector `steps` start at passed by
/// value; it sets the VU0 flags), else adds or subtracts one word at a
/// time, restoring vf00 after each subtraction.
fn step_xy<M, C>(mem: &mut M, call: &mut C, dst: u32, base: u32, steps: [Step; 2])
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let r = |mem: &M, a: u32| mem.rebase(a);
    if let [Step::Minus(at), Step::Minus(_)] = steps {
        let o = mem.read::<[u32; 4]>(r(mem, at));
        let (d, b) = (r(mem, dst), r(mem, base));
        call(
            mem,
            r(mem, ext::SUB_XY_FLAGS),
            &[d, b, o[0], o[1], o[2], o[3]],
        );
        return;
    }
    for (i, step) in (0u32..).zip(steps) {
        let b = G::get(mem, base + 4 * i);
        match step {
            Step::Plus(at) => {
                G::put(mem, dst + 4 * i, G::get(mem, at) + b);
            }
            Step::Minus(at) => {
                G::put(mem, dst + 4 * i, b - G::get(mem, at));
                call(mem, r(mem, ext::RESTORE_VF00), &[]);
            }
            Step::Keep => {}
        }
    }
}

/// The rim vertex's x, y into [`vf::POS`]: the centre's, stepped.
fn rim_position<M, C>(mem: &mut M, call: &mut C, rim: &Rim)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    step_xy(mem, call, vf::POS, vf::CENTER, rim.pos);
}

/// The rim vertex's UV offset onto [`vf::UV`].
fn rim_uv<M, C>(mem: &mut M, call: &mut C, rim: &Rim)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    step_xy(mem, call, vf::UV, vf::UV, rim.uv);
}

/// The vertex's UV from [`vf::UV`]: scaled into [`vf::UV_SCALED`], then
/// mapped from the PS2 screen (2048 at the centre, 1024 per unit) to the
/// frame buffer texture (0..1, y down).
fn store_uv<M: Mem>(mem: &mut M, at: u32, s: &Screen) {
    let k = G::get(mem, vf::UV_SCALE);
    let u = G::put(mem, vf::UV_SCALED, k * G::get(mem, vf::UV));
    let v = G::put(mem, vf::UV_SCALED + 4, k * G::get(mem, vf::UV + 4));
    let tu = st((ld(u) * 1024.0 - 2048.0) / s.width + 0.5);
    let tv = st(0.5 - (ld(v) * 1024.0 - 2048.0) / 448.0);
    let tv = st(ld(st(ld(tv) * s.ratio)) - s.bias);
    mem.write(at, [tu, tv]);
}
