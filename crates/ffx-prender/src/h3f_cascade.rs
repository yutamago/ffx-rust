//! The field shadow's cascade fit (M4, list `h3-glue-render`, agent h3f):
//! `ShadowManager_fitFieldCascades_006cebd0(this; light, solution)`, the
//! field sibling of `ShadowManager_fitBattleCascade_006d00d0`
//! ([`crate::h3b_glue9`]), called by `ShadowManager_updateCascades_006ce860`
//! outside battle.
//!
//! It builds the light's view (from the light's own 3x4 for solution 4,
//! else a fixed tilt of the light's inverse), writes the view's world
//! matrix into the manager's world object (`+0x8c`) and its inverse into
//! `+0x90`, then fits one cascade (two for solution 4): the light frustum
//! between the cascade's near and far distance (`+0x4034`, `+0x403c`, plus
//! the light's range), projected, bounded, an orthographic box around it
//! (`+0x110`), the box times the view (`+0x190`) and that with the texture
//! bias (`+0x210`). 64 bytes per cascade in each.
//!
//! x87: the original keeps the light scales `-sy, -sx, sx, sy` on the FPU
//! stack across the cascade loop; they are exactly the `f32` values it
//! stores in the frame (`- 0x18c`, `- 0x188`, `- 0x118`, `- 0x10c`) and
//! reloads at each iteration's end, so the port carries them as `f64`
//! locals. Every other value is rounded where the original stores it.

pub use crate::h3b_glue9::{
    C_FAR, C_HALF, C_MAX, C_MIN, C_NEAR, COPY, COPY_453340, FROM_3X4, INVERSE, MUL_THIS, MULTIPLY,
    ORTHO, ROTATION_X, ROW_TIMES, TO_WORLD, TRANSFORM3,
};

/// `Phyre_Vector4_normalize_005744a0(out, &v)` (cdecl).
pub const NORMALIZE: u32 = 0x0057_44a0;
/// `PMatrix4_FromRows_00450d40(m; r0, r1, r2, r3)`.
pub const FROM_ROWS: u32 = 0x0045_0d40;

/// Constants: -500.0 (the view's `[13]` for solution 4), -5.0 (the tilt's
/// `[13]`), -1.5708 (the tilt angle), 0.01 (f64, the bounds' margin), 0.5
/// (`f32`, the bias), 100.0 (f64); the twelve frustum edges (pairs of
/// corner indices).
pub const C_MINUS_500: u32 = 0x00b4_99bc;
pub const C_MINUS_5: u32 = 0x00b4_99b8;
pub const C_TILT: u32 = 0x00b4_99a0;
pub const C_MARGIN: u32 = 0x00b1_f2b8;
pub const C_HALF_F32: u32 = 0x00b9_2194;
pub const C_HUNDRED: u32 = 0x00b9_22e0;
pub const EDGES: u32 = 0x00b4_9868;

