//! M5 stage 1, round 7 (list `left7`, from `pc-ui.list`): the sphere
//! grid's moving token (`AbmapUpdateMovingChrPos`): a point carried from
//! one node to the next along an arc around the grid's centre.
//!
//! Floats follow the x87 at 53 bits: f64 in the original's order, rounded
//! to f32 at each of its stores (the emulated VU0 registers and the frame's
//! spills included); `_CIatan2` and `_CIsqrt` stay calls to the CRT.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's quantities
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::Host;

    /// The CRT intrinsics `_CIatan2` (`ST1`, `ST0`) and `_CIsqrt`.
    pub const CI_ATAN2: u32 = 0x0094_98bc;
    pub const CI_SQRT: u32 = 0x0094_97b8;
    /// `g_Vu0_Vf00` (x, y, z, w), the emulated VU0 `vf` cells the
    /// original computes in (0x00c8f810.., 0x00c8f820.., 0x00c8f830..) and
    /// `Q`; `g_pppSinTable` (4096 f32 over a turn).
    pub const VU0_VF00: u32 = 0x00c0_a004;
    pub const VU0_A: u32 = 0x00c8_f810;
    pub const VU0_B: u32 = 0x00c8_f820;
    pub const VU0_C: u32 = 0x00c8_f830;
    pub const VU0_Q: u32 = 0x00c8_f788;
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// f64 pi, 2 pi, -pi (each the f32 value) and 32768.
    pub const PI: u32 = 0x00b9_2288;
    pub const TWO_PI: u32 = 0x00b3_9378;
    pub const MINUS_PI: u32 = 0x00b5_c008;
    pub const HALF_TURN: u32 = 0x00b5_1588;

    fn f(h: &impl Host, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    fn k64(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `fstp dword` of `x` to `p`, the bits.
    fn put(h: &mut impl Host, p: u32, x: f64) -> u32 {
        let b = x87::store(x);
        h.write(p, b);
        b
    }

    /// The emulated VU0 length of the vector in `VU0_C` (its x, y, z
    /// cells already hold the vector): the squares, their sum through
    /// `VU0_B`'s y and z, `_CIsqrt` of its magnitude (each step stored as
    /// f32).
    fn vu0_length(h: &mut impl Host, x: u32, y: u32) -> u32 {
        let c = h.rebase(VU0_C);
        let b = h.rebase(VU0_B);
        // the squares: x by itself, y by its cell, z (0) by 0
        put(h, c, x87::mul(x87::load(x), x87::load(x)));
        let yy = x87::mul(x87::load(y), f(h, c + 4));
        put(h, c + 4, yy);
        let zz = x87::mul(f(h, c + 8), 0.0);
        put(h, c + 8, zz);
        let s = x87::add(f(h, c + 4), f(h, c));
        put(h, b + 4, s);
        let s = x87::add(f(h, c + 8), f(h, b + 4));
        put(h, b + 8, s);
        let m = x87::store(f(h, b + 8).abs());
        let sqrt = h.rebase(CI_SQRT);
        h.call_intrinsic(sqrt, x87::load(m)).to_bits()
    }

    /// `AbmapUpdateMovingChrPos(out, from, to, centre, t)`: `out` (x, y,
    /// z, w) a point between `from` and `to` on the arc around `centre`:
    /// the angles of `from - centre` and `to - centre` (`_CIatan2`), their
    /// difference brought into -pi..pi (`+-2 pi` once), the distances
    /// through the emulated VU0 (whose cells, `Q` and the normalised
    /// `to - centre` are left as the original leaves them), both
    /// interpolated by `t`; the angle as a 16-bit turn (`ftol(a * 32768 /
    /// pi)`) into the sine table (`(u + 0x4000) >> 4` its cosine, `u >> 4`
    /// its sine); z the centre's, w 1. Returns the sine's index (the
    /// `eax` the original leaves).
    pub fn update_moving_chr_pos(
        h: &mut impl Host,
        out: u32,
        from: u32,
        to: u32,
        centre: u32,
        t: u32,
    ) -> u32 {
        let (cx, cy) = (f(h, centre), f(h, centre + 4));
        let d1x = x87::store(x87::sub(f(h, from), cx));
        let d1y = x87::store(x87::sub(f(h, from + 4), cy));
        let atan2 = h.rebase(CI_ATAN2);
        let ang1 = h
            .call_intrinsic2(atan2, x87::load(d1y), x87::load(d1x))
            .to_bits();
        let d2x = x87::store(x87::sub(f(h, to), cx));
        let d2y = x87::store(x87::sub(f(h, to + 4), cy));
        let ang2 = h
            .call_intrinsic2(atan2, x87::load(d2y), x87::load(d2x))
            .to_bits();
        let one = 1.0_f32.to_bits();
        let c = h.rebase(VU0_C);
        let v0 = h.rebase(VU0_VF00);
        for (o, v) in [(0, d1x), (4, d1y), (8, 0), (0xc, one)] {
            h.write(c + o, v);
        }
        let r1 = vu0_length(h, d1x, d1y);
        for (o, v) in [(0, d2x), (4, d2y), (8, 0), (0xc, one)] {
            h.write(c + o, v);
        }
        let len1 = x87::load(x87::store(x87::add(x87::load(r1), f(h, v0))));
        // to - centre plus vf00 into the first cell
        let a = h.rebase(VU0_A);
        put(h, a, x87::add(f(h, v0), x87::load(d2x)));
        put(h, a + 4, x87::add(x87::load(d2y), f(h, v0 + 4)));
        put(h, a + 8, x87::add(f(h, v0 + 8), 0.0));
        let r2 = vu0_length(h, d2x, d2y);
        let len2 = put(h, c, x87::add(x87::load(r2), f(h, v0)));
        let q = put(
            h,
            h.rebase(VU0_Q),
            x87::div(f(h, v0 + 0xc), x87::load(len2)),
        );
        let q = x87::load(q);
        let b = h.rebase(VU0_B);
        put(h, b, x87::mul(f(h, a), q));
        put(h, b + 4, x87::mul(f(h, a + 4), q));
        put(h, b + 8, x87::mul(q, f(h, a + 8)));
        // the turn: the difference into -pi..pi
        let (a1, a2) = (x87::load(ang1), x87::load(ang2));
        let pi = k64(h, PI);
        let mut d = x87::load(x87::store(x87::sub(a2, a1)));
        if a1 <= a2 {
            if d > pi {
                d = x87::load(x87::store(x87::sub(d, k64(h, TWO_PI))));
            }
        } else if d < k64(h, MINUS_PI) {
            d = x87::load(x87::store(x87::add(d, k64(h, TWO_PI))));
        }
        let (len1, len2) = (len1, x87::load(len2));
        let t = x87::load(t);
        let radius = x87::store(x87::add(len1, x87::mul(x87::sub(len2, len1), t)));
        let angle = x87::store(x87::add(a1, x87::mul(d, t)));
        let turn = x87::div(x87::mul(x87::load(angle), k64(h, HALF_TURN)), pi);
        let u = x87::ftol(h, turn) & 0xffff;
        let table = h.rebase(SIN_TABLE);
        let cos = f(h, table + 4 * (((u + 0x4000) >> 4) & 0xfff));
        let sin = f(h, table + 4 * (u >> 4));
        let r = x87::load(radius);
        // the centre read again (fld [edi], fadd [edi+4])
        let cx = f(h, centre);
        put(h, out, x87::add(x87::mul(cos, r), cx));
        let cy = f(h, centre + 4);
        put(h, out + 4, x87::add(x87::mul(r, sin), cy));
        let z = f(h, centre + 8);
        put(h, out + 8, z);
        h.write(out + 12, one);
        u >> 4
    }
}
