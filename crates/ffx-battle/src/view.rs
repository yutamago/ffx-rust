//! The battle camera's view matrices (`docs/rewrite-plan.md`, M2):
//! `MsCameraMatrix`, the look-at matrix with a roll, and `MsScreenMatrix`,
//! the projection. Like the vector
//! helpers ([`crate::vector`]) it is PS2 VU0 macro code the PC port runs
//! on the x87 through the emulated VF registers (`0x00c8f788` Q, the
//! accumulator at `0x00c8f790`, 16-byte registers from `0x00c8f7d0`);
//! the values it leaves there are outputs too.
//!
//! Arithmetic as in [`crate::vector`]: `f64` at the x87's 53 bits, rounded
//! to `f32` at each store; a product feeding an add without a store
//! between is exact (two `f32`s), only the sum is rounded.

use ffx_bridge::x87::{add, div, load, mul, store, sub};

use crate::camera::x87_copy;
use crate::vector::Vec4;

/// The registers `MsCameraMatrix` leaves, from `0x00c8f7d0` on (nine
/// 16-byte registers), with the accumulator and Q.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraRegs {
    /// `0x00c8f788`: the last normalisation's 1 / length.
    pub q: u32,
    /// `0x00c8f790`.
    pub acc: Vec4,
    /// `0x00c8f7d0`..`0x00c8f85c`.
    pub vf: [Vec4; 9],
}

/// The inputs `MsCameraMatrix` takes besides its arguments: the registers
/// whose old `w` it reads, `vf00` as `restoreVf00Register` leaves it, the
/// roll's cosine and sine (`f32` bits from the CRT), and the four
/// uninitialised stack words it copies into the roll rows' `z` and `w`.
#[derive(Debug, Clone, Copy)]
pub struct CameraIn {
    /// The eye, the point looked at and the up vector (16 bytes each).
    pub eye: Vec4,
    pub target: Vec4,
    pub up: Vec4,
    /// `0x00c8f7fc` and `0x00c8f83c` before the call (`w - w` keeps a NaN
    /// or turns an infinity into one).
    pub old_7fc: u32,
    pub old_83c: u32,
    /// `vf00` (`0x00c0a004`).
    pub vf00: Vec4,
    pub cos: u32,
    pub sin: u32,
    /// The original's `[ebp-0x18]`, `[ebp-0x14]`, `[ebp-0x28]`,
    /// `[ebp-0x24]`: never written in its frame.
    pub stale: [u32; 4],
}

/// One float's bits through `fld`.
fn ld(bits: u32) -> f64 {
    load(bits)
}

/// The VU's outer product as the port runs it (`vopmula` then `vopmsub`):
/// the accumulator `a.yzx * b.zxy` (stored), then `acc - a.zxy * b.yzx`.
/// Returns the accumulator and the result's `xyz`.
fn cross(a: Vec4, b: Vec4) -> ([u32; 3], [u32; 3]) {
    let acc = [
        store(mul(ld(a[1]), ld(b[2]))),
        store(mul(ld(a[2]), ld(b[0]))),
        store(mul(ld(a[0]), ld(b[1]))),
    ];
    let r = [
        store(sub(ld(acc[0]), mul(ld(a[2]), ld(b[1])))),
        store(sub(ld(acc[1]), mul(ld(a[0]), ld(b[2])))),
        store(sub(ld(acc[2]), mul(ld(a[1]), ld(b[0])))),
    ];
    (acc, r)
}

/// `v.xyz` times `1 / |v.xyz|` as the port computes it: the squares
/// stored, summed `y + x` then `z + that`, `|sum|` stored; Q is 0 for a
/// zero sum, else `one / sqrt` (the root stored as `f32`, the quotient
/// too); each lane times Q stored. Returns the scaled `xyz` and Q.
fn normalise(v: [u32; 3], one: u32) -> ([u32; 3], u32) {
    let sq = |x: u32| store(mul(ld(x), ld(x)));
    let (xx, yy, zz) = (sq(v[0]), sq(v[1]), sq(v[2]));
    let sum = store(add(ld(zz), ld(store(add(ld(yy), ld(xx))))));
    let t = ld(store(ld(sum).abs()));
    let q = if t == 0.0 {
        0.0f32.to_bits()
    } else {
        let root = store(ffx_bridge::x87::sqrt(t));
        store(div(ld(one), ld(root)))
    };
    (v.map(|x| store(mul(ld(x), ld(q)))), q)
}

