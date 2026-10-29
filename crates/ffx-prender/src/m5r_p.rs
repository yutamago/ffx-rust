//! M5 stage 1, list `render-pcui` (round 5), batch 3: the radial line's
//! per-frame update.

use crate::m5r_c::quiet;

/// `.rdata` constants of the radial line update: the start threshold
/// 0.3f, the fade length 16.0f, the doubles 0.3, 8.0, 1/128 and 128.0.
pub const RL_START: u32 = 0x00b3_eb44;
pub const RL_FADE: u32 = 0x00b9_231c;
pub const RL_START_D: u32 = 0x00b4_8308;
pub const RL_EIGHT: u32 = 0x00b9_22a8;
pub const RL_SCALE: u32 = 0x00b1_70b8;
pub const RL_FULL: u32 = 0x00b4_c170;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::Host;

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn load(h: &impl Mem, at: u32) -> f64 {
        x87::load(word(h, at))
    }

    fn f32c(h: &impl Mem, sym: u32) -> f64 {
        x87::load(h.read::<u32>(h.rebase(sym)))
    }

    fn f64c(h: &impl Mem, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    /// `RadialLine_update_006f8f00(this; hold, color, a, b)` (`ret
    /// 0x10`): for each of `2 (+0x30 + +0x34)` lines (the count read
    /// again each round, `<< 2 >> 1` as the original wraps it): the
    /// line's state (`+0x64`, 0x20 bytes) takes `a` at `+8`, `b` at
    /// `+0xc`, the colour at `+0x10`; its time (`+0`) advances by `+4`
    /// unless `hold`. Its intensity `+0x1c`: past the start (0.3), the
    /// progress `(t - 0.3) / speed` at or above 16 ends the line (time and
    /// intensity 0), below it fades `(128 - 8 p) / 128`; before the start
    /// `t / speed` below 16 gives `8 p / 128`, else 1. Its vertex pair
    /// (`+0x54`, 0x20 bytes) gets the colour with alpha `min(int a, 1)`
    /// then `min(int b, 1)`; its uv pair (`+0x58 + 8`, 0x10 stride) moves
    /// from the source rectangle (`+0x5c`) by the time.
    pub fn radial_update(h: &mut impl Host, this: u32, hold: u8, color: u32, a: u32, b: u32) {
        let start = f32c(h, RL_START);
        let fade = f32c(h, RL_FADE);
        let start_d = f64c(h, RL_START_D);
        let eight = f64c(h, RL_EIGHT);
        let scale = f64c(h, RL_SCALE);
        let full = f64c(h, RL_FULL);
        let lines = |h: &mut _| {
            let n = word(h, this + 0x30).wrapping_add(word(h, this + 0x34));
            n.wrapping_shl(2) >> 1
        };
        if word(h, this + 0x30)
            .wrapping_add(word(h, this + 0x34))
            .wrapping_shl(2)
            & !1
            == 0
        {
            return;
        }
        let mut i = 0_u32;
        loop {
            let e = word(h, this + 0x64).wrapping_add(0x20 * i);
            h.write(e + 8, quiet(a));
            h.write(e + 0xc, quiet(b));
            for k in 0..3 {
                let v = quiet(word(h, color.wrapping_add(4 * k)));
                h.write(e + 0x10 + 4 * k, v);
            }
            if hold == 0 {
                let t = x87::store(x87::add(load(h, e + 4), load(h, e)));
                h.write(e, t);
            }
            let t = load(h, e);
            let intensity;
            if start <= t {
                let p = x87::store(x87::sub(t, start_d));
                let p = x87::load(x87::store(x87::div(x87::load(p), load(h, e + 4))));
                if p < fade || p.is_nan() {
                    let x = x87::sub(full, x87::mul(p, eight));
                    intensity = x87::store(x87::mul(scale, x));
                } else {
                    h.write(e, 0_u32);
                    intensity = 0;
                }
            } else {
                let p = x87::load(x87::store(x87::div(t, load(h, e + 4))));
                if p < fade {
                    intensity = x87::store(x87::mul(scale, x87::mul(eight, p)));
                } else {
                    intensity = 1.0_f32.to_bits();
                }
            }
            h.write(e + 0x1c, intensity);
            let p = [
                quiet(word(h, e + 0x10)),
                quiet(word(h, e + 0x14)),
                quiet(word(h, e + 0x18)),
            ];
            let w = x87::store(x87::mul(load(h, e + 0x1c), load(h, e + 8)));
            let w = if x87::load(w) > 1.0 {
                1.0_f32.to_bits()
            } else {
                w
            };
            let v = word(h, this + 0x54).wrapping_add(0x20 * i);
            for (k, &x) in (0u32..).zip(&p) {
                h.write(v + 4 * k, x);
            }
            h.write(v + 0xc, w);
            let x = x87::store(x87::mul(load(h, e + 0x1c), load(h, e + 0xc)));
            let x = if x87::load(x) > 1.0 {
                1.0_f32.to_bits()
            } else {
                x
            };
            let q = word(h, this + 0x5c).wrapping_add(0x10 * i);
            let t = load(h, e);
            let (q0, q4, q8, qc) = (load(h, q), load(h, q + 4), load(h, q + 8), load(h, q + 0xc));
            let r0 = x87::store(x87::sub(q8, x87::mul(x87::sub(q8, q0), t)));
            let r1 = x87::store(x87::sub(qc, x87::mul(x87::sub(qc, q4), t)));
            let v = word(h, this + 0x54).wrapping_add(0x20 * i);
            for (k, &x) in (0u32..).zip(&p) {
                h.write(v + 0x10 + 4 * k, x);
            }
            h.write(v + 0x1c, x);
            let r = word(h, this + 0x58).wrapping_add(8 + 0x10 * i);
            h.write(r, r0);
            h.write(r + 4, r1);
            i += 1;
            if i >= lines(h) {
                break;
            }
        }
    }
}
