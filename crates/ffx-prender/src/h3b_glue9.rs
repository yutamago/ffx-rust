//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b, round 2): `ShadowManager_fitBattleCascade_006d00d0`, the
//! battle shadow's light camera (`ShadowManager_updateCascades_006ce860`
//! in battle): the light's rotation from the manager's two angles, the
//! light's view and its inverse, the corners of the light frustum, an
//! orthographic box around the focus and the texture-space bias.

/// The two floats `0x006c8480(this; &x, &y)` stores (the focus).
pub const FOCUS: u32 = 0x006c_8480;
/// `Matrix_RotationX/Y(out, angle)` (cdecl).
pub const ROTATION_X: u32 = 0x0066_c8a0;
pub const ROTATION_Y: u32 = 0x0066_c920;
/// `PMatrix4_RowTimes_004c9c20(m; out, row)`.
pub const ROW_TIMES: u32 = 0x004c_9c20;
/// `PMatrix4_Copy_004c9430(out; m)`, `PMatrix4_Copy_00453340(out; m)`.
pub const COPY: u32 = 0x004c_9430;
pub const COPY_453340: u32 = 0x0045_3340;
/// `PMatrix4_MulThis_004c9da0(a; out, b)`.
pub const MUL_THIS: u32 = 0x004c_9da0;
/// `PMatrix4_toWorldMatrix3x4(out; m)`.
pub const TO_WORLD: u32 = 0x0045_1190;
/// `PMatrix4_Rotate3_004ca140(m; out, v)`.
pub const ROTATE3: u32 = 0x004c_a140;
/// `PMatrix4_FromMatrix3x4_00451080(out; m3x4)`.
pub const FROM_3X4: u32 = 0x0045_1080;
/// `PMatrix4_Inverse_004cc0c0(out, m)` (cdecl).
pub const INVERSE: u32 = 0x004c_c0c0;
/// `PMatrix4_TransformPoint3_004c9eb0(m; out, p)`.
pub const TRANSFORM3: u32 = 0x004c_9eb0;
/// `PMatrix4_orthographic_00455dc0(out, left, right, bottom, top, near,
/// far)` (cdecl).
pub const ORTHO: u32 = 0x0045_5dc0;
/// `PMatrix4_Multiply_004ca360(out, a, b)` (cdecl).
pub const MULTIPLY: u32 = 0x004c_a360;

/// Constants: 500.0, 360.0 (f64), 2 pi as 6.283185 (f64), 90.0, the far
/// distance global, `FLT_MAX`, `-FLT_MAX`, 210.0 (f64), -1000.0, 1000.0, 0.5
/// (f64); the twelve frustum edges (pairs of corner indices).
pub const C_500: u32 = 0x00b4_998c;
pub const C_360: u32 = 0x00b4_8380;
pub const C_TWO_PI: u32 = 0x00b4_9960;
pub const C_90: u32 = 0x00b4_997c;
pub const FAR_DISTANCE: u32 = 0x00c3_4400;
pub const C_MAX: u32 = 0x00b1_7504;
pub const C_MIN: u32 = 0x00b1_f2d0;
pub const C_210: u32 = 0x00b4_9970;
pub const C_NEAR: u32 = 0x00b4_5bbc;
pub const C_FAR: u32 = 0x00b1_f2cc;
pub const C_HALF: u32 = 0x00b9_2220;
pub const EDGES: u32 = 0x00b4_9808;

