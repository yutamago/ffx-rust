//! Lights: the point lights (`Sg_PLight*`), the light matrices, the water
//! surface height (`Sg_SetSuimen`).
//!
//! The point lights are a table of `0x2c`-byte entries the global
//! [`bridge::PLIGHTS`] points to; [`bridge::PLIGHT_LAST`] is the last
//! valid index. Every accessor compares the index with it signed (`jg`),
//! so a negative index reaches before the table, as in the original. An
//! entry: position at `+0x0`, colour at `+0x10`, radius `f32` at `+0x20`,
//! attenuation `f32` at `+0x24`, enabled `u8` at `+0x28`, the light it
//! belongs to `i8` at `+0x29` (`docs/formats/main.md`).

/// Bytes per point light.
pub const PLIGHT_SIZE: u32 = 0x2c;
/// The colour, three `f32` from 0 to 1.
pub const PLIGHT_COLOR: u32 = 0x10;
/// Radius (`f32`).
pub const PLIGHT_RADIUS: u32 = 0x20;
/// Attenuation (`f32`).
pub const PLIGHT_ATTENUATION: u32 = 0x24;
/// Enabled (`u8`).
pub const PLIGHT_ENABLE: u32 = 0x28;
/// The light (`i8`).
pub const PLIGHT_LIGHT: u32 = 0x29;

/// The originals over game memory (`ffx_bridge::Mem`).
#[allow(clippy::many_single_char_names)] // index, pointers, channels
pub mod bridge {
    use ffx_bridge::{Global, Mem, x87};

    use super::{PLIGHT_ATTENUATION, PLIGHT_ENABLE, PLIGHT_LIGHT, PLIGHT_RADIUS, PLIGHT_SIZE};
    use crate::lo8;

    /// The point light table (`Sg_PLightGetPtr` returns this global's address).
    pub const PLIGHTS: Global<u32> = Global::new(0x0130_08cc);
    /// The last valid point light index (`i32`).
    pub const PLIGHT_LAST: Global<i32> = Global::new(0x0130_08d0);
    /// The colour scale of the point lights (`f64` 255.0 in `.rdata`).
    pub const COLOR_SCALE: Global<f64> = Global::new(0x00b1_70c0);
    /// The light normal (direction) matrix (`Sg_LightSetDir` fills it).
    pub const NORMAL_MATRIX: Global<u8> = Global::new(0x0130_0978);
    /// The light colour matrix (`Sg_LightSetColor` fills it).
    pub const COLOR_MATRIX: Global<u8> = Global::new(0x023c_4400);
    /// The water surface height (`Sg_SetSuimen`, `Sg_GetSuimen`).
    pub const SUIMEN: Global<f32> = Global::new(0x0130_09d8);
    /// The water limit (`Sg_SetWaterLimit`; `Sg_SetSuimen` sets it too).
    pub const WATER_LIMIT: Global<f32> = Global::new(0x0130_09dc);

    /// The address of field `off` of point light `i`, or `None` past the
    /// last one.
    fn field(mem: &impl Mem, i: i32, off: u32) -> Option<u32> {
        (i <= PLIGHT_LAST.get(mem)).then(|| {
            PLIGHTS
                .get(mem)
                .wrapping_add(i.cast_unsigned().wrapping_mul(PLIGHT_SIZE))
                .wrapping_add(off)
        })
    }

    fn set_f32(mem: &mut impl Mem, i: i32, off: u32, v: f32) {
        if let Some(at) = field(mem, i, off) {
            mem.write(at, x87::round_trip(v));
        }
    }

    fn set_u8(mem: &mut impl Mem, i: i32, off: u32, v: u32) {
        if let Some(at) = field(mem, i, off) {
            mem.write(at, lo8(v));
        }
    }

    fn get_f32(mem: &impl Mem, i: i32, off: u32) -> f32 {
        field(mem, i, off).map_or(0.0, |at| mem.read(at))
    }

    fn get_i8(mem: &impl Mem, i: i32, off: u32) -> i32 {
        field(mem, i, off).map_or(0, |at| i32::from(mem.read::<i8>(at)))
    }