/// One row of the result: `v` turned by the roll in its `xy` plane
/// (`x' = v.y·(-sin) + cos·v.x`, `y' = cos·v.y + sin·v.x`, the products
/// `cos·v.x` and `sin·v.x` stored first in the accumulator), `z`, `w`
/// copied. Returns the row and the accumulator's `xy`.
fn roll_row(v: Vec4, cos: u32, sin: u32) -> (Vec4, [u32; 2]) {
    let neg_sin = sin ^ 0x8000_0000;
    let acc = [store(mul(ld(cos), ld(v[0]))), store(mul(ld(sin), ld(v[0])))];
    let row = [
        store(add(mul(ld(v[1]), ld(neg_sin)), ld(acc[0]))),
        store(add(mul(ld(cos), ld(v[1])), ld(acc[1]))),
        x87_copy(v[2]),
        x87_copy(v[3]),
    ];
    (row, acc)
}

/// `MsCameraMatrix` past its equal-points test: the view matrix looking
/// from `eye` at `target` with `up` and the roll, and the registers it
/// leaves.
///
/// The basis: `d` = normalised `target - eye`, `c` = normalised `d × up`,
/// `e` = `d × c` normalised; the rows' `w` cleared by `w - w`; then the
/// three transposed (`vaddx`-style moves adding a `vf00` lane) into the
/// columns `col0 = (c.x, e.x, d.x)`, `col1 = (c.y, e.y, d.y)`,
/// `col2 = (c.z, e.z, d.z)`; `r` = `eye.x·col0 + eye.y·col1 + eye.z·col2`
/// and the translation `vf00 - r`, `w` 1; each of the four rows turned by
/// the roll ([`roll_row`]).
#[must_use]
#[allow(clippy::similar_names, clippy::many_single_char_names)]
pub fn camera_matrix(i: &CameraIn) -> ([Vec4; 4], CameraRegs) {
    let [x0, y0, z0, one] = i.vf00;
    let sub3 = |a: Vec4, b: Vec4| [0, 1, 2].map(|k| store(sub(ld(a[k]), ld(b[k]))));

    // the forward axis
    let (d, _) = normalise(sub3(i.target, i.eye), one);
    // the side axis
    let (_, side) = cross([d[0], d[1], d[2], 0], i.up);
    let (c, _) = normalise(side, one);
    // the up axis
    let (_, upward) = cross([d[0], d[1], d[2], 0], [c[0], c[1], c[2], 0]);
    let (e, q) = normalise(upward, one);
    // `w - w`: c's w was set to 0 on entry
    let w_minus_w = |w: u32| store(sub(ld(w), ld(w)));
    let d_w = w_minus_w(i.old_7fc);
    let c_w = w_minus_w(0);
    let e_w = w_minus_w(i.old_83c);

    // transposed: the off-diagonal lanes moved by adding a vf00 lane
    let lane = |v: u32, z: u32| store(add(ld(v), ld(z)));
    let col0 = [
        x87_copy(c[0]),
        lane(e[0], y0),
        lane(d[0], z0),
        x87_copy(c_w),
    ];
    let col1 = [
        lane(c[1], x0),
        x87_copy(e[1]),
        lane(d[1], z0),
        x87_copy(e_w),
    ];
    let col2 = [lane(c[2], x0), lane(e[2], y0), d[2], d_w];

    // the eye through the rotation
    let [px, py, pz, _] = i.eye;
    let acc0 = col0.map(|v| store(mul(ld(px), ld(v))));
    let acc1: Vec4 = std::array::from_fn(|k| store(add(mul(ld(py), ld(col1[k])), ld(acc0[k]))));
    let r: Vec4 = std::array::from_fn(|k| store(add(mul(ld(pz), ld(col2[k])), ld(acc1[k]))));
    let t = [
        store(sub(ld(x0), ld(r[0]))),
        store(sub(ld(y0), ld(r[1]))),
        store(sub(ld(z0), ld(r[2]))),
        x87_copy(one),
    ];

    // the roll
    let (row0, _) = roll_row(col0, i.cos, i.sin);
    let (row1, _) = roll_row(col1, i.cos, i.sin);
    let (row2, _) = roll_row(col2, i.cos, i.sin);
    let (row3, acc) = roll_row(t, i.cos, i.sin);

    let [s0, s1, s2, s3] = i.stale;
    let neg_sin = x87_copy(i.sin) ^ 0x8000_0000;
    let regs = CameraRegs {
        q,
        acc: [acc[0], acc[1], acc1[2], acc1[3]],
        vf: [
            [i.cos, i.sin, s0, s1],
            [neg_sin, i.cos, s2, s3],
            col2,
            [
                x87_copy(c[0]),
                x87_copy(c[1]),
                x87_copy(c[2]),
                x87_copy(c_w),
            ],
            row0,
            row1,
            row2,
            row3,
            t,
        ],
    };
    ([row0, row1, row2, row3], regs)
}

