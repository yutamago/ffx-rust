//! Float helpers the menu code shares with the rest of the game. The
//! originals are x87 code at 53-bit precision: double intermediates,
//! rounded to float where they store one; `f64` here does the same.

/// π as the original's double constant (`0x00b92288`, and its negative
/// at `0x00b5c008`): the float π widened.
const PI: f64 = std::f32::consts::PI as f64;
/// 2π likewise (`0x00b39378`).
const TAU: f64 = std::f32::consts::TAU as f64;

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

/// The originals with game data (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The quarter sine wave, `0x2000` `u16`.
    pub const SIN_QUARTER: Global<u16> = Global::new(0x00c5_4e40);

    /// `Math_SinTable(angle)` (`0x0088c690`).
    pub fn math_sin_table(mem: &impl Mem, angle: u32) -> i32 {
        let t = SIN_QUARTER.ptr(mem);
        super::sin_table(angle, |i| t.at(i).get(mem))
    }
}

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