    /// `Sg_PLightGetPtr()` (`0x00831b70`): the address of the table pointer.
    pub fn get_ptr(mem: &impl Mem) -> u32 {
        PLIGHTS.ptr(mem).addr
    }
    /// `Sg_PLightGetAttenuation(i)` (`0x00831a40`): 0 past the last light.
    pub fn get_attenuation(mem: &impl Mem, i: i32) -> f32 {
        get_f32(mem, i, PLIGHT_ATTENUATION)
    }
    /// `Sg_PLightGetRadius(i)` (`0x00831b80`).
    pub fn get_radius(mem: &impl Mem, i: i32) -> f32 {
        get_f32(mem, i, PLIGHT_RADIUS)
    }
    /// `Sg_PLightGetEnable(i)` (`0x00831ad0`): sign-extended, 0 past the last light.
    pub fn get_enable(mem: &impl Mem, i: i32) -> i32 {
        get_i8(mem, i, PLIGHT_ENABLE)
    }
    /// `Sg_PLightGetLight(i)` (`0x00831b00`).
    pub fn get_light(mem: &impl Mem, i: i32) -> i32 {
        get_i8(mem, i, PLIGHT_LIGHT)
    }
    /// `Sg_PLightSetAttenuation(i, a)` (`0x00831bf0`).
    pub fn set_attenuation(mem: &mut impl Mem, i: i32, a: f32) {
        set_f32(mem, i, PLIGHT_ATTENUATION, a);
    }
    /// `Sg_PLightSetRadius(i, r)` (`0x00831ce0`).
    pub fn set_radius(mem: &mut impl Mem, i: i32, r: f32) {
        set_f32(mem, i, PLIGHT_RADIUS, r);
    }
    /// `Sg_PLightSetEnable(i, on)` (`0x00831c60`).
    pub fn set_enable(mem: &mut impl Mem, i: i32, on: u32) {
        set_u8(mem, i, PLIGHT_ENABLE, on);
    }
    /// `Sg_PLightSetLight(i, light)` (`0x00831c80`).
    pub fn set_light(mem: &mut impl Mem, i: i32, light: u32) {
        set_u8(mem, i, PLIGHT_LIGHT, light);
    }

    /// `Sg_PLightGetPos(i, &x, &y, &z)` (`0x00831b30`): each non-null
    /// output gets its coordinate.
    pub fn get_pos(mem: &mut impl Mem, i: i32, out: [u32; 3]) {
        let Some(p) = field(mem, i, 0) else { return };
        for (k, o) in (0..).zip(out) {
            if o != 0 {
                let v: f32 = mem.read(p.wrapping_add(4 * k));
                mem.write(o, x87::round_trip(v));
            }
        }
    }

    /// `Sg_PLightSetPos(i, mask, x, y, z)` (`0x00831ca0`): the coordinates
    /// whose bit (1, 2, 4) is in `mask`.
    pub fn set_pos(mem: &mut impl Mem, i: i32, mask: u32, xyz: [f32; 3]) {
        let Some(p) = field(mem, i, 0) else { return };
        for (k, v) in (0..).zip(xyz) {
            if mask & (1 << k) != 0 {
                mem.write(p.wrapping_add(4 * k), x87::round_trip(v));
            }
        }
    }

    /// `Sg_PLightSetColor(i, r, g, b)` (`0x00831c10`): each channel (an
    /// integer, rounded to `f32` first) over [`COLOR_SCALE`].
    pub fn set_color(mem: &mut impl Mem, i: i32, rgb: [i32; 3]) {
        let Some(p) = field(mem, i, super::PLIGHT_COLOR) else {
            return;
        };
        let scale = x87::Reg::constant(COLOR_SCALE.get(mem));
        for (k, c) in (0..).zip(rgb) {
            let c = f32::from_bits(x87::store(f64::from(c)));
            // The last channel is `fdivr m32`; the others divide registers.
            let v = if k == 2 {
                scale.divr_m32(c)
            } else {
                x87::Reg::load(c) / scale
            };
            mem.write(p.wrapping_add(4 * k), v.store());
        }
    }

    /// `Sg_PLightGetColor(i, &r, &g, &b)` (`0x00831a60`): each non-null
    /// output gets its channel times [`COLOR_SCALE`] as an integer
    /// (`_ftol2_sse`).
    pub fn get_color(mem: &mut impl Mem, i: i32, out: [u32; 3]) {
        let Some(p) = field(mem, i, super::PLIGHT_COLOR) else {
            return;
        };
        let scale = x87::Reg::constant(COLOR_SCALE.get(mem));
        for (k, o) in (0..).zip(out) {
            if o != 0 {
                let c: f32 = mem.read(p.wrapping_add(4 * k));
                // The last channel is `fmul m32`; the others multiply registers.
                let v = if k == 2 {
                    scale.mul_m32(c)
                } else {
                    x87::Reg::load(c) * scale
                };
                let n = x87::ftol(mem, v.value());
                mem.write(o, n);
            }
        }
    }

    /// `Sg_LightGetColorMatrixPtr()` (`0x008361b0`).
    pub fn get_color_matrix_ptr(mem: &impl Mem) -> u32 {
        COLOR_MATRIX.ptr(mem).addr
    }
    /// `Sg_LightGetNormalMatrixPtr()` (`0x00836240`).
    pub fn get_normal_matrix_ptr(mem: &impl Mem) -> u32 {
        NORMAL_MATRIX.ptr(mem).addr
    }

    /// `Sg_GetSuimen()` (`0x008360a0`).
    pub fn get_suimen(mem: &impl Mem) -> f32 {
        SUIMEN.get(mem)
    }
    /// `Sg_SetSuimen(h)` (`0x00836640`): the height and the water limit.
    pub fn set_suimen(mem: &mut impl Mem, h: f32) {
        let h = x87::round_trip(h);
        SUIMEN.set(mem, h);
        WATER_LIMIT.set(mem, h);
    }
    /// `Sg_SetWaterLimit(h)` (`0x00836660`).
    pub fn set_water_limit(mem: &mut impl Mem, h: f32) {
        WATER_LIMIT.set(mem, x87::round_trip(h));
    }
}