/// The six vectors `MsScreenMatrix` reads from a screen (`0x1c0` bytes
/// each from `0x01137000`): `+0x10` the scale (`MsSetScreen`'s b, c, the
/// depth factor, a), `+0x20` the centre (d, e, the depth offset), `+0x30`,
/// `+0x40`, `+0x50`, `+0x70`, as `f32` bits.
#[derive(Debug, Clone, Copy)]
pub struct Screen {
    pub s10: Vec4,
    pub s20: Vec4,
    pub s30: Vec4,
    pub s40: Vec4,
    pub s50: Vec4,
    pub s70: Vec4,
}

/// What `MsScreenMatrix` computes: the matrix, the nine registers it
/// leaves from `0x00c8f7a0`, Q, and the VU flags it raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenOut {
    pub matrix: [Vec4; 4],
    pub vf: [Vec4; 9],
    pub q: u32,
    /// `setZSFlag` (a centre coordinate or the `y` scale 0) and
    /// `setSSFlag` (one of them negative).
    pub zero: bool,
    pub sign: bool,
}

/// `MsScreenMatrix` on a screen: the projection's diagonal and centre.
///
/// The centre `(+0x30 + +0x20) + +0x70`, less `+0x50` (`xy`, through
/// `Vec2SubScalarsFlags_0075e870`, which raises the flags on it); the
/// scales `((s10.w · s10.x) · s40.z) / s50.z + vf00.x` and
/// `((s10.y · s10.w) · s40.w) / s50.w` (Q) `+ vf00.y`, each step stored.
/// Rows: `(sx, 0, 0, 0)`, `(0, sy, 0, 0)`, `(cx, cy, s20.z, 1)`,
/// `(0, 0, s10.z, 0)`.
#[must_use]
pub fn screen_matrix(s: &Screen, vf00: Vec4) -> ScreenOut {
    let zero = 0.0f32.to_bits();
    let one = 1.0f32.to_bits();
    let centre = |k: usize| {
        let c = store(add(ld(s.s30[k]), ld(s.s20[k])));
        let c = store(add(ld(s.s70[k]), ld(c)));
        store(sub(ld(c), ld(s.s50[k])))
    };
    let (cx, cy) = (centre(0), centre(1));
    let (fx, fy) = (ld(cx), ld(cy));
    let w = ld(s.s10[3]);
    let sx = store(mul(w, ld(s.s10[0])));
    let sx = store(mul(ld(s.s40[2]), ld(sx)));
    let sx = store(div(ld(sx), ld(s.s50[2])));
    let sx = store(add(ld(sx), ld(vf00[0])));
    let sy = store(mul(ld(s.s10[1]), w));
    let sy = store(mul(ld(s.s40[3]), ld(sy)));
    let q = store(div(ld(sy), ld(s.s50[3])));
    let sy = store(add(ld(vf00[1]), ld(q)));
    let fsy = ld(sy);
    let row0 = [sx, zero, zero, zero];
    let row1 = [zero, sy, zero, zero];
    let row2 = [cx, cy, s.s20[2], one];
    let row3 = [zero, zero, s.s10[2], zero];
    ScreenOut {
        matrix: [row0, row1, row2, row3],
        vf: [
            [zero; 4], row0, row1, row3, row2, s.s30, s.s50, s.s40, s.s70,
        ],
        q,
        zero: fx == 0.0 || fy == 0.0 || fsy == 0.0,
        sign: fx < 0.0 || fy < 0.0 || fsy < 0.0,
    }
}

