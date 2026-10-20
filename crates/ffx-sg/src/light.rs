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
    use ffx_bridge::x87::Reg;
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

    /// The three light directions (16 bytes each): two in `.data`, the
    /// third at `0x013008e8` (`Sg_LightGetDir`).
    pub const LIGHT_DIRS: [Global<u8>; 3] = [
        Global::new(0x00c4_9724),
        Global::new(0x00c4_9734),
        Global::new(0x0130_08e8),
    ];
    /// The three light colours and the ambient colour, 16 bytes apart
    /// (`Sg_LightGetColor`).
    pub const LIGHT_COLORS: Global<[u32; 4]> = Global::new(0x00c4_9744);
    /// The attenuation `Sg_PLightDefine` gives (`f32` in `.rdata`).
    pub const DEFAULT_ATTENUATION: Global<f32> = Global::new(0x00b9_2268);

    /// The address of light colour `i` (0..=3, 3 the ambient).
    fn color(mem: &impl Mem, i: u32) -> u32 {
        LIGHT_COLORS.ptr(mem).addr.wrapping_add(16 * i)
    }

    fn copy16(mem: &mut impl Mem, from: u32, to: u32) {
        let v: [u32; 4] = mem.read(from);
        mem.write(to, v);
    }

    /// `Sg_LightGetColor(i, &c)` (`0x008360f0`): colour `i` (0..=3), 16
    /// bytes; nothing for another `i`.
    pub fn get_color_of(mem: &mut impl Mem, i: u32, out: u32) {
        if i <= 3 {
            let at = color(mem, i);
            copy16(mem, at, out);
        }
    }

    /// `Sg_LightGetDir(i, &d)` (`0x008361c0`): direction `i` (0..=2), 16
    /// bytes; nothing for another `i`.
    pub fn get_dir(mem: &mut impl Mem, i: u32, out: u32) {
        if let Some(g) = LIGHT_DIRS.get(i as usize) {
            let at = g.ptr(mem).addr;
            copy16(mem, at, out);
        }
    }

    /// `Sg_GetLightPos(_, &d)` (`0x00836070`): the first direction into `d`
    /// when not null (the first argument is unused); its address.
    pub fn get_light_pos(mem: &mut impl Mem, out: u32) -> u32 {
        let at = LIGHT_DIRS[0].ptr(mem).addr;
        if out != 0 {
            copy16(mem, at, out);
        }
        at
    }

    /// The light matrices from three directions and four colours (by
    /// address): `FFXVu0NormalLightMatrix`, then `FFXVu0LightColorMatrix`.
    fn matrices(mem: &mut impl Mem, dirs: [u32; 3], colors: [u32; 4]) -> (u32, u32) {
        use ffx_ps2::vu0::bridge as vu0;
        let nm = NORMAL_MATRIX.ptr(mem).addr;
        let cm = COLOR_MATRIX.ptr(mem).addr;
        let a = vu0::normal_light_matrix(mem, nm, dirs);
        let b = vu0::light_color_matrix(mem, cm, colors);
        (a, b)
    }

    /// `Sg_InitLight()` (`0x008360b0`): the colour matrix from the four
    /// colours, then the normal matrix from the three directions; returns
    /// `FFXVu0NormalLightMatrix`'s result.
    pub fn init_light(mem: &mut impl Mem) -> u32 {
        use ffx_ps2::vu0::bridge as vu0;
        let colors = [0, 1, 2, 3].map(|i| color(mem, i));
        let cm = COLOR_MATRIX.ptr(mem).addr;
        vu0::light_color_matrix(mem, cm, colors);
        let dirs = LIGHT_DIRS.map(|g| g.ptr(mem).addr);
        let nm = NORMAL_MATRIX.ptr(mem).addr;
        vu0::normal_light_matrix(mem, nm, dirs)
    }

    /// `Sg_LightSetChr(chr)` (`0x00836250`): the light matrices for a
    /// character: bits 0..=2 of its `i8` at `+0x188` take direction and
    /// colour `i` from the character (`+0x3c4 + 16 i`, `+0x384 + 16 i`)
    /// instead of the global ones, bit 3 its ambient (`+0x3b4`); returns
    /// `FFXVu0LightColorMatrix`'s result.
    pub fn set_chr(mem: &mut impl Mem, chr: u32) -> u32 {
        let flags = mem.read::<u8>(chr.wrapping_add(0x188));
        let pick =
            |own: bool, at: u32, global: u32| if own { chr.wrapping_add(at) } else { global };
        let dirs = [0u32, 1, 2].map(|i| {
            pick(
                flags & (1 << i) != 0,
                0x3c4 + 16 * i,
                LIGHT_DIRS[i as usize].ptr(mem).addr,
            )
        });
        let colors =
            [0u32, 1, 2, 3].map(|i| pick(flags & (1 << i) != 0, 0x384 + 16 * i, color(mem, i)));
        matrices(mem, dirs, colors).1
    }

    /// `Sg_AnimValF(v)` (`0x00836020`): a value easing to its target over
    /// a count of frames (`{f32 value, f32 target, i32 frames}`): unless
    /// the value equals the target, it moves `(target - value) / frames`
    /// (the target itself when no frames are left); the count then drops
    /// by one unless 0. `EAX` ends as the new count.
    pub fn anim_val_f(mem: &mut impl Mem, v: u32) -> u32 {
        let cur: f32 = mem.read(v);
        let target: f32 = mem.read(v.wrapping_add(4));
        // `fucompp`: equal skips; unordered moves.
        #[allow(clippy::float_cmp)] // the original's exact comparison
        let equal = cur == target;
        if !equal {
            let n: i32 = mem.read(v.wrapping_add(8));
            let next = if n == 0 {
                x87::round_trip(target)
            } else {
                let fnum = f32::from_bits(x87::store(f64::from(n)));
                Reg::load(target)
                    .sub_m32(cur)
                    .div_m32(fnum)
                    .add_m32(cur)
                    .store()
            };
            mem.write(v, next);
        }
        let n: u32 = mem.read(v.wrapping_add(8));
        if n == 0 {
            0
        } else {
            mem.write(v.wrapping_add(8), n - 1);
            n - 1
        }
    }

    /// `Sg_PLightDefine(i, x, y, z, radius, r, g, b, light)` (`0x008319b0`):
    /// a whole point light (position, colour, radius, light, the default
    /// attenuation, enabled) unless past the last one.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn define(
        mem: &mut impl Mem,
        i: i32,
        xyz: [f32; 3],
        radius: f32,
        rgb: [i32; 3],
        light: u32,
    ) {
        if i > PLIGHT_LAST.get(mem) {
            return;
        }
        set_pos(mem, i, 7, xyz.map(x87::round_trip));
        set_color(mem, i, rgb);
        set_radius(mem, i, x87::round_trip(radius));
        set_light(mem, i, light);
        let a = x87::round_trip(DEFAULT_ATTENUATION.get(mem));
        set_attenuation(mem, i, a);
        set_enable(mem, i, 1);
    }

    /// `FFXVu0LightColorMatrix(dst, c0, c1, c2, ambient)`.
    pub const LIGHT_COLOR_MATRIX: u32 = 0x006e_d4f0;
    /// `graphicCharacterLightingSetLightColor(kind, r, g, b)`.
    pub const SET_LIGHT_COLOUR: u32 = 0x0063_cb00;

    /// `Sg_LightSetColor(kind, rgba)` (`0x00836310`; from `Light_Set`,
    /// `opu_ffxlight` and others): for `kind` 0..=3 (unsigned) the four
    /// words at `rgba` into colour `kind` of [`LIGHT_COLORS`]; then the
    /// colour matrix ([`COLOR_MATRIX`]) from the four slots
    /// (`FFXVu0LightColorMatrix`) and `graphicCharacterLightingSetLightColor
    /// (kind, r, g, b)` with `rgba`'s first three floats (through x87),
    /// whose `eax` is returned.
    pub fn sg_light_set_color(h: &mut impl ffx_bridge::Host, kind: u32, rgba: u32) -> u32 {
        let slots = color(h, 0);
        if kind <= 3 {
            for i in 0..4 {
                let w: u32 = h.read(rgba.wrapping_add(4 * i));
                h.write(slots + 0x10 * kind + 4 * i, w);
            }
        }
        let (f, matrix) = (h.rebase(LIGHT_COLOR_MATRIX), h.rebase(0x023c_4400));
        h.call(
            f,
            &[matrix, slots, slots + 0x10, slots + 0x20, slots + 0x30],
        );
        let rgb = [8u32, 4, 0].map(|o| x87::store(x87::load(h.read(rgba.wrapping_add(o)))));
        let f = h.rebase(SET_LIGHT_COLOUR);
        h.call(f, &[kind, rgb[2], rgb[1], rgb[0]])
    }

    /// `Sg_LightSetColorRGB(kind, r, g, b)` (`0x00836400`, name ours, after
    /// `Sg_LightSetDirXYZ`; `Native_4012`'s): [`sg_light_set_color`] with
    /// `(r, g, b, 0)` from the stack (each through `fld`/`fstp`); its `eax`.
    pub fn sg_light_set_color_rgb(
        h: &mut impl ffx_bridge::Host,
        kind: u32,
        r: f32,
        g: f32,
        b: f32,
    ) -> u32 {
        use crate::scratch::{SCRATCH, Scratch};
        let mut s = Scratch::new(h);
        let v = [r, g, b].map(x87::round_trip);
        s.write(SCRATCH, [v[0], v[1], v[2], 0.0]);
        sg_light_set_color(&mut s, kind, SCRATCH)
    }

    /// `0x0084eb10(light, r, g, b, pitch, yaw)`: the debug log of
    /// `Light_Set`'s arguments.
    pub const LIGHT_SET_LOG: u32 = 0x0084_eb10;

    /// `Light_Set(light, r, g, b, pitch, yaw)` (`0x00836520`): the colour
    /// `(r, g, b) / 255` ([`sg_light_set_color`]) and the direction from
    /// the two angles in degrees ([`crate::control::bridge::light_set_dir`]):
    /// `(sin p cos y, sin y, cos p cos y)`, each angle `f32(deg * pi /
    /// 180)` through `Angle_Wrap`, `sin` and `cos` the CRT's (`trig`,
    /// `f32` bits in and out). Every integer goes through `f32` first
    /// (`fild`, `fstp dword`). Both vectors live in one stack local whose
    /// fourth word the original never writes (0 here). With
    /// `g_FFXIsDebug` set, [`LIGHT_SET_LOG`] gets the arguments; `eax` is the
    /// last call's.
    #[allow(clippy::similar_names)] // the two angles' sines and cosines
    pub fn light_set(
        h: &mut impl ffx_bridge::Host,
        trig: &dyn Fn(ffx_battle::pos::bridge::Trig, u32) -> u32,
        light: u32,
        rgb: [i32; 3],
        pitch: i32,
        yaw: i32,
    ) -> u32 {
        use crate::player::bridge::{DEG_180, DEG_PI};
        use crate::scratch::{SCRATCH, Scratch};
        use ffx_battle::pos::bridge::Trig;
        let int = |v: i32| f32::from_bits(x87::store(f64::from(v)));
        let scale = Reg::constant(COLOR_SCALE.get(h));
        let pi = Reg::constant(DEG_PI.get(h));
        let half_turn = Reg::constant(DEG_180.get(h));
        let radians = |deg: i32| (Reg::load(int(deg)) * pi / half_turn).store();
        let ret = {
            let mut s = Scratch::new(h);
            let color = rgb.map(|c| (Reg::load(int(c)) / scale).store());
            s.write(SCRATCH, [color[0], color[1], color[2]]);
            sg_light_set_color(&mut s, light, SCRATCH);
            let w_yaw = crate::vec::bridge::angle_wrap(radians(yaw));
            let sin_yaw = trig(Trig::Sin, w_yaw.to_bits());
            let cos_yaw = f32::from_bits(trig(Trig::Cos, w_yaw.to_bits()));
            let w_pitch = crate::vec::bridge::angle_wrap(radians(pitch));
            let sin_pitch = f32::from_bits(trig(Trig::Sin, w_pitch.to_bits()));
            let x = Reg::load(sin_pitch).mul_m32(cos_yaw).store();
            let cos_pitch = f32::from_bits(trig(Trig::Cos, w_pitch.to_bits()));
            let z = Reg::load(cos_pitch).mul_m32(cos_yaw).store();
            let y = x87::round_trip(f32::from_bits(sin_yaw));
            s.write(SCRATCH, [x, y, z]);
            crate::control::bridge::light_set_dir(&mut s, light, SCRATCH)
        };
        if crate::control::bridge::IS_DEBUG.get(h) == 0 {
            return ret;
        }
        let f = h.rebase(LIGHT_SET_LOG);
        let [r, g, b] = rgb.map(i32::cast_unsigned);
        h.call(
            f,
            &[light, r, g, b, pitch.cast_unsigned(), yaw.cast_unsigned()],
        )
    }
}
