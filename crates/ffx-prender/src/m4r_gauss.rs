//! The legacy glow's Gaussian tables (`0x00a0ff50`, list p6-render-c;
//! `m4-rest`), which `PLegacyGlow_setup` (`0x00a176d0`) fills once with
//! `(width, height, 7, 1.0)`.
//!
//! `thiscall(glow; width, height, taps, spread)`, `ret 0x10` -> 0, or 5
//! when `taps` is not odd in `3..=7`. Everything it writes is in the glow
//! object (`n` taps, `k` a tap):
//!
//! | offset | what |
//! |---|---|
//! | `+0x20` | the weights: 4 in the middle, times 0.6 per step out (`f32` each step) |
//! | `+0x3c` | the non-zero tap offsets, `(k - n/2) * spread` |
//! | `+0x58` | all tap offsets |
//! | `+0x74` | `n x n` vectors `(offset[col] / (w/2), offset[row] / (h/2), 0, 0)` |
//! | `+0x384` | `n x n` distances, `min(1, |(x, y)| / sqrt(1.8))` over the cell centres in `-1..1` |
//! | `+0x448` | `n x n` products `weight[row] * weight[col]`, divided by their sum |
//! | `+0x50c` | weights 0..3 divided by the weights' sum (word 3 is stale with 3 taps) |
//! | `+0x51c` | `n` horizontal vectors `(offset / (w/2), 0, 0, 0)` |
//! | `+0x58c` | `n` vertical vectors `(0, offset / (h/2), 0, 0)` |
//! | `+0x5fc` | the first `n x n - 1` vectors' `(x, y)` packed four to 32 bytes |
//! | `+0x90c` | the first `n x n - 1` products |
//! | `+0xc1c` | the first `n x n - 1` distances |
//!
//! Floats as the original keeps them: `f64` on the x87 stack at 53 bits,
//! rounded to `f32` at each of its stores (`ffx_bridge::x87`). The square
//! roots are the C runtime's `_CIsqrt` (`x87::sqrt`).

use ffx_bridge::{Mem, x87};

/// `0xb922f8`: the middle weight.
const MIDDLE: f32 = 4.0;
/// `0xb48320`: each weight out from the middle is the last one times this
/// (`f64` of the `f32` 0.6).
const FALLOFF: f32 = 0.6;
/// `0xb51560`: the distances are divided by its square root (`f64` of
/// the `f32` 1.8).
const RADIUS_SQUARED: f32 = 1.8;

/// The result for a tap count outside `3..=7` or even.
pub const BAD_TAPS: u32 = 5;

/// `f32` bits as `fld dword` loads them.
fn ld(bits: u32) -> f64 {
    x87::load(bits)
}

/// `(f32)(x >> 1)`: `shr`, `fild` of the (non-negative) word, `fstp
/// dword`.
fn half_size(x: u32) -> f64 {
    ld(x87::store(f64::from(x >> 1)))
}

/// A cell centre in `-1..1`: `(i + 0.5) / n * 2 - 1`, `i` through an
/// `f32` (`fild`, `fstp dword`).
fn centre(i: usize, n: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)] // i < 7
    let i = i as f64;
    x87::sub(x87::mul(x87::div(x87::add(i, 0.5), n), 2.0), 1.0)
}

pub mod bridge {
    //! The original over game memory.

    use super::{BAD_TAPS, FALLOFF, MIDDLE, Mem, RADIUS_SQUARED, centre, half_size, ld, x87};

    /// The glow object and its tap count.
    struct Glow {
        at: u32,
        n: usize,
    }

    impl Glow {
        /// Word `k` of the table at `+base`.
        fn word(&self, base: u32, k: usize) -> u32 {
            #[allow(clippy::cast_possible_truncation)] // k < 0x100
            let k = k as u32;
            self.at + base + 4 * k
        }
        fn weight(&self, k: usize) -> u32 {
            self.word(0x20, k)
        }
        fn offset(&self, k: usize) -> u32 {
            self.word(0x58, k)
        }
    }

    /// `0x00a0ff50(glow; width, height, taps, spread)`: `spread` as its
    /// `f32` bits.
    pub fn gaussian_tables(
        m: &mut impl Mem,
        glow: u32,
        width: u32,
        height: u32,
        taps: u32,
        spread: u32,
    ) -> u32 {
        // odd, and signed 3..=7
        #[allow(clippy::cast_possible_wrap)]
        let signed = taps as i32;
        if taps & 1 == 0 || !(3..=7).contains(&signed) {
            return BAD_TAPS;
        }
        let g = Glow {
            at: glow,
            n: taps as usize,
        };
        weights(m, &g, spread);
        cells(m, &g, width, height);
        products(m, &g);
        separable(m, &g, width, height);
        pack(m, &g);
        0
    }

