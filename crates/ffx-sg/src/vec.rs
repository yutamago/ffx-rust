//! Small float helpers of the main module: three-component vector
//! arithmetic (used by the cloth and triangle tests), the global wind,
//! the camera's defaults, a vector's length, radians to degrees.
//!
//! All x87 at the game's 53-bit precision (`ffx_bridge::x87`): each
//! component is computed and stored before the next one is read, so an
//! output that overlaps an input behaves as in the original.

#![allow(clippy::many_single_char_names)] // vectors and their components

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::{self, Reg};
    use ffx_bridge::{Global, Mem};

    /// The global wind: direction x, y, z (`Sg_GlobalSetWind`).
    pub const WIND: Global<[f32; 3]> = Global::new(0x012f_bc44);
    /// The wind's default x (`f32` 0.01 in `.rdata`, `Sg_GlobalInitWind`).
    pub const WIND_DEFAULT: Global<f32> = Global::new(0x00b1_f2b0);
    /// The camera: a word and two floats (`Sg_InitCamera`).
    pub const CAMERA: Global<u32> = Global::new(0x0130_1bb0);
    /// The camera's default (`f32` 50.0 in `.rdata`).
    pub const CAMERA_DEFAULT: Global<f32> = Global::new(0x00b1_f2c8);
    /// 180 (`f64` in `.rdata`).
    pub const DEG_180: Global<f64> = Global::new(0x00b4_8378);
    /// π (`f64` in `.rdata`).
    pub const PI: Global<f64> = Global::new(0x00b5_82f0);

    fn get(mem: &impl Mem, v: u32, k: u32) -> f32 {
        mem.read(v.wrapping_add(4 * k))
    }

    fn put(mem: &mut impl Mem, v: u32, k: u32, x: f32) {
        mem.write(v.wrapping_add(4 * k), x);
    }

    /// `Sg_VecAdd(out, a, b)` (`0x00824ce0`): `out = a + b`.
    pub fn add(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        for k in 0..3 {
            let x = Reg::load(get(mem, a, k)).add_m32(get(mem, b, k));
            put(mem, out, k, x.store());
        }
    }

    /// `Sg_VecSub(out, a, b)` (`0x00824da0`): `out = a - b`.
    pub fn sub(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        for k in 0..3 {
            let x = Reg::load(get(mem, a, k)).sub_m32(get(mem, b, k));
            put(mem, out, k, x.store());
        }
    }

    /// `Sg_VecDot(a, b, &out)` (`0x00824d40`): `(a.y b.y + a.x b.x) + a.z b.z`.
    pub fn dot(mem: &mut impl Mem, a: u32, b: u32, out: u32) {
        let [x, y, z] = [0, 1, 2].map(|k| Reg::load(get(mem, a, k)).mul_m32(get(mem, b, k)));
        mem.write(out, ((y + x) + z).store());
    }

    /// `Sg_VecScale(out, v, s)` (`0x00824d70`): `out = v * s` (`x` and `y`
    /// multiply registers, `z` is `s * [v.z]`).
    pub fn scale(mem: &mut impl Mem, out: u32, v: u32, s: f32) {
        let s = Reg::load(s);
        let x = Reg::load(get(mem, v, 0)) * s;
        put(mem, out, 0, x.store());
        let y = Reg::load(get(mem, v, 1)) * s;
        put(mem, out, 1, y.store());
        let z = s.mul_m32(get(mem, v, 2));
        put(mem, out, 2, z.store());
    }

    /// `Sg_VecCopy4(out, v)` (`0x00824d10`): four floats through the x87
    /// (a signalling NaN comes out quiet).
    pub fn copy4(mem: &mut impl Mem, out: u32, v: u32) {
        for k in 0..4 {
            let x = x87::round_trip(get(mem, v, k));
            put(mem, out, k, x);
        }
    }

    /// `Sg_GlobalSetWind(x, y, z)` (`0x00824e30`).
    pub fn global_set_wind(mem: &mut impl Mem, x: f32, y: f32, z: f32) {
        WIND.set(mem, [x, y, z].map(x87::round_trip));
    }

    /// `Sg_GlobalInitWind()` (`0x00824e10`): `(0.01, 0, 0)`.
    pub fn global_init_wind(mem: &mut impl Mem) {
        let x = x87::round_trip(WIND_DEFAULT.get(mem));
        WIND.set(mem, [x, 0.0, 0.0]);
    }

    /// `Sg_InitCamera()` (`0x0083f780`): the word 0, both floats 50.
    pub fn init_camera(mem: &mut impl Mem) {
        let d = x87::round_trip(CAMERA_DEFAULT.get(mem));
        let p = CAMERA.ptr(mem).addr;
        mem.write(p, 0u32);
        mem.write(p.wrapping_add(4), [d, d]);
    }

    /// `Sg_RadToDeg(r)` (`0x008366f0`): `r * 180 / π`, stored as `f32`.
    pub fn rad_to_deg(mem: &impl Mem, r: f32) -> f32 {
        (Reg::load(r) * Reg::constant(DEG_180.get(mem)) / Reg::constant(PI.get(mem))).store()
    }

    /// `Sg_VectorGetLength(v)` (`0x00836670`): `sqrt((x² + y²) + z²)` (the
    /// CRT's `_CIsqrt`, the x87's `fsqrt`), stored as `f32`.
    pub fn vector_get_length(mem: &impl Mem, v: u32) -> f32 {
        let [x, y, z] = [0, 1, 2].map(|k| Reg::load(get(mem, v, k)));
        let s = (x * x + y * y) + z * z;
        // `fsqrt` passes a NaN through (Rust's `sqrt` may not keep its bits).
        let r = if s.value().is_nan() {
            s
        } else {
            Reg::constant(s.value().sqrt())
        };
        r.store()
    }
}