/// The frame: bytes below `ebp` (the cookie at `- 4`).
pub const FRAME: u32 = 0x9a0;
/// The view (`- 0x23c`) and its second matrix (`- 0x1fc`), the inverse
/// world (`- 0x3f0`), the light-space corners (`- 0x84`, 16 bytes apart)
/// and their projections (`- 0x104`).
pub const VIEW: u32 = 0x23c;
pub const VIEW2: u32 = 0x1fc;
pub const INV_WORLD: u32 = 0x3f0;
pub const CORNERS: u32 = 0x84;
pub const PROJECTED: u32 = 0x104;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn f32_at(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn f64_at(h: &impl Host, at: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(at)))
    }

    /// A float constant of the image as `fld`/`fstp` copies it.
    fn constant(h: &impl Host, at: u32) -> u32 {
        x87::store(f32_at(h, h.rebase(at)))
    }

    /// `n` floats copied through the x87 (`fld`/`fstp dword`).
    fn copy(h: &mut impl Host, to: u32, from: u32, n: u32) {
        for k in 0..n {
            let v = x87::store(f32_at(h, from + 4 * k));
            h.write(to + 4 * k, v);
        }
    }

    /// `n` words copied with `mov`.
    fn copy_words(h: &mut impl Host, to: u32, from: u32, n: u32) {
        for k in 0..n {
            let w: u32 = h.read(from + 4 * k);
            h.write(to + 4 * k, w);
        }
    }

    fn put(h: &mut impl Host, at: u32, v: f64) -> u32 {
        let bits = x87::store(v);
        h.write(at, bits);
        bits
    }

    /// Writes `v` at each `ebp - off`.
    fn fill(h: &mut impl Host, ebp: u32, offs: &[u32], v: u32) {
        for &off in offs {
            h.write(ebp - off, v);
        }
    }

    /// The original's NaN test of a float: copied to `- 0x10c`, `- 0x118`
    /// and `- 0x114`, then compared with itself (`fucompp`).
    fn is_nan(h: &mut impl Host, ebp: u32, at: u32) -> bool {
        let v = x87::store(f32_at(h, at));
        fill(h, ebp, &[0x10c, 0x118, 0x114], v);
        f32::from_bits(v).is_nan()
    }

    /// `ShadowManager_fitFieldCascades_006cebd0(this; light, solution)`
    /// (`ret 8`, no result; frame at `ebp`, see the module's offsets).
    pub fn fit_field_cascades(h: &mut impl Host, ebp: u32, this: u32, light: u32, solution: u32) {
        h.write(ebp - 0x250, light);
        if solution == 4 {
            light_view(h, ebp, this, light);
        } else {
            tilted_view(h, ebp, light);
        }

        // the view's world matrix into the world object, its inverse into
        // +0x90
        call(h, TO_WORLD, ebp - 0x460, &[ebp - VIEW]);
        let world: u32 = h.read(this + 0x8c);
        copy(h, world, ebp - 0x460, 12);
        let world: u32 = h.read(this + 0x8c);
        call(h, FROM_3X4, ebp - 0x580, &[world]);
        fill(h, ebp, &[0x550, 0x54c, 0x548], 0);
        let inv = h.call_sym(INVERSE, Conv::Cdecl, &[ebp - 0x8e0, ebp - 0x580]);
        call(h, COPY, ebp - 0x2d0, &[inv]);
        copy(h, ebp - INV_WORLD, ebp - 0x2d0, 16);
        copy(h, this + 0x90, ebp - 0x2d0, 16);

        if is_nan(h, ebp, ebp - VIEW2) {
            // a singular view: the bias is the identity
            identity(h, ebp - 0x520, 0);
            call(h, COPY_453340, this + 0x190, &[ebp - 0x520]);
            return;
        }
        call(h, MUL_THIS, ebp - INV_WORLD, &[ebp - 0x660, ebp - VIEW2]);
        call(h, COPY_453340, ebp - 0x5d0, &[light + 0x30]);

        // the light's scales: reciprocals of its projection's [0] and [5]
        // (0 for 0)
        let reciprocal = |v: f64| {
            #[allow(clippy::float_cmp)] // fucom against 0: only equality
            if v == 0.0 { 0.0 } else { x87::div(1.0, v) }
        };
        let sx = put(h, ebp - 0x118, reciprocal(f32_at(h, ebp - 0x5d0)));
        let sy = put(h, ebp - 0x10c, reciprocal(f32_at(h, ebp - 0x5bc)));
        let count = if solution == 4 { 2 } else { 1 };
        h.write(ebp - 0x114, count);
        h.write(ebp - 0x124, 0u32);
        let (sx, sy) = (x87::load(sx), x87::load(sy));
        let (nx, ny) = (put(h, ebp - 0x188, -sx), put(h, ebp - 0x18c, -sy));
        let scales = Scales {
            sx,
            sy,
            nx: x87::load(nx),
            ny: x87::load(ny),
        };
        let q = put(h, ebp - 0x538, x87::add(0.0, f64_at(h, C_HALF)));
        for i in 0..count {
            fit_cascade(h, ebp, this, light, solution, i, &scales, x87::load(q));
            h.write(ebp - 0x124, i + 1);
        }
    }

    /// The light's scales as the original keeps them on the FPU stack, and
    /// their negations.
    struct Scales {
        sx: f64,
        sy: f64,
        nx: f64,
        ny: f64,
    }

    /// The frame 4x4 at `m` set to the identity with `w` as `[13]`.
    fn identity(h: &mut impl Host, m: u32, w: u32) {
        for k in 0..16 {
            let v = match k {
                0 | 5 | 10 | 15 => 0x3f80_0000,
                13 => w,
                _ => 0,
            };
            h.write(m + 4 * k, v);
        }
    }

    /// Solution 4: the view from the light's own 3x4 (`[light + 0xb0]`):
    /// its first row `m`, its third row `f` (y cleared, normalized) and the
    /// manager's up vector `u` (`+0x4020`, normalized) give the rows
    /// `cross(cross(m, u), u)`, `cross(m, u)`, `u` of the view (`- 0x23c`,
    /// translation `(0, -500, 0)`), and a second matrix (`- 0x1fc`) of `m`,
    /// `(0, 1, 0, m[7])`, `f` and `m`'s translation with y 0.
    #[allow(clippy::many_single_char_names)] // the vectors
    fn light_view(h: &mut impl Host, ebp: u32, this: u32, light: u32) {
        let m3x4: u32 = h.read(light + 0xb0);
        let lm = ebp - 0x290;
        call(h, FROM_3X4, lm, &[m3x4]);
        if is_nan(h, ebp, lm) {
            // the identity's non-zero words and some of its zeros
            fill(h, ebp, &[0x290, 0x268, 0x254], 0x3f80_0000);
            fill(
                h,
                ebp,
                &[
                    0x28c, 0x288, 0x284, 0x274, 0x270, 0x26c, 0x264, 0x260, 0x258,
                ],
                0,
            );
        }
        // f = (m[8], 0, m[10], m[11]), normalized
        copy_words(h, ebp - 0x19c, lm + 0x20, 4);
        h.write(ebp - 0x198, 0u32);
        let r = h.call_sym(NORMALIZE, Conv::Cdecl, &[ebp - 0x600, ebp - 0x19c]);
        copy(h, ebp - 0x19c, r, 4);
        // u = the manager's +0x4020, normalized
        copy_words(h, ebp - 0x184, this + 0x4020, 4);
        let r = h.call_sym(NORMALIZE, Conv::Cdecl, &[ebp - 0x610, ebp - 0x184]);
        copy(h, ebp - 0x184, r, 4);

        let (sub, mul) = (x87::sub, x87::mul);
        let m = [0x290, 0x28c, 0x288].map(|at| f32_at(h, ebp - at));
        let u = [0x184, 0x180, 0x17c].map(|at| f32_at(h, ebp - at));
        let c = [
            sub(mul(u[2], m[1]), mul(u[1], m[2])),
            sub(mul(m[2], u[0]), mul(u[2], m[0])),
            sub(mul(m[0], u[1]), mul(m[1], u[0])),
        ];
        let c = [
            put(h, ebp - 0x174, c[0]),
            put(h, ebp - 0x170, c[1]),
            put(h, ebp - 0x16c, c[2]),
        ];
        let v = ebp - VIEW;
        h.write(v + 0x3c, 0x3f80_0000u32);
        for (k, w) in (0..).zip(c) {
            h.write(v + 0x10 + 4 * k, w);
        }
        h.write(v + 0x1c, 0u32);
        // row 2: u (its x, z and w as the stack and memory hold them)
        put(h, v + 0x20, u[0]);
        copy(h, v + 0x24, ebp - 0x180, 1);
        put(h, v + 0x28, u[2]);
        copy(h, v + 0x2c, ebp - 0x178, 1);
        let c = c.map(x87::load);
        let d = [
            sub(mul(c[1], u[2]), mul(c[2], u[1])),
            sub(mul(c[2], u[0]), mul(u[2], c[0])),
            sub(mul(c[0], u[1]), mul(c[1], u[0])),
        ];
        put(h, ebp - 0x174, d[0]);
        put(h, ebp - 0x170, d[1]);
        put(h, ebp - 0x16c, d[2]);
        copy(h, v, ebp - 0x174, 3);
        h.write(v + 0xc, 0u32);
        h.write(v + 0x30, 0u32);
        let c500 = constant(h, C_MINUS_500);
        h.write(v + 0x34, c500);
        h.write(v + 0x38, 0u32);

        let v2 = ebp - VIEW2;
        copy(h, v2, lm, 4);
        h.write(v2 + 0x10, 0u32);
        h.write(v2 + 0x14, 0x3f80_0000u32);
        h.write(v2 + 0x18, 0u32);
        copy(h, v2 + 0x1c, lm + 0x1c, 1);
        copy(h, v2 + 0x20, ebp - 0x19c, 4);
        copy(h, v2 + 0x30, lm + 0x30, 1);
        h.write(v2 + 0x34, 0u32);
        copy(h, v2 + 0x38, lm + 0x38, 2);
    }

    /// Other solutions: the view is an identity with `[13] = -5` times a
    /// rotation about x by -1.5708; the second matrix the inverse of the
    /// light's 3x4.
    fn tilted_view(h: &mut impl Host, ebp: u32, light: u32) {
        let c5 = constant(h, C_MINUS_5);
        identity(h, ebp - 0x4a0, c5);
        let angle = constant(h, C_TILT);
        let r = h.call_sym(ROTATION_X, Conv::Cdecl, &[ebp - 0x7a0, angle]);
        call(h, COPY, ebp - 0x6a0, &[r]);
        let r = call(h, MUL_THIS, ebp - 0x4a0, &[ebp - 0x920, ebp - 0x6a0]);
        copy(h, ebp - VIEW, r, 16);
        call(h, FROM_3X4, ebp - 0x6e0, &[light]);
        let inv = h.call_sym(INVERSE, Conv::Cdecl, &[ebp - 0x9a0, ebp - 0x6e0]);
        call(h, COPY, ebp - 0x4e0, &[inv]);
        copy(h, ebp - VIEW2, ebp - 0x4e0, 16);
    }

    /// Cascade `i`: the frustum, its bounds, the box and the three
    /// matrices.
    #[allow(clippy::too_many_arguments)] // the original's state
    fn fit_cascade(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        light: u32,
        solution: u32,
        i: u32,
        s: &Scales,
        q: f64,
    ) {
        let range = f32_at(h, light + 0xb4);
        let near = put(
            h,
            ebp - 0x108,
            x87::add(f32_at(h, this + 0x4034 + 4 * i), range),
        );
        let far = put(
            h,
            ebp - 0x110,
            x87::add(f32_at(h, this + 0x403c + 4 * i), range),
        );
        frustum(h, ebp, s, x87::load(near), x87::load(far));
        longest_edge(h, ebp);
        let (lo, hi) = bounds(h, ebp);

        // the box: the bounds widened by the margin
        let k = f64_at(h, C_MARGIN);
        let lo = [
            put(h, ebp - 0x174, x87::sub(lo[0], k)),
            put(h, ebp - 0x170, x87::sub(lo[1], k)),
            put(h, ebp - 0x16c, x87::sub(lo[2], k)),
        ];
        let hi = [
            put(h, ebp - 0x24c, x87::add(hi[0], k)),
            put(h, ebp - 0x248, x87::add(hi[1], k)),
            put(h, ebp - 0x244, x87::add(hi[2], k)),
        ];
        let box_ = ortho_box(h, ebp, this, solution, i, lo, hi);
        copy(h, ebp - 0x164, box_, 16);
        let r = h.call_sym(
            MULTIPLY,
            Conv::Cdecl,
            &[ebp - 0x820, ebp - 0x164, ebp - 0x430],
        );
        copy(h, ebp - 0x164, r, 16);
        copy(h, this + 0x110 + 0x40 * i, ebp - 0x164, 16);

        // the texture bias (- 0x3b0) and the box times the view (+0x190)
        let half = constant(h, C_HALF_F32);
        let b = ebp - 0x3b0;
        for k in 0..16 {
            h.write(b + 4 * k, 0u32);
        }
        for k in [0, 5, 10, 12, 13] {
            h.write(b + 4 * k, half);
        }
        h.write(b + 0x3c, 0x3f80_0000u32);
        copy(h, b + 0x38, ebp - 0x538, 1);
        let r = h.call_sym(
            MULTIPLY,
            Conv::Cdecl,
            &[ebp - 0x8a0, ebp - 0x164, ebp - INV_WORLD],
        );
        let m = this + 0x190 + 0x40 * i;
        copy(h, m, r, 16);
        bias_row(h, ebp, m, q);
        let r2 = call(h, ROW_TIMES, b, &[ebp - 0x590, m + 0x20]);
        let r1 = call(h, ROW_TIMES, b, &[ebp - 0x5e0, m + 0x10]);
        let r0 = call(h, ROW_TIMES, b, &[ebp - 0x620, m]);
        call(h, FROM_ROWS, ebp - 0x720, &[r0, r1, r2, ebp - 0x370]);
        call(h, COPY_453340, this + 0x210 + 0x40 * i, &[ebp - 0x720]);
    }

    /// The light frustum's eight corners between `near` and `far`
    /// (`ebp - 0x84`, 16 bytes apart, three floats): `x = -+sx d`,
    /// `y = -+sy d`, `z = -d`, with the original's scratch copies (its
    /// scratch word `- 0x108` is overwritten by [`longest_edge`]).
    fn frustum(h: &mut impl Host, ebp: u32, s: &Scales, near: f64, far: f64) {
        // the scratch triple of each corner, then the corner
        const SLOTS: [[u32; 2]; 8] = [
            [0x360, 0x84],
            [0x350, 0x74],
            [0x320, 0x64],
            [0x340, 0x54],
            [0x2e0, 0x44],
            [0x2f0, 0x34],
            [0x310, 0x24],
            [0x330, 0x14],
        ];
        let mut corners = [[0u32; 3]; 8];
        for (k, d) in [near, far].into_iter().enumerate() {
            let x0 = x87::store(x87::mul(s.nx, d));
            let y0 = x87::store(x87::mul(s.ny, d));
            let z = x87::store(-d);
            let x1 = x87::store(x87::mul(d, s.sx));
            let y1 = x87::store(x87::mul(d, s.sy));
            let c = &mut corners[4 * k..4 * k + 4];
            c[0] = [x0, y0, z];
            c[1] = [x1, y0, z];
            c[2] = [x1, y1, z];
            c[3] = [x0, y1, z];
        }
        for (c, [scratch, at]) in corners.into_iter().zip(SLOTS) {
            for (w, v) in (0..).zip(c) {
                h.write(ebp - scratch + 4 * w, v);
                h.write(ebp - at + 4 * w, v);
            }
        }
    }

    /// Each corner through the light's view (`- 0x660`) into `x, y, 0` at
    /// `- 0x104`; the longest squared edge (table [`EDGES`]) into
    /// `- 0x11c`, which nothing reads.
    fn longest_edge(h: &mut impl Host, ebp: u32) {
        for k in 0..8 {
            let p = call(
                h,
                TRANSFORM3,
                ebp - 0x660,
                &[ebp - 0x5f0, ebp - CORNERS + 0x10 * k],
            );
            let at = ebp - PROJECTED + 0x10 * k;
            copy(h, at, p, 2);
            h.write(at + 8, 0u32);
        }
        h.write(ebp - 0x11c, 0u32);
        let edges = h.rebase(EDGES);
        for e in 0..12 {
            let a: u32 = h.read(edges + 8 * e);
            let b: u32 = h.read(edges + 8 * e + 4);
            let (pa, pb) = (
                (ebp - PROJECTED).wrapping_add(a.wrapping_mul(16)),
                (ebp - PROJECTED).wrapping_add(b.wrapping_mul(16)),
            );
            let mut d = [0u32; 3];
            for (k, v) in (0..).zip(d.iter_mut()) {
                *v = put(
                    h,
                    ebp - 0x300 + 4 * k,
                    x87::sub(f32_at(h, pb + 4 * k), f32_at(h, pa + 4 * k)),
                );
            }
            let [dx, dy, dz] = d.map(x87::load);
            let s = put(h, ebp - 0x108, x87::mul(dx, dx));
            let s = put(h, ebp - 0x108, x87::add(x87::load(s), x87::mul(dy, dy)));
            let t = put(h, ebp - 0x108, x87::add(x87::load(s), x87::mul(dz, dz)));
            // fcomp + test 0x5: the new length only when longer
            if f32_at(h, ebp - 0x11c) < x87::load(t) {
                h.write(ebp - 0x11c, t);
            }
        }
    }

    /// The corners' bounds in the light's view times the identity
    /// (`- 0x760`): minimum (`- 0x1ac`) and maximum (`- 0x1bc`) in the
    /// frame, and as the original leaves them on the FPU stack.
    fn bounds(h: &mut impl Host, ebp: u32) -> ([f64; 3], [f64; 3]) {
        identity(h, ebp - 0x430, 0);
        call(h, MUL_THIS, ebp - 0x430, &[ebp - 0x760, ebp - 0x660]);
        let max = constant(h, C_MAX);
        let min = constant(h, C_MIN);
        fill(h, ebp, &[0x1ac, 0x1a8, 0x1a4], max);
        fill(h, ebp, &[0x1bc, 0x1b8, 0x1b4], min);
        // fcom + test 0x41: the new value unless it is greater (smaller)
        let low = |v: f64, m: f64| if v > m { m } else { v };
        let high = |v: f64, m: f64| if m > v { m } else { v };
        let (mut lo, mut hi) = ([0.0; 3], [0.0; 3]);
        for k in 0..8 {
            call(
                h,
                TRANSFORM3,
                ebp - 0x760,
                &[ebp - 0x530, ebp - CORNERS + 0x10 * k],
            );
            let p = [0x530, 0x52c, 0x528].map(|at| f32_at(h, ebp - at));
            let m = [0x1ac, 0x1a8, 0x1a4].map(|at| f32_at(h, ebp - at));
            let n = [low(p[0], m[0]), low(p[1], m[1]), low(p[2], m[2])].map(x87::store);
            let m = [0x1bc, 0x1b8, 0x1b4].map(|at| f32_at(h, ebp - at));
            let x = [high(p[0], m[0]), high(p[1], m[1]), high(p[2], m[2])].map(x87::store);
            // the scratch words: the minimum's z, then the maximum's
            h.write(ebp - 0x11c, n[2]);
            h.write(ebp - 0x110, x[0]);
            h.write(ebp - 0x120, x[1]);
            h.write(ebp - 0x108, x[2]);
            for (k, off) in [0x1ac, 0x1a8, 0x1a4].into_iter().enumerate() {
                h.write(ebp - off, n[k]);
            }
            for (k, off) in [0x1bc, 0x1b8, 0x1b4].into_iter().enumerate() {
                h.write(ebp - off, x[k]);
            }
            lo = n.map(x87::load);
            hi = x.map(x87::load);
        }
        (lo, hi)
    }

    /// The orthographic box (`PMatrix4_orthographic_00455dc0`); its
    /// address. Solution 4 takes the bounds' x and y with z -1000..1000,
    /// or for its first cascade with `+0x4054` set a 300-unit square
    /// around the manager's point `(+0x4058, +0x405c)`; other solutions
    /// take the bounds, z from -(max z + 100) to -(min z).
    #[allow(clippy::too_many_arguments)] // the original's state
    fn ortho_box(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        solution: u32,
        i: u32,
        lo: [u32; 3],
        hi: [u32; 3],
    ) -> u32 {
        let (near, far) = (constant(h, C_NEAR), constant(h, C_FAR));
        if solution != 4 {
            let far = x87::store(-x87::load(lo[2]));
            let near = put(
                h,
                ebp - 0x108,
                x87::sub(-x87::load(hi[2]), f64_at(h, C_HUNDRED)),
            );
            return h.call_sym(
                ORTHO,
                Conv::Cdecl,
                &[ebp - 0x7e0, lo[0], hi[0], lo[1], hi[1], near, far],
            );
        }
        let point: u8 = h.read(this + 0x4054);
        if point == 0 || i != 0 {
            return h.call_sym(
                ORTHO,
                Conv::Cdecl,
                &[ebp - 0x960, lo[0], hi[0], lo[1], hi[1], near, far],
            );
        }
        let x: u32 = h.read(this + 0x4058);
        let y: u32 = h.read(this + 0x405c);
        // fild (signed), fstp dword: one rounding
        #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
        let f = |v: u32| (v as i32 as f32).to_bits();
        let side = |v: u32| (f(v.wrapping_sub(150)), f(v.wrapping_add(150)));
        let (bottom, top) = side(y);
        let (left, right) = side(x);
        // (the original's ints and floats in - 0x108 are overwritten by
        // the second cascade, which solution 4 always has)
        h.call_sym(
            ORTHO,
            Conv::Cdecl,
            &[ebp - 0x860, left, right, bottom, top, near, far],
        )
    }

    /// Row 3 of `m`'s bias (`- 0x370`): `m[12..16]` times the half, with
    /// the products by 0 the original keeps (they carry NaNs and
    /// infinities), in its x87 order; `q` the bias's `[14]`.
    fn bias_row(h: &mut impl Host, ebp: u32, m: u32, q: f64) {
        let (add, mul) = (x87::add, x87::mul);
        let half = f64_at(h, C_HALF);
        let [a, b, c, d] = [0x30, 0x34, 0x38, 0x3c].map(|k| f32_at(h, m + k));
        let (za, zb, zc) = (mul(a, 0.0), mul(b, 0.0), mul(c, 0.0));
        let (ha, hb, hc, hd) = (mul(a, half), mul(b, half), mul(c, half), mul(d, half));
        put(h, ebp - 0x370, add(add(add(ha, zb), zc), hd));
        put(h, ebp - 0x36c, add(hd, add(add(hb, za), zc)));
        put(h, ebp - 0x368, add(add(hc, add(zb, za)), mul(d, q)));
        put(h, ebp - 0x364, add(add(zc, add(zb, za)), d));
    }
}
