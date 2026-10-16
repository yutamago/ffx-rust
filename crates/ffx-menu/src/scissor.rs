//! The menu's software scissor: `BeginScissor_XYUV` clips a textured
//! rectangle (x0, y0)–(x1, y1) with texture coordinates (u0, v0)–(u1, v1)
//! to the scissor box, moving the texture coordinates along; `CloseScissor`
//! turns it off.
//!
//! The original runs on the x87 at 53-bit precision: every intermediate is
//! a double, rounded to a float only where it stores one (the clip ratio
//! and the results). The core does the same in `f64`, which agrees exactly
//! for float inputs (no intermediate leaves the double range).

/// The scissor's state (`g_TkMenuScissor*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scissor {
    /// On when 1 (`u32`); otherwise nothing is clipped.
    pub on: u32,
    /// "XYUV" mode when 1 (`u8`): x1 is first scaled by `scale_b /
    /// scale_a` from x0, clipped, and scaled back.
    pub xyuv: u8,
    /// `0x018716a8`.
    pub scale_a: f32,
    /// `0x018716ac`.
    pub scale_b: f32,
    /// The box.
    pub left: f32,
    /// See [`Scissor::left`].
    pub top: f32,
    /// See [`Scissor::left`].
    pub right: f32,
    /// See [`Scissor::left`].
    pub bottom: f32,
}

/// The eight floats `BeginScissor_XYUV` gets pointers to, in argument order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    X0,
    Y0,
    X1,
    Y1,
    U0,
    V0,
    U1,
    V1,
}

/// Where the corners live: read and written one at a time, in the
/// original's order (its arguments are pointers, which may alias).
pub trait Corners {
    /// The value now.
    fn get(&self, c: Corner) -> f32;
    /// Stores a value.
    fn set(&mut self, c: Corner, v: f32);
}

impl Corners for [f32; 8] {
    fn get(&self, c: Corner) -> f32 {
        self[c as usize]
    }
    fn set(&mut self, c: Corner, v: f32) {
        self[c as usize] = v;
    }
}

fn d(v: f32) -> f64 {
    f64::from(v)
}

#[allow(clippy::cast_possible_truncation)] // the original's fstp to a float
fn f(v: f64) -> f32 {
    v as f32
}

/// One axis: the low edge moves up to `lo` (and its texture coordinate
/// along), then the high edge down to `hi`. `(a0, a1)` are the positions,
/// `(t0, t1)` the texture coordinates.
fn clip_axis(q: &mut impl Corners, [a0, a1, t0, t1]: [Corner; 4], lo: f32, hi: f32) {
    if q.get(a0) < lo && q.get(a1) > lo {
        let num = d(lo) - d(q.get(a0));
        let den = d(q.get(a1)) - d(q.get(a0));
        let ratio = f(num / den);
        let span = d(q.get(t1)) - d(q.get(t0));
        let v = span * d(ratio) + d(q.get(t0));
        q.set(t0, f(v));
        q.set(a0, lo);
    }
    if q.get(a0) < hi && q.get(a1) > hi {
        let end = q.get(t1);
        let num = d(q.get(a1)) - d(hi);
        let den = d(q.get(a1)) - d(q.get(a0));
        let ratio = f(num / den);
        let span = d(end) - d(q.get(t0));
        q.set(t1, f(d(end) - span * d(ratio)));
        q.set(a1, hi);
    }
}

/// x1 scaled from x0 by `mul / div` (the XYUV mode's change of units).
fn scale_x1(q: &mut impl Corners, mul: f32, div: f32) {
    let w = d(q.get(Corner::X1)) - d(q.get(Corner::X0));
    let v = w * d(mul) / d(div) + d(q.get(Corner::X0));
    q.set(Corner::X1, f(v));
}

/// `BeginScissor_XYUV`: false when the rectangle is wholly outside the box
/// (on an edge counts as outside), after clipping what it had clipped so
/// far; true otherwise, clipped.
pub fn begin_scissor_xyuv(s: &Scissor, q: &mut impl Corners) -> bool {
    use Corner::{U0, U1, V0, V1, X0, X1, Y0, Y1};
    if s.on != 1 {
        return true;
    }
    let xyuv = s.xyuv == 1;
    if xyuv {
        scale_x1(q, s.scale_b, s.scale_a);
    }
    if s.left >= q.get(X1) || s.right <= q.get(X0) {
        return false;
    }
    clip_axis(q, [X0, X1, U0, U1], s.left, s.right);
    if xyuv {
        scale_x1(q, s.scale_a, s.scale_b);
    }
    if s.bottom <= q.get(Y0) || s.top >= q.get(Y1) {
        return false;
    }
    clip_axis(q, [Y0, Y1, V0, V1], s.top, s.bottom);
    true
}

/// What `BeginScissor_XY` puts in all four corners of a rectangle wholly
/// outside the box (`0x00b368b8`).
pub const OUTSIDE: f32 = -100_000.0;

