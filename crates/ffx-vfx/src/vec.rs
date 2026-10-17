//! Small vector helpers of the effect code that work on `f32` triples and
//! pairs, some with the emulated VU0's status flags.

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_ps2::ee::bridge::{restore_vf00, set_ss_flag, set_zs_flag};

    use crate::x87::{add, sub};

    /// `Vec3AddScalars_00735100(dst, src, x, y, z, w)` (`0x00735100`):
    /// `dst.xyz` = `src.xyz` + (`x`, `y`, `z`) (`w` unused); returns `src`
    /// (left in `eax`).
    pub fn vec3_add_scalars(mem: &mut impl Mem, dst: u32, src: u32, k: [f32; 3]) -> u32 {
        for (i, c) in (0u32..).zip(k) {
            let v = add(mem.read(src + 4 * i), c);
            mem.write(dst + 4 * i, v);
        }
        src
    }

    /// `Vec3SubScalars_0074b670(dst, src, x, y, z)` (`0x0074b670`):
    /// `dst.xyz` = `src.xyz` - (`x`, `y`, `z`), then `restoreVf00Register`.
    pub fn vec3_sub_scalars(mem: &mut impl Mem, dst: u32, src: u32, k: [f32; 3]) {
        for (i, c) in (0u32..).zip(k) {
            let v = sub(mem.read(src + 4 * i), c);
            mem.write(dst + 4 * i, v);
        }
        restore_vf00(mem);
    }

    /// `Vec2SubScalarsFlags_0075e870(dst, src, x, y)` (`0x0075e870`):
    /// `dst.xy` = `src.xy` - (`x`, `y`); VU0's zero flag (`setZSFlag`) when
    /// either is 0 and its sign flag (`setSSFlag`) when either is
    /// negative, then `restoreVf00Register`.
    pub fn vec2_sub_scalars_flags(mem: &mut impl Mem, dst: u32, src: u32, x: f32, y: f32) {
        let a = sub(mem.read(src), x);
        mem.write(dst, a);
        let b = sub(mem.read(src + 4), y);
        mem.write(dst + 4, b);
        let (a, b): (f32, f32) = (mem.read(dst), mem.read(dst + 4));
        if a == 0.0 || b == 0.0 {
            set_zs_flag(mem);
        }
        if a < 0.0 || b < 0.0 {
            set_ss_flag(mem);
        }
        restore_vf00(mem);
    }

    /// `Vec4Max_00808b70(dst, src, x, y, z, w)` (`0x00808b70`): each
    /// component of `dst` is `src`'s when the argument is below it, else
    /// the argument (so a NaN on either side gives the argument).
    pub fn vec4_max(mem: &mut impl Mem, dst: u32, src: u32, k: [u32; 4]) {
        for (i, a) in (0u32..).zip(k) {
            let s: u32 = mem.read(src + 4 * i);
            let below = f32::from_bits(a) < f32::from_bits(s);
            mem.write(dst + 4 * i, crate::x87::load(if below { s } else { a }));
        }
    }

    /// `op_ivec4_to_fvec4_007e3100(dst, src)` (`0x007e3100`): the four
    /// `i32` of `src` (read first) as `f32` into `dst`.
    pub fn ivec4_to_fvec4(mem: &mut impl Mem, dst: u32, src: u32) {
        let v: [i32; 4] = mem.read(src);
        for (i, x) in (0u32..).zip(v) {
            #[allow(clippy::cast_precision_loss)] // fild, fstp m32
            mem.write(dst + 4 * i, x as f32);
        }
    }

    /// `Vec4AddScalars_0074bc50(dst, src, x, y, z, w)` (`0x0074bc50`):
    /// `dst` = `src` + (`x`, `y`, `z`, `w`).
    pub fn vec4_add_scalars(mem: &mut impl Mem, dst: u32, src: u32, k: [f32; 4]) {
        for (i, c) in (0u32..).zip(k) {
            let v = add(mem.read(src + 4 * i), c);
            mem.write(dst + 4 * i, v);
        }
    }
}
