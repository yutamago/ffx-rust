//! Float helpers the menu code shares with the rest of the game. The
//! originals are x87 code at 53-bit precision: double intermediates,
//! rounded to float where they store one; `f64` here does the same.

/// π as the original's double constant (`0x00b92288`, and its negative
/// at `0x00b5c008`): the float π widened.
const PI: f64 = std::f32::consts::PI as f64;
/// 2π likewise (`0x00b39378`).
const TAU: f64 = std::f32::consts::TAU as f64;

/// `Math_ClampPi(a)` (`0x0088b800`, own name): `a` clamped to
/// [−π, π] (π rounded to a float); a NaN passes through.
#[must_use]
pub fn clamp_pi(a: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let mut a = a;
    if a < -pi {
        a = -pi;
    }
    if a > pi {
        a = pi;
    }
    a
}

/// `CIsqrt(x)` (`0x0088c400`): `_CIsqrt` (`fsqrt` at 53 bits) stored to a
/// float; the double rounding equals one rounding for a square root.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn ci_sqrt(x: f32) -> f32 {
    f64::from(x).sqrt() as f32
}

/// `Math_WrapAngleFloor(a)` (`0x0088c630`, own name): `a` moved into
/// [−π, π) by whole turns with `floor`, each step stored as a float.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn wrap_angle_floor(a: f32) -> f32 {
    let a = (f64::from(a) + PI) as f32;
    let t = (f64::from(a) / TAU) as f32;
    let turns = f64::from(t).floor() as f32;
    let r = (f64::from(a) - f64::from(turns) * TAU) as f32;
    (f64::from(r) - PI) as f32
}

/// The `CI*` float wrappers (`CIcos` `0x0088b3e0`, `CIsin` `0x0088c3a0`,
/// `CItan` `0x0088c420`): `x` loaded as a float, the C runtime's x87
/// intrinsic (`intrinsic`, on the widened value), the result stored as a
/// float.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn ci_unary(x: f32, intrinsic: impl FnOnce(f64) -> f64) -> f32 {
    intrinsic(f64::from(ffx_bridge::x87::round_trip(x))) as f32
}

/// `CIatan2(y, x)` (`0x00889e70`, own name): `_CIatan2` with `y` in `ST1`
/// and `x` in `ST0`, stored as a float.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn ci_atan2(y: f32, x: f32, intrinsic: impl FnOnce(f64, f64) -> f64) -> f32 {
    let r = ffx_bridge::x87::round_trip;
    intrinsic(f64::from(r(y)), f64::from(r(x))) as f32
}

/// `Math_WrapAngle(a)`: `a` moved into [−π, π] by whole turns, one turn at
/// a time (each step rounded to a float). Like the original it never ends
/// for an infinity or a magnitude where a turn no longer changes the float.
#[must_use]
pub fn wrap_angle(a: f32) -> f32 {
    let mut a = a;
    #[allow(clippy::cast_possible_truncation)] // the original's fstp to a float
    {
        while f64::from(a) < -PI {
            a = (f64::from(a) + TAU) as f32;
        }
        while f64::from(a) > PI {
            a = (f64::from(a) - TAU) as f32;
        }
    }
    a
}

/// `Math_InRadius(r, dx, dy, dz)`: whether `dx² + dy² + dz² <= r²`, the
/// squares rounded to floats, their sum a double; false with a NaN.
#[must_use]
pub fn in_radius(r: f32, dx: f32, dy: f32, dz: f32) -> bool {
    let sum = f64::from(dx * dx) + f64::from(dy * dy) + f64::from(dz * dz);
    f64::from(r * r) >= sum
}

/// The original's degree factor (`0x00b11330`): the float π/180 widened.
const DEG: f64 = f32::from_bits(0x3c8e_fa35) as f64;

/// `TkDegtoRad(deg)`: radians (a double product rounded to a float),
/// wrapped by [`wrap_angle`].
#[must_use]
pub fn deg_to_rad(deg: f32) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    wrap_angle((f64::from(deg) * DEG) as f32)
}

/// `Math_MinAbs(a, b)` (`0x0088b400`): `|a|` when `b` is greater, else `b`
/// (so a NaN `b` stays).
#[must_use]
pub fn min_abs(a: f32, b: f32) -> f32 {
    if b > a.abs() { a.abs() } else { b }
}

/// `Math_LengthSq3(x, y, z)` (`0x0088c3c0`): `x² + y² + z²`, the squares
/// rounded to floats, the sum a double rounded to a float.
#[must_use]
pub fn length_sq3(x: f32, y: f32, z: f32) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    {
        (f64::from(x * x) + f64::from(y * y) + f64::from(z * z)) as f32
    }
}

/// `Math_RoundUp(n, step)` (`0x0088d120`): `n` up to a multiple of `step`
/// (unsigned; a zero `step` faults like the original's `div`).
#[must_use]
pub fn round_up(n: u32, step: u32) -> u32 {
    (n.wrapping_sub(1).wrapping_add(step) / step).wrapping_mul(step)
}

/// `Math_Clamp1To99(n)` (`0x008ced10`).
#[must_use]
pub fn clamp_1_to_99(n: i32) -> i32 {
    n.clamp(1, 99)
}

/// `Math_SinTable(angle)` from the quarter-wave table: `angle` in
/// 1/0x8000 turns (15 bits), the table's `0x2000` entries the first
/// quarter.
pub fn sin_table(angle: u32, quarter: impl Fn(u32) -> u16) -> i32 {
    let a = angle & 0x7fff;
    let (a, sign) = if a < 0x4000 { (a, 1) } else { (a - 0x4000, -1) };
    let i = if a < 0x2000 { a } else { 0x3fff - a };
    sign * i32::from(quarter(i))
}

