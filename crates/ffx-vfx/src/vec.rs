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

    /// `pppVu0Length_copy_0072ab20(v)` (`0x0072ab20`): `|v.xyz|` through
    /// the emulated VU0 registers: the squares into `0x00c8f830`.., the
    /// running sums into `0x00c8f820`.., `_CIsqrt` of their `fabs` (exact
    /// in 53 bits, then `f32`) into `0x00c8f788`, plus `vf00.x` into
    /// `0x00c8f830`. Returns it in `ST0`.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_vu0_length(mem: &mut impl Mem, v: u32) -> f32 {
        use crate::x87::{ADD, MUL, Reg};
        let reg = |a: u32| ffx_bridge::Global::<u32>::new(a);
        let (sq, acc, q) = (
            reg(0x00c8_f830).ptr(mem).addr,
            reg(0x00c8_f820).ptr(mem).addr,
            reg(0x00c8_f788).ptr(mem).addr,
        );
        let vv: [u32; 4] = mem.read(v);
        mem.write(sq, vv);
        let c: [f32; 4] = mem.read(v);
        for i in 0..3u32 {
            let x: f32 = mem.read(sq + 4 * i);
            let p = Reg::load_f32(x).mem(MUL, c[i as usize]).store();
            mem.write(sq + 4 * i, p);
        }
        let s0: f32 = mem.read(sq);
        mem.write(acc, crate::x87::load_f32(s0));
        let s1 = Reg::load_f32(s0).mem(ADD, mem.read(sq + 4)).store();
        mem.write(acc + 4, s1);
        let s2 = Reg::load_f32(mem.read(sq + 8)).mem(ADD, s1).store();
        mem.write(acc + 8, s2);
        let a = f32::from_bits(s2.to_bits() & 0x7fff_ffff);
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let root = f64::from(crate::x87::load_f32(a)).sqrt() as f32;
        mem.write(q, root);
        let vx: f32 = mem.read(crate::prog::bridge::VF00.ptr(mem).addr);
        let out = Reg::load_f32(vx).mem(ADD, root).store();
        mem.write(sq, out);
        out
    }

    /// `Vec4SubScalars_0074be90(dst, src, x, y, z, w)` (`0x0074be90`):
    /// `dst` = `src` - (`x`, `y`, `z`, `w`), then `restoreVf00Register`.
    pub fn vec4_sub_scalars(mem: &mut impl Mem, dst: u32, src: u32, k: [f32; 4]) {
        for (i, c) in (0u32..).zip(k) {
            let v = sub(mem.read(src + 4 * i), c);
            mem.write(dst + 4 * i, v);
        }
        restore_vf00(mem);
    }

    /// `op_rot_point_007ea9d0(x0, y0, ang, x1, y1, ox, oy)` (`0x007ea9d0`):
    /// the point (`x1`, `y1`) turned by `-ang` about (`x0`, `y0`) into
    /// `*ox`, `*oy`: `c`, `s` the CRT's `_CIcos`/`_CIsin` of `-ang` (through
    /// `trig`, `true` for the cosine; each stored as `f32`), `dx`, `dy` the
    /// offsets as `f32`, then `x0 + (c dx + s dy)` and `(c dy - s dx) + y0`
    /// in 53 bits. Returns `oy` (left in `eax`).
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn op_rot_point<M: Mem>(
        mem: &mut M,
        x0: f32,
        y0: f32,
        ang: f32,
        p1: [f32; 2],
        out: [u32; 2],
        trig: &mut impl FnMut(&mut M, bool, f32) -> f32,
    ) -> u32 {
        use crate::x87::{ADD, MUL, Reg, SUB};
        let a = f32::from_bits(crate::x87::load(ang.to_bits()) ^ 0x8000_0000);
        let c = trig(mem, true, a);
        let s = trig(mem, false, a);
        let dx = Reg::load_f32(p1[0]).reg(SUB, Reg::load_f32(x0)).store();
        let dy = Reg::load_f32(p1[1]).mem(SUB, y0).store();
        let (cr, sr) = (Reg::load_f32(c), Reg::load_f32(s));
        let sum = cr.mem(MUL, dx).reg(ADD, sr.mem(MUL, dy));
        let x = Reg::load_f32(x0).reg(ADD, sum).store();
        mem.write(out[0], x);
        let d = cr.mem(MUL, dy).reg(SUB, sr.mem(MUL, dx));
        let y = d.mem(ADD, y0).store();
        mem.write(out[1], y);
        out[1]
    }
}