/// The frame (`ebp` offsets, below `ebp`): the bytes the port uses (the
/// cookie at `- 4`).
pub const FRAME: u32 = 0x21c;
/// The frame's 4x4s: A (`- 0x21c`), B (`- 0x1bc`), C (`- 0x16c`, its
/// rows 1-3 also the 3x4 at `- 0x15c`), D (`- 0xc4`), E (`- 0x44`,
/// shared with the last four corners).
pub const MA: u32 = 0x21c;
pub const MB: u32 = 0x1bc;
pub const MC: u32 = 0x16c;
pub const MD: u32 = 0xc4;
pub const ME: u32 = 0x44;
/// The eight light-space corners (16 bytes apart, three floats) and their
/// transforms (`x, y, 0`).
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

    /// `n` floats copied through the x87 (`fld`/`fstp dword`).
    fn copy(h: &mut impl Host, to: u32, from: u32, n: u32) {
        for k in 0..n {
            let v = x87::store(f32_at(h, from + 4 * k));
            h.write(to + 4 * k, v);
        }
    }

    fn put(h: &mut impl Host, at: u32, v: f64) -> u32 {
        let bits = x87::store(v);
        h.write(at, bits);
        bits
    }

    /// The frame 4x4 at `m` set to the identity with `w` as `[13]` (the
    /// original's store order).
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

    /// `ShadowManager_fitBattleCascade_006d00d0(this; light, focus)` (`ret
    /// 8`; frame at `ebp`, see the module's offsets).
    #[allow(clippy::too_many_lines)] // the original's sequence
    pub fn fit_battle_cascade(h: &mut impl Host, ebp: u32, this: u32, light: u32, focus: u8) {
        let (ma, mb, mc, md, me) = (ebp - MA, ebp - MB, ebp - MC, ebp - MD, ebp - ME);
        h.write(ebp - 0x11c, 0u32);
        h.write(ebp - 0x120, 0u32);
        h.write(ebp - 0x12c, this);
        h.write(ebp - 0x108, light);
        if focus != 0 {
            call(h, FOCUS, this, &[ebp - 0x11c, ebp - 0x120]);
        }
        let c500 = x87::store(f32_at(h, h.rebase(C_500)));
        identity(h, ma, c500);

        // the light's rotation: x from +0x4018, y from -+0x401c (degrees)
        let (c360, two_pi) = (f64_at(h, C_360), f64_at(h, C_TWO_PI));
        let angle = |v: f64| x87::mul(x87::div(v, c360), two_pi);
        let ax = put(h, ebp - 0x124, angle(f32_at(h, this + 0x4018)));
        let rx = h.call_sym(ROTATION_X, Conv::Cdecl, &[md, ax]);
        h.write(ebp - 0x128, rx);
        let ay = put(h, ebp - 0x124, angle(-f32_at(h, this + 0x401c)));
        let ry = h.call_sym(ROTATION_Y, Conv::Cdecl, &[me, ay]);
        let r3 = call(h, ROW_TIMES, ry, &[ebp - 0x118, rx.wrapping_add(0x30)]);
        h.write(ebp - 0x124, r3);
        let r2 = call(h, ROW_TIMES, ry, &[ebp - 0x17c, rx.wrapping_add(0x20)]);
        let r1 = call(h, ROW_TIMES, ry, &[ebp - 0x1cc, rx.wrapping_add(0x10)]);
        let r0 = call(h, ROW_TIMES, ry, &[ebp - 0x1dc, rx]);
        for (k, row) in (0..).zip([r0, r1, r2, r3]) {
            copy(h, mb + 0x10 * k, row, 4);
        }
        call(h, COPY, mc, &[mb]);
        let c90 = x87::store(f32_at(h, h.rebase(C_90)));
        h.write(this + 0x4018, c90);
        h.write(this + 0x401c, 0u32);

        // the light's world matrix into +0x8c, the focus rotated
        let r = call(h, MUL_THIS, ma, &[me, mc]);
        copy(h, ma, r, 16);
        call(h, TO_WORLD, ebp - 0x15c, &[ma]);
        let world: u32 = h.read(this + 0x8c);
        copy(h, world, ebp - 0x15c, 12);
        let fx = x87::store(f32_at(h, ebp - 0x11c));
        h.write(ebp - 0x17c, fx);
        h.write(ebp - 0x178, 0u32);
        let fy = x87::store(f32_at(h, ebp - 0x120));
        h.write(ebp - 0x174, fy);
        let r = call(h, ROTATE3, ma, &[ebp - 0x118, ebp - 0x17c]);
        copy(h, ebp - 0x17c, r, 2);
        call(h, FROM_3X4, mb, &[world]);
        for at in [0x18c, 0x188, 0x184] {
            h.write(ebp - at, 0u32);
        }
        let inv = h.call_sym(INVERSE, Conv::Cdecl, &[me, mb]);
        call(h, COPY, mc, &[inv]);
        copy(h, mb, mc, 16);
        copy(h, this + 0x90, mc, 16);
        call(h, FROM_3X4, ma, &[light]);
        let inv = h.call_sym(INVERSE, Conv::Cdecl, &[me, ma]);
        call(h, COPY, ma, &[inv]);
        let first = x87::store(f32_at(h, ma));
        for at in [0x108, 0x12c, 0x128] {
            h.write(ebp - at, first);
        }
        if f32::from_bits(first).is_nan() {
            // a singular light: the bias is the identity
            identity(h, mb, 0);
            call(h, COPY_453340, this + 0x190, &[mb]);
            return;
        }
        call(h, MUL_THIS, mb, &[mc, ma]);
        frustum(h, ebp, light);
        bounds(h, ebp, mc, md, ma);

        // the orthographic box around the rotated focus
        let (vx, vy) = (f32_at(h, ebp - 0x17c), f32_at(h, ebp - 0x178));
        let c210 = f64_at(h, C_210);
        let near = x87::store(f32_at(h, h.rebase(C_NEAR)));
        let far = x87::store(f32_at(h, h.rebase(C_FAR)));
        let top = x87::store(x87::sub(-vy, c210));
        let bottom = x87::store(x87::sub(c210, vy));
        let right = x87::store(x87::add(vx, c210));
        let left = x87::store(x87::sub(vx, c210));
        h.write(ebp - 0x108, left);
        h.call_sym(
            ORTHO,
            Conv::Cdecl,
            &[mc, left, right, bottom, top, near, far],
        );
        let r = h.call_sym(MULTIPLY, Conv::Cdecl, &[me, mc, ma]);
        copy(h, mc, r, 16);
        copy(h, this + 0x110, mc, 16);
        let half = f64_at(h, C_HALF);
        put(h, ebp - 0x174, x87::add(0.0, half));
        let r = h.call_sym(MULTIPLY, Conv::Cdecl, &[me, mc, mb]);
        copy(h, this + 0x190, r, 16);
        bias(h, ebp, this, half);
        call(h, COPY_453340, this + 0x210, &[mb]);
    }

    /// The light frustum's eight corners (`ebp - 0x84`, 16 bytes apart):
    /// `x = -+rx d`, `y = -+ry d`, `z = -d` for the light's range `d`
    /// (`+0xb4`) and then the far distance; `rx`, `ry` the reciprocals of
    /// the light's `+0x30` and `+0x44` (0 for 0). Each through C's
    /// transform into `x, y, 0` at `ebp - 0x104`.
    fn frustum(h: &mut impl Host, ebp: u32, light: u32) {
        let range = x87::store(f32_at(h, light + 0xb4));
        h.write(ebp - 0x12c, range);
        for k in 0..4 {
            let w: u32 = h.read(light + 0x30 + 4 * k);
            h.write(ebp - 0x118 + 4 * k, w);
        }
        let reciprocal = |v: f64| {
            #[allow(clippy::float_cmp)] // fucom against 0: only equality
            if v == 0.0 { 0.0 } else { x87::div(1.0, v) }
        };
        let sx = x87::store(reciprocal(f32_at(h, light + 0x30)));
        for k in 0..4 {
            let w: u32 = h.read(light + 0x40 + 4 * k);
            h.write(ebp - 0x118 + 4 * k, w);
        }
        h.write(ebp - 0x120, sx);
        let sy = x87::store(reciprocal(f32_at(h, light + 0x44)));
        h.write(ebp - 0x11c, sy);
        let far = x87::store(f32_at(h, h.rebase(FAR_DISTANCE)));
        h.write(ebp - 0x128, far);
        let (sx, sy) = (x87::load(sx), x87::load(sy));
        let mut corners = [[0u32; 3]; 8];
        for (k, d) in [range, far].into_iter().enumerate() {
            let d = x87::load(d);
            let x0 = x87::store(x87::mul(-sx, d));
            let y0 = x87::store(x87::mul(-sy, d));
            let z = x87::store(-d);
            let x1 = x87::store(x87::mul(sx, d));
            let y1 = x87::store(x87::mul(sy, d));
            let c = &mut corners[4 * k..4 * k + 4];
            c[0] = [x0, y0, z];
            c[1] = [x1, y0, z];
            c[2] = [x1, y1, z];
            c[3] = [x0, y1, z];
        }
        for (k, c) in (0..).zip(corners) {
            for (w, v) in (0..).zip(c) {
                h.write(ebp - CORNERS + 0x10 * k + 4 * w, v);
            }
        }
        // the scratch words as the original leaves them
        let last = corners[7];
        h.write(ebp - 0x118, last[0]);
        h.write(ebp - 0x114, last[1]);
        h.write(ebp - 0x110, last[2]);
        h.write(ebp - 0x108, x87::store(x87::mul(x87::load(far), sy)));
        for k in 0..8 {
            let p = call(
                h,
                TRANSFORM3,
                ebp - MC,
                &[ebp - 0x118, ebp - CORNERS + 0x10 * k],
            );
            let at = ebp - PROJECTED + 0x10 * k;
            copy(h, at, p, 2);
            h.write(at + 8, 0u32);
        }
        // the longest edge (squared) of the projected frustum
        h.write(ebp - 0x11c, 0u32);
        let edges = h.rebase(EDGES);
        for e in 0..12 {
            let b: u32 = h.read(edges + 8 * e);
            let a: u32 = h.read(edges + 8 * e + 4);
            let (pa, pb) = (
                (ebp - PROJECTED).wrapping_add(a.wrapping_mul(16)),
                (ebp - PROJECTED).wrapping_add(b.wrapping_mul(16)),
            );
            let mut d = [0u32; 3];
            for (k, v) in d.iter_mut().enumerate() {
                let k = 4 * k as u32;
                *v = put(
                    h,
                    ebp - 0x118 + k,
                    x87::sub(f32_at(h, pa + k), f32_at(h, pb + k)),
                );
            }
            let [dx, dy, dz] = d.map(x87::load);
            let s = put(h, ebp - 0x108, x87::mul(dx, dx));
            let s = put(h, ebp - 0x108, x87::add(x87::load(s), x87::mul(dy, dy)));
            let t = put(h, ebp - 0x108, x87::add(x87::load(s), x87::mul(dz, dz)));
            let longest = f32_at(h, ebp - 0x11c);
            if longest < x87::load(t) {
                h.write(ebp - 0x11c, t);
            }
        }
    }

    /// The projected corners' bounds through D (the light's view times
    /// the identity A): minimum (`ebp - 0x1dc`) and maximum (`- 0x1cc`),
    /// left in the frame.
    fn bounds(h: &mut impl Host, ebp: u32, mc: u32, md: u32, ma: u32) {
        identity(h, ma, 0);
        call(h, MUL_THIS, ma, &[md, mc]);
        let max = x87::store(f32_at(h, h.rebase(C_MAX)));
        let min = x87::store(f32_at(h, h.rebase(C_MIN)));
        for at in [0x1dc, 0x1d8, 0x1d4] {
            h.write(ebp - at, max);
        }
        for at in [0x1cc, 0x1c8, 0x1c4] {
            h.write(ebp - at, min);
        }
        for k in 0..8 {
            call(h, TRANSFORM3, md, &[ebp - 0x118, ebp - CORNERS + 0x10 * k]);
            let p = [0x118, 0x114, 0x110].map(|at| f32_at(h, ebp - at));
            // fcom + test 0x41: the new value unless it is greater
            let low = |v: f64, m: f64| if v > m { m } else { v };
            let high = |v: f64, m: f64| if m > v { m } else { v };
            let lo = [0x1dc, 0x1d8, 0x1d4].map(|at| f32_at(h, ebp - at));
            let hi = [0x1cc, 0x1c8, 0x1c4].map(|at| f32_at(h, ebp - at));
            let lo = [low(p[0], lo[0]), low(p[1], lo[1]), low(p[2], lo[2])].map(x87::store);
            let hi = [high(p[0], hi[0]), high(p[1], hi[1]), high(p[2], hi[2])].map(x87::store);
            for (at, v) in [0x1dc, 0x1d8, 0x1d4].into_iter().zip(lo) {
                h.write(ebp - at, v);
            }
            for (at, v) in [0x1cc, 0x1c8, 0x1c4].into_iter().zip(hi) {
                h.write(ebp - at, v);
            }
            // the scratch words: x's high, z's high, y's high
            h.write(ebp - 0x124, hi[0]);
            h.write(ebp - 0x120, hi[2]);
            h.write(ebp - 0x11c, hi[1]);
        }
    }

    /// The texture-space bias of `+0x190` (`m`): each row `r` of the
    /// result is `(m[r] * h + m[r+1] * 0 + ...)` in the original's x87
    /// order (the zero products kept: they carry NaNs and infinities),
    /// into B's rows.
    #[allow(clippy::many_single_char_names)] // the matrix entries
    fn bias(h: &mut impl Host, ebp: u32, this: u32, half: f64) {
        let q = f32_at(h, ebp - 0x174);
        let (add, mul) = (x87::add, x87::mul);
        let e: Vec<f64> = (0..16).map(|i| f32_at(h, this + 0x190 + 4 * i)).collect();
        // rows 3, 2, 1: products by 0 and by the half
        let row = |a: f64, b: f64, c: f64, d: f64| -> [f64; 4] {
            let (za, zb, zc) = (mul(a, 0.0), mul(b, 0.0), mul(c, 0.0));
            let (ha, hb, hc, hd) = (mul(a, half), mul(b, half), mul(c, half), mul(d, half));
            [
                add(add(add(ha, zb), zc), hd),
                add(hd, add(add(hb, za), zc)),
                add(add(hc, add(zb, za)), mul(q, d)),
                add(add(zc, add(zb, za)), d),
            ]
        };
        let r3 = row(e[12], e[13], e[14], e[15]);
        let r2 = row(e[8], e[9], e[10], e[11]);
        let r1 = row(e[4], e[5], e[6], e[7]);
        let r0 = row(e[0], e[1], e[2], e[3]);
        let slots = [
            (r3, [0x118, 0x114, 0x110, 0x10c]),
            (r2, [0x1dc, 0x1d8, 0x1d4, 0x1d0]),
            (r1, [0x1cc, 0x1c8, 0x1c4, 0x1c0]),
            (r0, [0x17c, 0x178, 0x174, 0x170]),
        ];
        let mut out = [[0u32; 4]; 4];
        for (k, (r, at)) in slots.into_iter().enumerate() {
            for w in 0..4 {
                out[k][w] = put(h, ebp - at[w], r[w]);
            }
        }
        // B: rows 0, 1, 2, 3 = r0, r1, r2, r3
        for (k, r) in [out[3], out[2], out[1], out[0]].into_iter().enumerate() {
            for (w, v) in r.into_iter().enumerate() {
                h.write(ebp - MB + 0x10 * k as u32 + 4 * w as u32, v);
            }
        }
    }
}