    /// The tap offsets and the weights.
    fn weights(m: &mut impl Mem, g: &Glow, spread: u32) {
        let n = g.n;
        let half = n / 2;
        // the tap offsets: (k - n/2) * spread, the non-zero ones also at
        // +0x3c
        m.write::<u32>(g.weight(half), MIDDLE.to_bits());
        let spread = ld(spread);
        let mut nonzero = g.at + 0x3c;
        #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
        for k in 0..n {
            let i = k as i32 - half as i32;
            let o = x87::store(x87::mul(f64::from(i), spread));
            m.write(g.offset(k), o);
            if i != 0 {
                m.write(nonzero, o);
                nonzero += 4;
            }
        }

        // the weights out from the middle, and their sum (an f32)
        let mut w = m.read::<u32>(g.weight(half));
        let mut sum = w;
        for d in 1..=half {
            w = x87::store(x87::mul(ld(w), f64::from(FALLOFF)));
            m.write(g.weight(half - d), w);
            m.write(g.weight(half + d), w);
            sum = x87::store(
                x87::Reg::constant(x87::mul(ld(w), 2.0))
                    .add_m32(f32::from_bits(sum))
                    .value(),
            );
        }
        let s = ld(sum);
        for k in 0..3 {
            let q = x87::store(x87::div(ld(m.read::<u32>(g.weight(k))), s));
            m.write(g.word(0x50c, k), q);
        }
        // fdivr dword: the fourth word is a memory operand
        let q = x87::Reg::constant(s).divr_m32(f32::from_bits(m.read::<u32>(g.weight(3))));
        m.write(g.at + 0x518, x87::store(q.value()));
    }

    /// The vectors and distances of the n x n cells.
    fn cells(m: &mut impl Mem, g: &Glow, width: u32, height: u32) {
        let n = g.n;
        let hw = half_size(width);
        let hh = half_size(height);
        #[allow(clippy::cast_precision_loss)]
        let nf = n as f64;
        let root = ld(x87::store(x87::sqrt(f64::from(RADIUS_SQUARED))));
        let inv_root = x87::div(1.0, root);
        for row in 0..n {
            let y = ld(x87::store(centre(row, nf)));
            let yy = x87::mul(y, y);
            for col in 0..n {
                let cell = row * n + col;
                let u = x87::store(x87::div(ld(m.read::<u32>(g.offset(col))), hw));
                let v = x87::Reg::constant(ld(m.read::<u32>(g.offset(row))))
                    .div_m32(f32::from_bits(x87::store(hh)));
                let vec = g.word(0x74, 4 * cell);
                m.write(vec, u);
                m.write(vec + 4, x87::store(v.value()));
                m.write(vec + 8, 0_u32);
                m.write(vec + 12, 0_u32);
                let x = ld(x87::store(centre(col, nf)));
                let r2 = ld(x87::store(x87::add(yy, x87::mul(x, x))));
                let r = ld(x87::store(x87::sqrt(r2)));
                let d = x87::store(x87::mul(r, inv_root));
                // fcom: 1.0 only when 1 < d (a NaN stays)
                let d = if 1.0 < f32::from_bits(d) {
                    1.0_f32.to_bits()
                } else {
                    d
                };
                m.write(g.word(0x384, cell), d);
            }
        }
    }

    /// The products weight[row] * weight[col], divided by their sum (an
    /// f32).
    fn products(m: &mut impl Mem, g: &Glow) {
        let n = g.n;
        let mut total = 0_u32;
        for row in 0..n {
            let wa = ld(x87::store(ld(m.read::<u32>(g.weight(row)))));
            for col in 0..n {
                let p = x87::store(x87::mul(ld(m.read::<u32>(g.weight(col))), wa));
                m.write(g.word(0x448, row * n + col), p);
                total = x87::store(
                    x87::Reg::constant(ld(p))
                        .add_m32(f32::from_bits(total))
                        .value(),
                );
            }
        }
        let inv = ld(x87::store(x87::div(1.0, ld(total))));
        for cell in 0..n * n {
            // the unrolled groups multiply m * inv, the rest inv * m: the
            // same, as no product is NaN (the weights are constants)
            let a = g.word(0x448, cell);
            let p = x87::store(x87::mul(ld(m.read::<u32>(a)), inv));
            m.write(a, p);
        }
    }

    /// The separable vectors: `(offset / (w/2), 0, 0, 0)` and `(0, offset
    /// / (h/2), 0, 0)`.
    fn separable(m: &mut impl Mem, g: &Glow, width: u32, height: u32) {
        let hw = half_size(width);
        let hh = half_size(height);
        for k in 0..g.n {
            let o = ld(m.read::<u32>(g.offset(k)));
            let u = x87::store(x87::div(o, hw));
            let v = x87::store(x87::div(o, hh));
            let h = g.word(0x51c, 4 * k);
            m.write(h, u);
            m.write(h + 4, 0_u32);
            m.write(h + 8, 0_u32);
            m.write(h + 12, 0_u32);
            let vv = g.word(0x58c, 4 * k);
            m.write(vv, 0_u32);
            m.write(vv + 4, v);
            m.write(vv + 8, 0_u32);
            m.write(vv + 12, 0_u32);
        }
    }

    /// The first n x n - 1 cells packed for the shader, four at a time
    /// (each word copied through the x87 stack).
    fn pack(m: &mut impl Mem, g: &Glow) {
        for group in 0..(g.n * g.n - 1) / 4 {
            for j in 0..4 {
                let cell = 4 * group + j;
                let vec = g.word(0x74, 4 * cell);
                let to = g.word(0x5fc, 8 * group + 2 * j);
                for (from, to) in [
                    (vec, to),
                    (vec + 4, to + 4),
                    (g.word(0x384, cell), g.word(0xc1c, cell)),
                    (g.word(0x448, cell), g.word(0x90c, cell)),
                ] {
                    let w = x87::store(ld(m.read::<u32>(from)));
                    m.write(to, w);
                }
            }
        }
    }
}