/// `BeginScissor_XY`: clips (x0, y0)–(x1, y1) to the box. A rectangle
/// wholly outside (on an edge counts) first goes to [`OUTSIDE`] in all four
/// corners; then each edge inside the box is moved onto it. Always true.
pub fn begin_scissor_xy(s: &Scissor, q: &mut impl Corners) -> bool {
    use Corner::{X0, X1, Y0, Y1};
    if s.on != 1 {
        return true;
    }
    if s.left >= q.get(X1) || s.right <= q.get(X0) || s.bottom <= q.get(Y0) || s.top >= q.get(Y1) {
        for c in [X0, Y0, X1, Y1] {
            q.set(c, OUTSIDE);
        }
    }
    if q.get(X0) < s.left && q.get(X1) > s.left {
        q.set(X0, s.left);
    }
    if q.get(X0) < s.right && q.get(X1) > s.right {
        q.set(X1, s.right);
    }
    if q.get(Y0) < s.top && q.get(Y1) > s.top {
        q.set(Y0, s.top);
    }
    if q.get(Y0) < s.bottom && q.get(Y1) > s.bottom {
        q.set(Y1, s.bottom);
    }
    true
}

/// The originals, on the game's globals (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{Corner, Corners, Scissor, begin_scissor_xyuv};

    /// On when 1 (`CloseScissor` clears it).
    pub const SCISSOR_ON: Global<u32> = Global::new(0x0187_16d0);
    /// The XYUV mode (`u8`).
    pub const SCISSOR_XYUV: Global<u8> = Global::new(0x0187_16d4);
    /// The two scales.
    pub const SCISSOR_SCALES: Global<[f32; 2]> = Global::new(0x0187_16a8);
    /// The box: left, top, right, bottom.
    pub const SCISSOR_BOX: Global<[f32; 4]> = Global::new(0x0187_16f8);

    /// The scissor's state now.
    pub fn scissor(mem: &impl Mem) -> Scissor {
        let [scale_a, scale_b] = SCISSOR_SCALES.get(mem);
        let [left, top, right, bottom] = SCISSOR_BOX.get(mem);
        Scissor {
            on: SCISSOR_ON.get(mem),
            xyuv: SCISSOR_XYUV.get(mem),
            scale_a,
            scale_b,
            left,
            top,
            right,
            bottom,
        }
    }

    /// The corners behind the original's eight pointers.
    struct Pointers<'a, M> {
        mem: &'a mut M,
        at: [u32; 8],
    }

    impl<M: Mem> Corners for Pointers<'_, M> {
        fn get(&self, c: Corner) -> f32 {
            self.mem.read(self.at[c as usize])
        }
        fn set(&mut self, c: Corner, v: f32) {
            self.mem.write(self.at[c as usize], v);
        }
    }

    /// `BeginScissor_XYUV(&x0, &y0, &x1, &y1, &u0, &v0, &u1, &v1)`
    /// (`0x008e5a20`).
    pub fn begin_scissor_xyuv_at(mem: &mut impl Mem, at: [u32; 8]) -> bool {
        let s = scissor(mem);
        begin_scissor_xyuv(&s, &mut Pointers { mem, at })
    }

    /// `BeginScissor_XY(&x0, &y0, &x1, &y1)` (`0x008e58e0`).
    pub fn begin_scissor_xy_at(mem: &mut impl Mem, at: [u32; 4]) -> bool {
        let s = scissor(mem);
        let [x0, y0, x1, y1] = at;
        let at = [x0, y0, x1, y1, 0, 0, 0, 0];
        super::begin_scissor_xy(&s, &mut Pointers { mem, at })
    }

    /// `CloseScissor()` (`0x008e5cd0`).
    pub fn close_scissor(mem: &mut impl Mem) {
        SCISSOR_ON.set(mem, 0);
        SCISSOR_XYUV.set(mem, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_and_moves_the_texture() {
        let s = Scissor {
            on: 1,
            xyuv: 0,
            scale_a: 1.0,
            scale_b: 1.0,
            left: 10.0,
            top: 0.0,
            right: 20.0,
            bottom: 100.0,
        };
        // x 0..40, u 0..1: clipped to 10..20, u to 0.25..0.5
        let mut q = [0.0, 10.0, 40.0, 20.0, 0.0, 0.0, 1.0, 1.0];
        assert!(begin_scissor_xyuv(&s, &mut q));
        assert_eq!(q, [10.0, 10.0, 20.0, 20.0, 0.25, 0.0, 0.5, 1.0]);
        // wholly left of the box, touching it
        let mut q = [0.0, 10.0, 10.0, 20.0, 0.0, 0.0, 1.0, 1.0];
        assert!(!begin_scissor_xyuv(&s, &mut q));
        // off: untouched
        let mut q = [0.0; 8];
        assert!(begin_scissor_xyuv(&Scissor { on: 2, ..s }, &mut q));
    }
}