/// The view matrices where the game keeps them, through
/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{CameraIn, Screen, Vec4, camera_matrix, screen_matrix};
    use crate::pos::bridge::Trig;

    /// `g_Vu0_Q` (`0x00c8f788`).
    pub const VU_Q: Global<u32> = Global::new(0x00c8_f788);
    /// The accumulator (`0x00c8f790`).
    pub const VU_ACC: Global<Vec4> = Global::new(0x00c8_f790);
    /// The nine registers `MsCameraMatrix` uses, from `0x00c8f7d0`.
    pub const VU_CAMERA: Global<[Vec4; 9]> = Global::new(0x00c8_f7d0);
    /// `vf00` (`g_Vu0_Vf00`).
    pub const VF00: Global<Vec4> = Global::new(0x00c0_a004);

    /// `MsCameraMatrix(out, eye, target, up, roll)` (`0x007c1be0`): the
    /// 4×4 view matrix into `out` (see [`super::camera_matrix`]). When
    /// `eye` and `target` hold the same 16 bytes, `out` is zero but for
    /// `out[15]` = 1 and no register changes. `trig` gives `_CIcos` and
    /// `_CIsin` of the roll, as `f32` bits. The original copies four
    /// stack words it never wrote into `0x00c8f7d8`/`dc`/`e8`/`ec`; this
    /// writes 0 there.
    ///
    /// Returns what the original leaves in `eax`: `out[15]`'s bits (1.0),
    /// or `eye.w`'s for equal points.
    pub fn ms_camera_matrix(
        mem: &mut impl Mem,
        trig: &dyn Fn(Trig, u32) -> u32,
        out: u32,
        eye: u32,
        target: u32,
        up: u32,
        roll: u32,
    ) -> u32 {
        let e: Vec4 = mem.read(eye);
        let t: Vec4 = mem.read(target);
        if e == t {
            let one = 1.0f32.to_bits();
            let mut m = [0u32; 16];
            m[15] = one;
            mem.write(out, m);
            return e[3];
        }
        let cos = trig(Trig::Cos, roll);
        let sin = trig(Trig::Sin, roll);
        let up: Vec4 = mem.read(up);
        let regs = VU_CAMERA.get(mem);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let input = CameraIn {
            eye: e,
            target: t,
            up,
            old_7fc: regs[2][3],
            old_83c: regs[6][3],
            vf00: VF00.get(mem),
            cos,
            sin,
            stale: [0; 4],
        };
        let (m, r) = camera_matrix(&input);
        VU_Q.set(mem, r.q);
        VU_ACC.set(mem, r.acc);
        VU_CAMERA.set(mem, r.vf);
        mem.write(out, m);
        m[3][3]
    }

    /// The nine registers `MsScreenMatrix` leaves, from `0x00c8f7a0`.
    pub const VU_SCREEN: Global<[Vec4; 9]> = Global::new(0x00c8_f7a0);

    /// `MsScreenMatrix(out, screen)` (`0x007c3d60`): the screen's
    /// projection into `out` ([`super::screen_matrix`]), with the
    /// registers, Q and the VU flags it leaves (`vf00` restored by the
    /// flag test on the centre, `Vec2SubScalarsFlags_0075e870`). Returns 0
    /// (`eax`, the last register word it copies).
    pub fn ms_screen_matrix(mem: &mut impl Mem, out: u32, screen: u32) -> u32 {
        let v = |mem: &mut _, o: u32| -> Vec4 { Mem::read(mem, screen.wrapping_add(o)) };
        let s = Screen {
            s10: v(mem, 0x10),
            s20: v(mem, 0x20),
            s30: v(mem, 0x30),
            s40: v(mem, 0x40),
            s50: v(mem, 0x50),
            s70: v(mem, 0x70),
        };
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let r = screen_matrix(&s, VF00.get(mem));
        if r.zero {
            ffx_ps2::ee::bridge::set_zs_flag(mem);
        }
        if r.sign {
            ffx_ps2::ee::bridge::set_ss_flag(mem);
        }
        VU_Q.set(mem, r.q);
        VU_SCREEN.set(mem, r.vf);
        mem.write(out, r.matrix);
        0
    }
}