/// `Math_UnitMatrix(m)`'s result: a 4×4 float identity.
pub const UNIT_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

/// `Math_Det3Points(x1, y1, x2, y2, x3, y3)` (`0x0088bc70`): twice the
/// signed area of the triangle, `x1·y2 + x2·y3 + x3·y1 − x1·y3 − x2·y1 −
/// x3·y2` summed in that order in doubles, rounded to a float.
#[must_use]
pub fn det3_points(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) -> f32 {
    let p = |a: f32, b: f32| f64::from(a) * f64::from(b);
    let v = p(x1, y2) + p(x2, y3) + p(x3, y1) - p(x1, y3) - p(x2, y1) - p(x3, y2);
    #[allow(clippy::cast_possible_truncation)]
    {
        v as f32
    }
}

/// `TkScreenToMenuX(x)` (`0x008e2690`) and `TkScreenToMenuY(y)`
/// (`0x008e26b0`): a 1920×1080 coordinate in the menu's 512×416 space;
/// the integer goes through a float first.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn screen_to_menu(v: i32, menu: f64, screen: f64) -> f32 {
    (f64::from(v as f32) * menu / screen) as f32
}

/// The originals with game data (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The quarter sine wave, `0x2000` `u16`.
    pub const SIN_QUARTER: Global<u16> = Global::new(0x00c5_4e40);

    /// `Math_UnitMatrix(m)` (`0x0088c510`): zeroes then the diagonal.
    pub fn math_unit_matrix(mem: &mut impl Mem, m: u32) {
        mem.write(m, [0f32; 16]);
        mem.write(m, super::UNIT_MATRIX);
    }

    /// `Math_SinTable(angle)` (`0x0088c690`).
    pub fn math_sin_table(mem: &impl Mem, angle: u32) -> i32 {
        let t = SIN_QUARTER.ptr(mem);
        super::sin_table(angle, |i| t.at(i).get(mem))
    }
}

/// `Math_CrossHeightGap(p1, p2, p3, p4)` (`0x00889e90`): where segments
/// `p1`-`p2` and `p3`-`p4` cross in the x-z plane (at `s` along the
/// second, `u` along the first), the gap between their heights, `|y1 -
/// y2|`; -1 (`[0x00b92340]`) for parallel segments. Every step rounded to
/// a float (the original is unoptimised x87 code at 53 bits).
#[must_use]
#[allow(clippy::many_single_char_names, clippy::similar_names)]
pub fn cross_height_gap(p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], p4: [f32; 3]) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    let r = |x: f64| x as f32;
    let f = f64::from;
    let a = r(f(p2[0]) - f(p1[0]));
    let b = r(f(p2[2]) - f(p1[2]));
    let c = r(f(p4[0]) - f(p3[0]));
    let d = r(f(p4[2]) - f(p3[2]));
    let e = r(f(p3[0]) - f(p1[0]));
    let g = r(f(p3[2]) - f(p1[2]));
    let det = r(f(a) * f(d) - f(c) * f(b));
    if det == 0.0 {
        return -1.0;
    }
    let s = r((f(e) * f(b) - f(a) * f(g)) / f(det));
    let u = if a == 0.0 {
        r((f(d) * f(s) + f(g)) / f(b))
    } else {
        r((f(c) * f(s) + f(e)) / f(a))
    };
    let y1 = r((f(p4[1]) - f(p3[1])) * f(s) + f(p3[1]));
    let y2 = r((f(p2[1]) - f(p1[1])) * f(u) + f(p1[1]));
    r((f(y1) - f(y2)).abs())
}

/// `Math_Atan2XZ(x0, z0, x1, z1)` (`0x0088b430`): the angle of
/// `(x1 - x0, z1 - z0)` (each difference a float): on the axes
/// ±π/2 (`dx` zero, minus for a negative or NaN `dz`) and π or -0
/// (`dz` zero, π for a negative or NaN `dx`), otherwise
/// `_CIatan2(dz, dx)` stored as a float.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // fstp dword
pub fn atan2_xz(
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    intrinsic: impl FnOnce(f64, f64) -> f64,
) -> f32 {
    let r = |v: f32| f64::from(ffx_bridge::x87::round_trip(v));
    let dx = (r(x1) - r(x0)) as f32;
    let dz = (r(z1) - r(z0)) as f32;
    #[allow(clippy::float_cmp)] // fucompp against 0
    if dx.abs() == 0.0 {
        return if dz >= 0.0 {
            FRAC_PI_2_F32
        } else {
            -FRAC_PI_2_F32
        };
    }
    #[allow(clippy::float_cmp)]
    if dz.abs() == 0.0 {
        return if dx >= 0.0 { -0.0 } else { PI_F32 };
    }
    intrinsic(f64::from(dz), f64::from(dx)) as f32
}

/// The float constants `Math_Atan2XZ` loads: π/2 (`0x00b43d38`) and π
/// (`0x00b922d0`).
const FRAC_PI_2_F32: f32 = f32::from_bits(0x3fc9_0fdb);
const PI_F32: f32 = f32::from_bits(0x4049_0fdb);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_and_measures() {
        assert_eq!(wrap_angle(0.5), 0.5);
        assert_eq!(wrap_angle(std::f32::consts::PI), std::f32::consts::PI);
        assert!((wrap_angle(7.0) - (7.0 - std::f32::consts::TAU)).abs() < 1e-6);
        assert!((wrap_angle(-20.0) - (-20.0 + 3.0 * std::f32::consts::TAU)).abs() < 1e-5);
        assert!(in_radius(5.0, 3.0, 4.0, 0.0));
        assert!(!in_radius(4.9, 3.0, 4.0, 0.0));
        assert!(!in_radius(f32::NAN, 0.0, 0.0, 0.0));
        assert!((deg_to_rad(270.0) + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }
}
