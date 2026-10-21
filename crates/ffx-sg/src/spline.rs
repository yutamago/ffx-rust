//! The camera splines' near-point search (`spline_get_near_point`): the
//! point of a spline nearest to a position, refined from the last search's
//! parameter.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::{Reg, round_trip};
    use ffx_bridge::{Global, Mem};

    /// The search's spline parameter (`f32`, 0..1), kept between calls
    /// (`spline_get_near_point_init` sets it to the nearest sample).
    pub const NEAR_T: Global<u32> = Global::new(0x01a8_5f30);
    /// The search's sample count (`f32`; `spline_get_near_point_init`:
    /// ten per point): the first step is `10 / samples`.
    pub const NEAR_SAMPLES: Global<u32> = Global::new(0x01a8_5f34);
    /// `10.0` (`f64`, `.rdata`), the first step's numerator.
    pub const TEN: Global<f64> = Global::new(0x00b9_22b0);

    /// How many steps the search takes at most.
    const STEPS: i32 = 0x28;

    fn word(mem: &impl Mem, at: u32, k: u32) -> f32 {
        f32::from_bits(mem.read(at.wrapping_add(4 * k)))
    }

    /// `((P.x - x)^2 + (P.y - y)^2) + (P.z - z)^2`, each difference stored
    /// as an `f32`, the sum too.
    fn distance(mem: &impl Mem, p: u32, pos: [f32; 3]) -> f32 {
        let diff = |k: u32| Reg::load(Reg::load(word(mem, p, k)).sub_m32(pos[k as usize]).store());
        let (dx, dy, dz) = (diff(0), diff(1), diff(2));
        (dx * dx + dy * dy + dz * dz).store()
    }

    /// The three words of point `p` to the three outputs, in order.
    fn put(mem: &mut impl Mem, outs: [u32; 3], p: u32) {
        for (k, out) in (0u32..).zip(outs) {
            let v = round_trip(word(mem, p, k));
            mem.write(out, v.to_bits());
        }
    }

    /// `spline_get_near_point(spline, x, y, z, &ox, &oy, &oz, cubic)`
    /// (`0x00a44120`; callers `MsCameraNearPoints_007c0d40`,
    /// `0x007c0120`, `0x0093f900`, `spline_get_near_point_init`): moves
    /// [`NEAR_T`] along the spline (`spline`: the point count, then the
    /// 16-byte points from `+0x10`) towards the point nearest to `(x, y,
    /// z)`. Step `i` (of up to 0x28) is `d = f32(f32(10 / samples) /
    /// f32(i * i + 1))` ([`NEAR_SAMPLES`]); the spline (`eval`: cubic
    /// `TkMtCalcSpline3`, else `TkMtCalcSpline`, into `locals`, 0x30
    /// bytes) at `t - d`, `t + d` and `t` gives the squared distances `a`,
    /// `b`, `c` ([`distance`]). With `b == a` the point at `t` is the
    /// result. Otherwise, when the side that is nearer (`a` for `b > a`,
    /// else `b`) is not nearer than `c`, the point at `t` is written and
    /// the search goes on; else `t` moves by `d` to that side and its
    /// point is written, but a `t` that leaves 0..1 (below 0, above 1)
    /// ends the search with the point at the old `t` and returns `f32(t
    /// back)` (the original's sum, not the stored `t`). The result is `t`
    /// in `ST0`; the outputs are always written by the steps.
    // coordinates; `fucomp`'s equality; a NaN comparison takes the other way
    #[allow(
        clippy::many_single_char_names,
        clippy::float_cmp,
        clippy::neg_cmp_op_on_partial_ord
    )]
    pub fn spline_get_near_point<M: Mem>(
        mem: &mut M,
        [spline, x, y, z, ox, oy, oz, cubic]: [u32; 8],
        locals: u32,
        eval: &mut dyn FnMut(&mut M, bool, [u32; 4]),
    ) -> f32 {
        let pos = [x, y, z].map(f32::from_bits);
        let outs = [ox, oy, oz];
        let samples = f32::from_bits(NEAR_SAMPLES.get(mem));
        let first = (Reg::constant(TEN.get(mem)) / Reg::load(samples)).store();
        let count: u32 = mem.read(spline);
        let points = spline.wrapping_add(0x10);
        let (a_pt, b_pt, c_pt) = (locals, locals.wrapping_add(0x10), locals.wrapping_add(0x20));
        let cubic = cubic != 0;
        let t_now = |mem: &M| f32::from_bits(NEAR_T.get(mem));
        for i in 0..STEPS {
            let k = Reg::constant(f64::from(i * i + 1)).store();
            let d = Reg::load(first).div_m32(k).store();
            let t = t_now(mem);
            let back = Reg::load(t).sub_m32(d).store();
            eval(mem, cubic, [a_pt, points, count, back.to_bits()]);
            let a = Reg::load(distance(mem, a_pt, pos)).value();
            let on = Reg::load(t_now(mem)).add_m32(d).store();
            eval(mem, cubic, [b_pt, points, count, on.to_bits()]);
            let b = Reg::load(distance(mem, b_pt, pos)).value();
            let t = round_trip(t_now(mem));
            eval(mem, cubic, [c_pt, points, count, t.to_bits()]);
            let c = Reg::load(distance(mem, c_pt, pos)).value();
            if b == a {
                put(mem, outs, c_pt);
                break;
            }
            let forward = !(b > a);
            let near = if forward { b } else { a };
            if !(c > near) {
                put(mem, outs, c_pt);
                continue;
            }
            let t = Reg::load(t_now(mem));
            if forward {
                let moved = (t + Reg::load(d)).store();
                NEAR_T.set(mem, moved.to_bits());
                if Reg::load(moved).value() > 1.0 {
                    put(mem, outs, c_pt);
                    return Reg::load(d).subr_m32(moved).store();
                }
                put(mem, outs, b_pt);
            } else {
                let moved = (t - Reg::load(d)).store();
                NEAR_T.set(mem, moved.to_bits());
                if Reg::load(moved).value() < 0.0 {
                    put(mem, outs, c_pt);
                    return Reg::load(d).add_m32(moved).store();
                }
                put(mem, outs, a_pt);
            }
        }
        round_trip(t_now(mem))
    }
}
