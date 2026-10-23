//! The op/opus layer's one-line helpers: word and vector copies, 64-bit
//! masks, float steps, the MSVC container accessors the opus trees use,
//! and similar leaves.

/// `hashU`'s hash of a string's bytes `c` (each sign-extended):
/// `h = (h * a + c) % 10007`, the multiplier `a` stepping from 31415 by
/// `a = a * 27183 % 10006` (32-bit, C remainders); made non-negative at
/// the end.
#[must_use]
pub fn hash_u(bytes: impl IntoIterator<Item = u8>) -> u32 {
    let (mut h, mut a) = (0i32, 0x7ab7i32);
    for c in bytes {
        h = h
            .wrapping_mul(a)
            .wrapping_add(i32::from(c.cast_signed()))
            .wrapping_rem(10007);
        a = a.wrapping_mul(0x6af5).wrapping_rem(10006);
    }
    (if h < 0 { h + 10007 } else { h }).cast_unsigned()
}

/// `pppViewOutCode(x, y)` (`0x00728dd0`): where a projected point lies
/// against the view rectangle: 1 left of `-8/9` (the `f32` value, as a
/// double), 2 from `8/9` right, 4 below `-0.5`, 8 from `0.5` up. A NaN
/// sets no bit.
#[must_use]
pub fn ppp_view_out_code(x: f32, y: f32) -> u32 {
    const EDGE_X: f64 = 0.888_888_895_511_627_2;
    let (x, y) = (f64::from(x), f64::from(y));
    let mut code = if x < -EDGE_X {
        1
    } else if x >= EDGE_X {
        2
    } else {
        0
    };
    if y < -0.5 {
        code |= 4;
    } else if y >= 0.5 {
        code |= 8;
    }
    code
}

/// The core of `pppCheckViewCylinder` (`0x00728f10`) on `point`
/// already in view space (`x`, `y`, `z`): 0 when `far` is below 0 (the
/// view test is off), 0xf when the point lies beyond `far` or the
/// cylinder's near side (`z + radius`) is not in front of the eye; else
/// the [`ppp_view_out_code`]s of the four corners of the cylinder's
/// box projected at `z + radius` (`x ± radius`, `y ± height`, over that
/// depth) combined with `&`: nonzero when the whole box is off one side. Each value
/// is rounded to `f32` where the original stores it; a NaN passes every
/// test that would end early (unordered compares fall through).
#[must_use]
pub fn check_view_cylinder(point: [f32; 3], far: f32, radius: f32, height: f32) -> u32 {
    use crate::x87::{ADD, MUL, Reg, SUB};
    const DIV: crate::x87::Op = |a, b| a / b;
    let [x, y, z] = point;
    if f64::from(far) < 0.0 {
        return 0;
    }
    if f64::from(far) < f64::from(z) {
        return 0xf;
    }
    let rad = Reg::load_f32(radius);
    let near = Reg::load_f32(z).reg(ADD, rad).store();
    if f64::from(near) <= 0.0 {
        return 0xf;
    }
    let inv = Reg::Num(1.0).reg(DIV, Reg::load_f32(near)).store();
    let inv_r = Reg::load_f32(inv);
    let sx = Reg::load_f32(x).reg(MUL, inv_r).store();
    let sy = Reg::load_f32(y).reg(MUL, inv_r).store();
    let sr = rad.reg(MUL, inv_r).store();
    let sh = inv_r.mem(MUL, height).store();
    let (sx, sy, sr, sh) = (
        Reg::load_f32(sx),
        Reg::load_f32(sy),
        Reg::load_f32(sr),
        Reg::load_f32(sh),
    );
    let (y0, x0) = (sy.reg(SUB, sh).store(), sx.reg(SUB, sr).store());
    let (x1, y1) = (sx.reg(ADD, sr).store(), sy.reg(ADD, sh).store());
    ppp_view_out_code(x1, y1)
        & ppp_view_out_code(x0, y1)
        & ppp_view_out_code(x1, y0)
        & ppp_view_out_code(x0, y0)
}

/// The core of the effect programs' 2D angle (`0x007590e0`, `0x0075e1f0`):
/// the angle of `(x, y)` in -pi..pi, `None` for (0, 0). With `x * y`
/// (as `f32`) above 0 it is `atan(y / x)` (the quotient unrounded, the
/// angle stored as `f32`) plus 0 (`x > 0`) or `f32(-pi)`; below 0,
/// `f32(+-pi / 2)` (the sign of `y`) minus `atan(x / y)`; else, with
/// `x` not 0 (or a NaN), 0 or `pi` by the sign of `x`, and with `x` 0,
/// `+-pi / 2` by the sign of `y`. The constants are the original's
/// doubles (3.141592654, 1.570796327); NaNs take the "not above 0"
/// side of each test. `atan` is the C runtime's `_CIatan` on the x87
/// value.
pub fn angle_xy(x: f32, y: f32, atan: &mut impl FnMut(f64) -> f64) -> Option<f32> {
    use crate::x87::{MUL, Reg};
    // the original's doubles (0x00b51548, 0x00b51670), not the exact ones
    #[allow(clippy::approx_constant)]
    const PI: f64 = 3.141_592_654;
    #[allow(clippy::approx_constant)]
    const HALF_PI: f64 = 1.570_796_327;
    const DIV: crate::x87::Op = |a, b| a / b;
    // the register as the f64 the shim loads (a NaN keeps its payload)
    let reg = |r: Reg| match r {
        Reg::Num(v) => v,
        Reg::Nan(b) => f64::from(f32::from_bits(b)),
    };
    #[allow(clippy::cast_possible_truncation)] // fstp m32
    let to_f32 = |v: f64| v as f32;
    let prod = f64::from(Reg::load_f32(x).mem(MUL, y).store());
    let (fx, fy) = (f64::from(x), f64::from(y));
    if prod > 0.0 {
        let off = to_f32(if 0.0 < fx { 0.0 } else { -PI });
        let r = to_f32(atan(reg(Reg::load_f32(y).mem(DIV, x))));
        return Some(Reg::load_f32(r).mem(crate::x87::ADD, off).store());
    }
    if prod < 0.0 {
        let off = to_f32(if 0.0 < fy { HALF_PI } else { -HALF_PI });
        let r = to_f32(atan(reg(Reg::load_f32(x).mem(DIV, y))));
        let d = Reg::Num(off.into()).reg(crate::x87::SUB, Reg::load_f32(r));
        return Some(d.store());
    }
    // (a NaN is not 0 either)
    let angle = if fx != 0.0 {
        if 0.0 < fx { 0.0 } else { PI }
    } else if fy == 0.0 {
        return None;
    } else if 0.0 < fy {
        HALF_PI
    } else {
        -HALF_PI
    };
    Some(to_f32(angle))
}

/// The four-texel average of [`bridge::op_texels_box`] (`row_a[2i]`,
/// `row_b[2i]`, `row_a[2i + 1]`, `row_b[2i + 1]`): per byte of the RGBA
/// words, `(a + b + c + d + 2) >> 2`, carried in two halves (bytes 0
/// and 2 low, 1 and 3 through `>> 2` of each word) as the original
/// does, so no carry crosses a byte.
#[must_use]
pub fn box4(texels: [u32; 4]) -> u32 {
    // the original adds row b's right texel and the rounding first, then
    // row a's left, row a's right and row b's left
    let [a_left, b_left, a_right, b_right] = texels;
    let sum = |part: fn(u32) -> u32, round: u32| {
        part(b_right)
            .wrapping_add(round)
            .wrapping_add(part(a_left))
            .wrapping_add(part(a_right))
            .wrapping_add(part(b_left))
    };
    let high = sum(|x| x >> 2 & 0xffc0_3fc0, 0xff80_0080);
    let low = sum(|x| x & 0x00ff_00ff, 0x0002_0002) >> 2;
    let top = sum(|x| x >> 2 & 0x3fc0_3fc0, 0x0080_0080);
    (high ^ low) & 0x00ff_00ff ^ top
}

/// The helpers over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::x87;

    fn rf(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
    }

    /// `pppCheckViewCylinder(m, v, far, radius, height)` (`0x00728f10`):
    /// [`super::check_view_cylinder`] on `v` (a vector) times the matrix
    /// `m` (`FFXVu0ApplyMatrix`, computed in place: the original's copy
    /// lands in its own frame), when `far` is not below 0.
    pub fn ppp_check_view_cylinder(
        mem: &impl Mem,
        m: u32,
        v: u32,
        far: f32,
        radius: f32,
        height: f32,
    ) -> u32 {
        if f64::from(far) < 0.0 {
            return 0;
        }
        let x: [f32; 4] = mem.read(v);
        let p: [f32; 3] = std::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation)] // 0..3
            let i = i as u32;
            let col = [0, 4, 8, 12].map(|row| rf(mem, m.wrapping_add(4 * (row + i))));
            ffx_system::math::mul4x4_component(i, x, col)
        });
        super::check_view_cylinder(p, far, radius, height)
    }

    /// The effect programs' 2D angle (`0x007590e0`, and the copy
    /// `0x0075e1f0`) of the vector at `v` (`x`, `y`) into `*out`:
    /// [`super::angle_xy`]; returns 1, or 0 (nothing written) for (0, 0).
    pub fn ppp_angle_xy<M: Mem>(
        mem: &mut M,
        v: u32,
        out: u32,
        atan: &mut impl FnMut(&mut M, f64) -> f64,
    ) -> u32 {
        let (x, y) = (rf(mem, v), rf(mem, v.wrapping_add(4)));
        let Some(a) = super::angle_xy(x, y, &mut |t| atan(mem, t)) else {
            return 0;
        };
        mem.write(out, a);
        1
    }

    /// `hashU(s)` (`0x00716570`): [`super::hash_u`] of the NUL-terminated
    /// string at `s`.
    pub fn hash_u_at(mem: &impl Mem, s: u32) -> u32 {
        let bytes = (0..)
            .map(|i| mem.read::<u8>(s.wrapping_add(i)))
            .take_while(|&c| c != 0);
        super::hash_u(bytes)
    }

    /// `op_char_table_set(table, chars, v)` (`0x007e13e0`): `table[c] = v`
    /// for each (signed) byte `c` of the string `chars`.
    pub fn op_char_table_set(mem: &mut impl Mem, table: u32, chars: u32, v: u32) {
        let mut p = chars;
        loop {
            let c: i8 = mem.read(p);
            if c == 0 {
                return;
            }
            mem.write(
                table.wrapping_add(i32::from(c).cast_unsigned()),
                v.to_le_bytes()[0],
            );
            p = p.wrapping_add(1);
        }
    }

    /// `op_str_unquote(s)` (`0x007e1b50`): the string with its `'` and `"`
    /// taken out, in place.
    pub fn op_str_unquote(mem: &mut impl Mem, s: u32) {
        let (mut src, mut dst) = (s, s);
        loop {
            let c: u8 = mem.read(src);
            if c != b'\'' && c != b'"' {
                if dst != src {
                    mem.write(dst, c);
                }
                dst = dst.wrapping_add(1);
            }
            src = src.wrapping_add(1);
            if c == 0 {
                return;
            }
        }
    }

    /// `op_and64(dst, a, b)` (`0x007e3040`) and `op_or64` (`0x007e6c80`,
    /// `or` set).
    pub fn op_logic64(mem: &mut impl Mem, dst: u32, a: u32, b: u32, or: bool) {
        let (x, y): (u64, u64) = (mem.read(a), mem.read(b));
        mem.write(dst, if or { x | y } else { x & y });
    }

    /// `op_or16_64(dst, src, v)` (`0x007e30a0`): the 64-bit `src` with the
    /// `u16` `v` or'd into its low word.
    pub fn op_or16_64(mem: &mut impl Mem, dst: u32, src: u32, v: u32) {
        let x: u64 = mem.read(src);
        mem.write(dst, x | u64::from(v & 0xffff));
    }

    /// `op_mask16_64(dst, a, b)` (`0x007eca00`): `(a & b) & 0xffff` as a
    /// 64-bit value.
    pub fn op_mask16_64(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        mem.write(dst, u64::from(a & b & 0xffff));
    }

    /// The float steps: `*dst = *v op s` (`op_float_scale` `0x007e3070` and
    /// `0x00802080`, `op_float_add` `0x007e5950`, `op_float_sub`
    /// `0x007e59a0`) at offset `at` (`op_vec_w_sub` `0x007e31c0`, `w`
    /// minus; `op_vec_z_scale` `0x007ecea0`, `z` times; `op_vec_w_add2`
    /// `0x007ecc00`, `w` plus).
    pub fn op_float_step(mem: &mut impl Mem, dst: u32, v: u32, s: f32, at: u32, op: char) {
        let x = rf(mem, v.wrapping_add(at));
        let r = match op {
            '*' => x87::mul(x, s),
            '+' => x87::add(x, s),
            _ => x87::sub(x, s),
        };
        mem.write(dst.wrapping_add(at), r);
    }

    /// `op_vec_xz_add(dst, v, x, _, z)` (`0x007e6ca0`): `dst.x = v.x + x`,
    /// `dst.z = v.z + z`.
    pub fn op_vec_xz_add(mem: &mut impl Mem, dst: u32, v: u32, x: f32, z: f32) {
        let sx = x87::add(rf(mem, v), x);
        mem.write(dst, sx);
        let sz = x87::add(rf(mem, v.wrapping_add(8)), z);
        mem.write(dst.wrapping_add(8), sz);
    }

    /// `op_word_copy_at(dst, src)` (`0x007ece80`): the word at `+8`.
    pub fn op_word_copy_at(mem: &mut impl Mem, dst: u32, src: u32, at: u32) {
        let w: u32 = mem.read(src.wrapping_add(at));
        mem.write(dst.wrapping_add(at), w);
    }

    /// `op_vec4_move(dst, src)` (`0x007e3f60`): four words.
    pub fn op_vec4_move(mem: &mut impl Mem, dst: u32, src: u32) {
        let v: [u32; 4] = mem.read(src);
        mem.write(dst, v);
    }

    /// `op_vec_zero(dst)` (`0x007e5660`).
    pub fn op_vec_zero(mem: &mut impl Mem, dst: u32) {
        mem.write(dst, [0u32; 4]);
    }

    /// `op_vec_splat(dst, v)` (`0x007e37f0`): (`v`, `v`, `v`, 1.0).
    pub fn op_vec_splat(mem: &mut impl Mem, dst: u32, v: f32) {
        let v = x87::load_f32(v);
        mem.write(dst, [v, v, v, 1.0]);
    }

    /// `op_unit_m(dst)` (`0x007ea6d0`): the identity matrix (zeroed by
    /// `memset`, then the diagonal 1.0).
    pub fn op_unit_m(mem: &mut impl Mem, dst: u32) {
        mem.write(dst, [0u32; 16]);
        for i in [15u32, 10, 5, 0] {
            mem.write(dst.wrapping_add(4 * i), 1.0f32);
        }
    }

    /// `op_fabs(x)` (`0x007e3360`): `-x` when `x < 0`, else `x` (through
    /// the FPU).
    #[must_use]
    pub fn op_fabs(x: f32) -> f32 {
        let x = x87::load_f32(x);
        if x < 0.0 { -x } else { x }
    }

    /// `op_pow2(n)` (`0x007e31e0`): 2 to the `n` (1 for `n <= 0`, 32-bit).
    #[must_use]
    pub fn op_pow2(n: u32) -> u32 {
        let n = n.cast_signed();
        if n <= 0 {
            1
        } else {
            // doubled n times: 0 from 32 on
            1u32.checked_shl(n.cast_unsigned()).unwrap_or(0)
        }
    }

    /// `op_log2_ceil(n)` (`0x007eaa90`): the smallest `k` with `2^k >= n`
    /// (0 for `n <= 1`).
    #[must_use]
    pub fn op_log2_ceil(n: u32) -> u32 {
        let n = n.cast_signed();
        let (mut k, mut p) = (0u32, 1i32);
        while p < n {
            p = p.wrapping_mul(2);
            k += 1;
        }
        k
    }

    /// `op_table2_get(t, i, j)` (`0x007e37b0`): word `j` of the table at
    /// `t` + the `i`th word of `t`.
    pub fn op_table2_get(mem: &impl Mem, t: u32, i: u32, j: u32) -> u32 {
        let row: u32 = mem.read(t.wrapping_add(i.wrapping_mul(4)));
        mem.read(row.wrapping_add(j.wrapping_mul(4)).wrapping_add(t))
    }

    /// `op_oef2_tex_pos_replace(rec, pos)` (`0x007e4fc0`): the low 14 bits
    /// of a table-2 record become `pos`'s, byte `+0x1f` 1; returns the old
    /// low 14 bits.
    pub fn op_oef2_tex_pos_replace(mem: &mut impl Mem, rec: u32, pos: u32) -> u32 {
        let old: u32 = mem.read(rec);
        mem.write(rec, old ^ ((pos ^ old) & 0x3fff));
        mem.write(rec.wrapping_add(0x1f), 1u8);
        old & 0x3fff
    }

    /// `op_color_shift_store_word`/`_half` (`0x007e30c0` word,
    /// `0x007eaa60` half) and `op_color_shift_load_word` (`0x007ecac0`):
    /// through `base + g_pppColorConShift`, the shift cleared.
    pub fn op_color_shift(mem: &mut impl Mem, p: u32, base: u32, half: bool, load: bool) {
        let shift = crate::ppp::bridge::COLOR_CON_SHIFT;
        let at = base.wrapping_add(shift.get(mem));
        let (src, dst) = if load { (at, p) } else { (p, at) };
        if half {
            let v: u16 = mem.read(src);
            mem.write(dst, v);
        } else {
            let v: u32 = mem.read(src);
            mem.write(dst, v);
        }
        shift.set(mem, 0);
    }

    /// `op_spr_cmd_size(cmd)` (`0x007f5ed0`): `((i16) cmd[+2] * (14 if bit
    /// 3 of cmd[0], else 7) + 7) * 16`.
    pub fn op_spr_cmd_size(mem: &impl Mem, cmd: u32) -> u32 {
        let n = i32::from(mem.read::<i16>(cmd.wrapping_add(2)));
        let k = if mem.read::<u8>(cmd) & 8 != 0 { 14 } else { 7 };
        (n.wrapping_mul(k).wrapping_add(7) << 4).cast_unsigned()
    }

    /// `op_is_not(x, g)`: -1 when `x` is not the word at `g`, else 0
    /// (`0x007fe430` with `0x0119fe90`, `0x007fe4c0` with
    /// `g_OpusEncount`).
    pub fn op_is_not(mem: &impl Mem, x: u32, g: u32) -> u32 {
        let v: u32 = mem.read(Global::<u32>::new(g).ptr(mem).addr);
        if x == v { 0 } else { u32::MAX }
    }

    /// `opus_draw_slot_clear(obj, v)` (`0x007fe690`): the 16 words at
    /// `obj + 0x60` equal to `v` zeroed.
    pub fn opus_draw_slot_clear(mem: &mut impl Mem, obj: u32, v: u32) {
        for i in 0..16 {
            let at = obj.wrapping_add(0x60 + 4 * i);
            if mem.read::<u32>(at) == v {
                mem.write(at, 0u32);
            }
        }
    }

    /// `op_et_eff_index(id)` (`0x007fed60`): the first `g_EtEff` record
    /// with this ID, or -1.
    pub fn op_et_eff_index(mem: &impl Mem, id: u32) -> i32 {
        let e = crate::et::bridge::EFF.ptr(mem).addr;
        (0..crate::et::bridge::EFFS)
            .find(|&i| mem.read::<u32>(e + 0x20 * i) == id)
            .map_or(-1, u32::cast_signed)
    }

    /// `op_oef2_sep_number_get(cue)` (`0x007fff80`): the separate sound ID
    /// of a cue's record (`+8` points at it, the ID at `+8`).
    pub fn op_oef2_sep_number_get(mem: &impl Mem, cue: u32) -> u32 {
        let rec = ffx_system::stub::ptr32(mem.read(cue.wrapping_add(8)));
        mem.read(rec.wrapping_add(8))
    }

    /// `opus_part_kind(p)` (`0x00800410`): the `i16` at `+2` of what `p
    /// +0xc` points at.
    pub fn opus_part_kind(mem: &impl Mem, p: u32) -> u32 {
        let q: u32 = mem.read(p.wrapping_add(0xc));
        i32::from(mem.read::<i16>(q.wrapping_add(2))).cast_unsigned()
    }

    /// The MSVC container helpers: `*dst = *src` when `dst` is not null
    /// (`0x00806490` and others).
    pub fn copy_if(mem: &mut impl Mem, dst: u32, src: u32) {
        if dst != 0 {
            let v: u32 = mem.read(src);
            mem.write(dst, v);
        }
    }

    /// `*this = v`; returns `this` (`0x00806730` and others).
    pub fn set_this(mem: &mut impl Mem, this: u32, v: u32) -> u32 {
        mem.write(this, v);
        this
    }

    /// `*this == *other` (`0x00806970`, `0x00806990`) or `!=`
    /// (`0x008069b0`, `0x008069d0`) as 1/0.
    pub fn iter_eq(mem: &impl Mem, this: u32, other: u32, ne: bool) -> u32 {
        let eq = mem.read::<u32>(this) == mem.read::<u32>(other);
        u32::from(eq != ne)
    }

    /// `0x00a39f00(n)`: a node whose state (top nibble of `+0x30`) is 2
    /// leaves its list (`+0x1c` previous, `+0x20` next) and goes in at
    /// the front of its owner's list (the head `[n +0x18] +0x48`), in
    /// state 1. Returns the state word as left in `eax`: the new word, or the
    /// top nibble alone when nothing moved.
    pub fn op_node_requeue(mem: &mut impl Mem, n: u32) -> u32 {
        let at = |o: u32| n.wrapping_add(o);
        let f: u32 = mem.read(at(0x30));
        if f & 0xf000_0000 != 0x2000_0000 {
            return f & 0xf000_0000;
        }
        node_to_owner(mem, n)
    }

    /// `n` out of its list and in at the front of its owner's list (the
    /// head `[n +0x18] +0x48`), in state 1; returns the new state word.
    fn node_to_owner(mem: &mut impl Mem, n: u32) -> u32 {
        let at = |o: u32| n.wrapping_add(o);
        let next: u32 = mem.read(at(0x20));
        let prev: u32 = mem.read(at(0x1c));
        let owner: u32 = mem.read(at(0x18));
        mem.write(next.wrapping_add(0x1c), prev);
        let prev: u32 = mem.read(at(0x1c));
        let next: u32 = mem.read(at(0x20));
        mem.write(prev.wrapping_add(0x20), next);
        let head = owner.wrapping_add(0x48);
        mem.write(at(0x1c), head);
        let first: u32 = mem.read(head.wrapping_add(0x20));
        mem.write(at(0x20), first);
        mem.write(first.wrapping_add(0x1c), n);
        let h: u32 = mem.read(at(0x1c));
        mem.write(h.wrapping_add(0x20), n);
        let f: u32 = mem.read(at(0x30));
        let v = (f & 0x0fff_ffff) | 0x1000_0000;
        mem.write(at(0x30), v);
        v
    }

    /// `op_node_requeue_all_00a39f60(set)` (`0x00a39f60`): every node of
    /// the set's list 2 (the head at `set + 0x80`, first node at `+0x1c`)
    /// moves to the front of its owner's list in state 1, as
    /// [`op_node_requeue`] moves one. Returns the head, as left in `eax`.
    pub fn op_node_requeue_all(mem: &mut impl Mem, set: u32) -> u32 {
        let head = set.wrapping_add(0x80);
        loop {
            let n: u32 = mem.read(head.wrapping_add(0x1c));
            if n == head {
                return head;
            }
            node_to_owner(mem, n);
        }
    }

    /// `op_texels_halve_00a385f0(dst, src, n, size)` (`0x00a385f0`): `n`
    /// pairs of adjacent texels averaged into one, rounding up: `size` 4
    /// per byte of each RGBA word (`(a | b) - ((a ^ b) >> 1 & 0x7f7f7f7f)`),
    /// `size` 1 single bytes; other sizes do nothing. Returns `eax` as
    /// left: the end of `src` (size 4), the last average (size 1, or 1
    /// for none), else `size`.
    pub fn op_texels_halve(mem: &mut impl Mem, dst: u32, src: u32, n: u32, size: u32) -> u32 {
        match size {
            4 => {
                for i in 0..n {
                    let s = src.wrapping_add(8 * i);
                    let (a, b): (u32, u32) = (mem.read(s), mem.read(s.wrapping_add(4)));
                    mem.write(
                        dst.wrapping_add(4 * i),
                        (a | b) - ((a ^ b) >> 1 & 0x7f7f_7f7f),
                    );
                }
                src.wrapping_add(n.wrapping_mul(8))
            }
            1 => {
                let mut last = 1;
                for i in 0..n {
                    let s = src.wrapping_add(2 * i);
                    let (a, b) = (mem.read::<u8>(s), mem.read::<u8>(s.wrapping_add(1)));
                    last = (u32::from(a) + u32::from(b) + 1) >> 1;
                    #[allow(clippy::cast_possible_truncation)] // at most 255
                    mem.write(dst.wrapping_add(i), last as u8);
                }
                last
            }
            _ => size,
        }
    }

    /// `op_texels_box_00a38670(dst, row_a, row_b, n, size)` (`0x00a38670`):
    /// a mip level from two rows: texel `i` is the 2x2 average of `row_a`
    /// and `row_b`'s texels `2i`, `2i + 1`, rounding to nearest: `size` 4
    /// per byte of each RGBA word ([`super::box4`]), `size` 1 single bytes
    /// (`(sum + 2) >> 2`); other sizes do nothing. Returns `eax` as left:
    /// `row_a - row_b` (size 4), the last average (size 1), 0 for `n` 0,
    /// else `size`.
    pub fn op_texels_box(
        mem: &mut impl Mem,
        dst: u32,
        row_a: u32,
        row_b: u32,
        n: u32,
        size: u32,
    ) -> u32 {
        if size != 4 && size != 1 {
            return size;
        }
        if n == 0 {
            return 0;
        }
        if size == 4 {
            for i in 0..n {
                let (a, b) = (row_a.wrapping_add(8 * i), row_b.wrapping_add(8 * i));
                let v = super::box4([
                    mem.read(a),
                    mem.read(b),
                    mem.read(a.wrapping_add(4)),
                    mem.read(b.wrapping_add(4)),
                ]);
                mem.write(dst.wrapping_add(4 * i), v);
            }
            return row_a.wrapping_sub(row_b);
        }
        let mut last = 0;
        for i in 0..n {
            let (a, b) = (row_a.wrapping_add(2 * i), row_b.wrapping_add(2 * i));
            let sum = [a, a.wrapping_add(1), b, b.wrapping_add(1)]
                .into_iter()
                .map(|x| u32::from(mem.read::<u8>(x)))
                .sum::<u32>();
            last = (sum + 2) >> 2;
            #[allow(clippy::cast_possible_truncation)] // at most 255
            mem.write(dst.wrapping_add(i), last as u8);
        }
        last
    }

    /// `op_texels_halve_v_00a38560(dst, dst_step, src, src_step, n, size)`
    /// (`0x00a38560`): [`op_texels_halve`] down a column: texel `i`
    /// (`dst + i dst_step`) is the average of `src + 2i src_step` and the
    /// one `src_step` below it. Returns `eax` as left: `src_step` when
    /// anything was averaged, else `size`.
    pub fn op_texels_halve_v(
        mem: &mut impl Mem,
        dst: u32,
        dst_step: u32,
        src: u32,
        src_step: u32,
        n: u32,
        size: u32,
    ) -> u32 {
        if (size != 4 && size != 1) || n == 0 {
            return size;
        }
        for i in 0..n {
            let from = src.wrapping_add(i.wrapping_mul(src_step.wrapping_mul(2)));
            let to = dst.wrapping_add(i.wrapping_mul(dst_step));
            let below = from.wrapping_add(src_step);
            if size == 4 {
                let (a, b): (u32, u32) = (mem.read(from), mem.read(below));
                mem.write(to, (a | b) - ((a ^ b) >> 1 & 0x7f7f_7f7f));
            } else {
                let (a, b) = (mem.read::<u8>(from), mem.read::<u8>(below));
                #[allow(clippy::cast_possible_truncation)] // at most 255
                mem.write(to, ((u32::from(a) + u32::from(b) + 1) >> 1) as u8);
            }
        }
        src_step
    }

    /// `opus_0437_fade_step_00a6e260(b)` (`0x00a6e260`): each of the
    /// `u16 b[0]` 0x60-byte entries from `b + 0x3c` that is in use (`i16`
    /// slot at `+0` not negative) counts its `u8 +2` up; below 0x20 its
    /// `u16 +0x52` = `(0x20 - count) * 0x80`, else its slot in the table
    /// `[b + 0xc]` gets `0xfffe` and the entry is freed (slot -1, count 0).
    /// Returns `eax` as left: 0 for no entries, else the end of the last.
    pub fn opus_0437_fade_step<M: Mem>(mem: &mut M, b: u32) -> u32 {
        let count = |mem: &M| u32::from(mem.read::<u16>(b));
        let entry = |i: u32| b.wrapping_add(0x3c + 0x60 * i);
        let mut i = 0;
        while i < count(mem) {
            let e = entry(i);
            i += 1;
            let slot: i16 = mem.read(e);
            if slot < 0 {
                continue;
            }
            let c = mem.read::<u8>(e.wrapping_add(2)).wrapping_add(1);
            mem.write(e.wrapping_add(2), c);
            if c < 0x20 {
                mem.write(e.wrapping_add(0x52), (0x20 - u16::from(c)) * 0x80);
            } else {
                let table: u32 = mem.read(b.wrapping_add(0xc));
                let at = table.wrapping_add((i32::from(slot) * 2).cast_unsigned());
                mem.write(at, 0xfffe_u16);
                mem.write(e, u16::MAX);
                mem.write(e.wrapping_add(2), 0u8);
            }
        }
        if i == 0 { 0 } else { entry(i) }
    }

    /// `pppFtoiRgba_0074c280(dst, src)` and its six copies: the four
    /// floats at `src` truncated toward zero (`fistp` with the rounding
    /// set to chop; out of range or NaN gives the integer indefinite) and
    /// their low bytes stored at `dst`. Returns the last byte, zero-extended,
    /// as left in `eax`.
    pub fn ppp_ftoi_rgba(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        let mut last = 0;
        for i in 0..4 {
            let x: f32 = mem.read(src.wrapping_add(4 * i));
            #[allow(clippy::cast_possible_truncation)] // the low byte
            let b = ffx_bridge::x87::cvttsd2si(f64::from(x)) as u8;
            mem.write(dst.wrapping_add(i), b);
            last = u32::from(b);
        }
        last
    }

    /// `pppKeShpTail3VfCopy_00751680(dst)` (`0x00751680`): the four
    /// emulated VU0 registers at `0x00c8f7b0..0x00c8f7ef` copied to `dst +
    /// 0xc0` (`pppKeShpTail3Draw` keeps a matrix there). Returns the last
    /// word, as left in `eax`.
    pub fn ppp_ke_shp_tail3_vf_copy(mem: &mut impl Mem, dst: u32) -> u32 {
        let vf: [u32; 16] = mem.read(Global::<u32>::new(0x00c8_f7b0).ptr(mem).addr);
        mem.write(dst.wrapping_add(0xc0), vf);
        vf[15]
    }

    /// `pppKeShpTail3XVfCopy_00753080(dst)` (`0x00753080`, for
    /// `pppKeShpTail3XDraw`): as [`ppp_ke_shp_tail3_vf_copy`] (the 16
    /// words at `0x00c8f7b0` to `dst + 0xc0`), then the 12 at `0x00c8f930`
    /// to `dst + 0x100`. Returns the last word, as left in `eax`.
    pub fn ppp_ke_shp_tail3x_vf_copy(mem: &mut impl Mem, dst: u32) -> u32 {
        ppp_ke_shp_tail3_vf_copy(mem, dst);
        let vf: [u32; 12] = mem.read(Global::<u32>::new(0x00c8_f930).ptr(mem).addr);
        mem.write(dst.wrapping_add(0x100), vf);
        vf[11]
    }

    /// `pppKeShpTail3VfLoad_00751740(src)` (`0x00751740`) and its copy
    /// `pppKeShpTail3XVfLoad_00751ab0` (`0x00751ab0`): the emulated VU0
    /// registers back from a saved block, the other way round from
    /// [`ppp_ke_shp_tail3_vf_copy`]: `src` words `0..0x20` to
    /// `0x00c8f830..0x00c8f8af`, `0x20..0x28` to `0x00c8f7f0..0x00c8f80f`,
    /// `0x40..0x4c` to `0x00c8f930..0x00c8f95f`, in that order. Returns the
    /// last word (`src + 0x12c`), as left in `eax`; the FPU is not touched.
    pub fn ppp_ke_shp_tail3_vf_load(mem: &mut impl Mem, src: u32) -> u32 {
        let a: [u32; 0x20] = mem.read(src);
        let at = Global::<u32>::new(0x00c8_f830).ptr(mem).addr;
        mem.write(at, a);
        let b: [u32; 8] = mem.read(src.wrapping_add(0x80));
        let at = Global::<u32>::new(0x00c8_f7f0).ptr(mem).addr;
        mem.write(at, b);
        let c: [u32; 12] = mem.read(src.wrapping_add(0x100));
        let at = Global::<u32>::new(0x00c8_f930).ptr(mem).addr;
        mem.write(at, c);
        c[11]
    }

    /// `pppDataLastPair_00761250(data, a, b)` (`0x00761250`, for
    /// `pppDataRcv`): walks the records from `data + [data + 4]` (a 16-byte
    /// header, its kind the `i8` at `+1`, until kind `0xff`, then `i16 +2`
    /// entries of a size per kind: 20, 28, 32, 40, 24, 32, 40, 48 bytes;
    /// other kinds have none) and stores the `+8` and `+0xc` words of the
    /// last record of kind 2, 3, 6 or 7 at `a` and `b` (0 for none).
    /// Returns `b`, as left in `eax`.
    pub fn ppp_data_last_pair(mem: &mut impl Mem, data: u32, a: u32, b: u32) -> u32 {
        let mut rec = data.wrapping_add(mem.read::<u32>(data.wrapping_add(4)));
        let (mut first, mut second) = (0u32, 0u32);
        loop {
            let kind: i8 = mem.read(rec.wrapping_add(1));
            if kind == -1 {
                break;
            }
            let n = i32::from(mem.read::<i16>(rec.wrapping_add(2)));
            let (size, pair) = match kind {
                0 => (20, false),
                1 => (28, false),
                2 => (32, true),
                3 | 6 => (40, true),
                4 => (24, false),
                5 => (32, false),
                7 => (48, true),
                _ => (0, false),
            };
            if pair {
                first = mem.read(rec.wrapping_add(8));
                second = mem.read(rec.wrapping_add(0xc));
            }
            rec = rec
                .wrapping_add(n.wrapping_mul(size).cast_unsigned())
                .wrapping_add(0x10);
        }
        mem.write(a, first);
        mem.write(b, second);
        b
    }

    /// `pppDataSetTex_007628c0(data, base, a, b)` (`0x007628c0`): walks
    /// the records as [`ppp_data_last_pair`] does; a record of kind 2, 3,
    /// 6 or 7 gets the first two words of the 12-byte texture record
    /// `u8 +4 + base` of `0x00d4be38` (`g_pppPmcomTexRecs`) at
    /// `+8` and `+0xc`, and the last such pair goes to `a` and `b` (0 for
    /// none). Returns `b`, as left in `eax`.
    pub fn ppp_data_set_tex(mem: &mut impl Mem, data: u32, base: u32, a: u32, b: u32) -> u32 {
        let table = Global::<u32>::new(0x00d4_be38).ptr(mem).addr;
        let mut rec = data.wrapping_add(mem.read::<u32>(data.wrapping_add(4)));
        let (mut first, mut second) = (0u32, 0u32);
        loop {
            let kind: i8 = mem.read(rec.wrapping_add(1));
            if kind == -1 {
                break;
            }
            let (size, tex) = match kind {
                0 => (20, false),
                1 => (28, false),
                2 => (32, true),
                3 | 6 => (40, true),
                4 => (24, false),
                5 => (32, false),
                7 => (48, true),
                _ => (0, false),
            };
            if tex {
                let i = u32::from(mem.read::<u8>(rec.wrapping_add(4))).wrapping_add(base);
                let at = table.wrapping_add(i.wrapping_mul(12));
                first = mem.read(at);
                mem.write(rec.wrapping_add(8), first);
                second = mem.read(at.wrapping_add(4));
                mem.write(rec.wrapping_add(0xc), second);
            }
            let n = i32::from(mem.read::<i16>(rec.wrapping_add(2)));
            rec = rec
                .wrapping_add(n.wrapping_mul(size).cast_unsigned())
                .wrapping_add(0x10);
        }
        mem.write(a, first);
        mem.write(b, second);
        b
    }

    /// `g_pppPmcomTexCount` (`0x00d4be34`): the texture records
    /// [`ppp_set_pmcom2_texture`] has filled.
    pub const PMCOM_TEX_COUNT: Global<u32> = Global::new(0x00d4_be34);
    /// `g_pppPmcomTexFirst` (`0x010d4cf4`): the first record of the
    /// current data, or -1 (set from the count on the next upload).
    pub const PMCOM_TEX_FIRST: Global<u32> = Global::new(0x010d_4cf4);
    /// `g_pppPmcomTexRecs` (`0x00d4be38`): 12-byte records, word 0 the
    /// texture's TEX0 low word, then `u16` width, height and their powers
    /// of two.
    pub const PMCOM_TEX_RECS: u32 = 0x00d4_be38;

    const SYNC_PATH: u32 = 0x0062_f880;
    const SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
    const FLUSH_CACHE: u32 = 0x0062_f4c0;
    const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
    const SCE_PRINTF: u32 = 0x0062_fbf0;
    /// `0x00762810(clut, cbp)`: the 8-bit texture's CLUT upload.
    const LOAD_CLUT8: u32 = 0x0076_2810;
    /// "setPmcom2Texture depth error!!!\n"
    const DEPTH_ERROR: u32 = 0x00b5_1778;
    /// "setPmcom2Texture depth error 2 !!!\n"
    const DEPTH_ERROR2: u32 = 0x00b5_179c;

    /// The `log2` of the power of two at or above `v` (0 for `v <= 1`),
    /// as the original's doubling loops count it.
    fn log2_up(v: i32) -> u32 {
        let (mut p, mut n) = (1i32, 0u32);
        while p < v {
            p = p.wrapping_add(p);
            n += 1;
        }
        n
    }

    /// `pppSetPmcom2Texture_007629e0(tex)` (`0x007629e0`, `pppDataRcv`'s):
    /// a pmcom texture header (`i16 +2` depth, `i16 +4`/`+6` width and
    /// height, `u16 +8` CLUT flag, `u16 +0xc`/`+0xe` buffer width and
    /// texture height, `u16 +0x10`/`+0x12` position, `+0x18` TBP, `+0x1c`
    /// CBP, pixels at `+0x20`) becomes the next record of
    /// [`PMCOM_TEX_RECS`] (the first of the data noted in
    /// [`PMCOM_TEX_FIRST`]): width, height and their powers of two. The
    /// pixels go up in strips of 0x40000 bytes: rows of the width's power
    /// of two (at least 0x80 for 4 and 8 bits) in the depth's units, at
    /// most 0x800, and the whole height when that is above the height's
    /// power of two; the CLUT follows the pixels (4 bits: an 8x2 upload;
    /// 8 bits: `0x00762810`). Word 0 becomes the TEX0 low word (TBP, TBW,
    /// PSM, TW, TH). The uploads (`sceGsSetDefLoadImage`, `FlushCache`,
    /// `sceGsExecLoadImage`, `sceGsSyncPath`) go through `call`, with the
    /// two load-image requests in the original's frame below `frame` (at
    /// `-0x60` and `-0xb0`). Another depth prints the error and stops,
    /// leaving word 0 as it was.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation, // the u16 stores
        clippy::cast_sign_loss
    )]
    pub fn ppp_set_pmcom2_texture<M: Mem>(
        mem: &mut M,
        t: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let (image_req, clut_req) = (frame.wrapping_sub(0x60), frame.wrapping_sub(0xb0));
        let index = PMCOM_TEX_COUNT.get(mem);
        if PMCOM_TEX_FIRST.get(mem) == u32::MAX {
            PMCOM_TEX_FIRST.set(mem, index);
        }
        PMCOM_TEX_COUNT.set(mem, index.wrapping_add(1));
        let rec = sym(mem, PMCOM_TEX_RECS).wrapping_add(index.wrapping_mul(12));
        let i16_at = |mem: &M, off: u32| i32::from(mem.read::<i16>(t.wrapping_add(off)));
        let u16_at = |mem: &M, off: u32| u32::from(mem.read::<u16>(t.wrapping_add(off)));
        let width = i16_at(mem, 4);
        mem.write(rec.wrapping_add(4), width as u16);
        let mut width_pow = 1i32 << log2_up(width);
        mem.write(rec.wrapping_add(8), width_pow as u16);
        let height = i16_at(mem, 6);
        mem.write(rec.wrapping_add(6), height as u16);
        let height_pow = 1i32 << log2_up(height);
        mem.write(rec.wrapping_add(0xa), height_pow as u16);
        // the row length in halfwords (kept as a u16, read back
        // sign-extended; only the 4- and 8-bit CLUTs use it) and the PSM
        let (line, psm) = match i16_at(mem, 2) {
            4 => (width_pow >> 2, 0x14),
            8 => (width_pow >> 1, 0x13),
            0x10 => (width_pow, 2),
            0x20 => (width_pow.wrapping_mul(2), 0),
            _ => {
                let msg = sym(mem, DEPTH_ERROR);
                call(mem, sym(mem, SCE_PRINTF), &[msg]);
                return;
            }
        };
        let line = i32::from(line as i16);
        // 4 and 8 bits: at least 0x80 pixels a row (no effect on the rows
        // below: they reach the 0x800 cap either way)
        if psm >= 0x13 && width_pow < 0x80 {
            width_pow = 0x80;
        }
        call(mem, sym(mem, SYNC_PATH), &[0, 0]);
        let tbp = u16_at(mem, 0x18) & 0x3fff;
        let per = 0x40000 / width_pow;
        let mut rows = match psm {
            0x14 => per.wrapping_mul(2),
            0x13 => per,
            2 => per >> 1,
            _ => per >> 2,
        };
        rows = rows.min(0x800);
        if rows > height_pow {
            rows = height;
        }
        // a height of 0 divides by zero in the original (an exception)
        let Some(strips) = height.checked_div(rows) else {
            return;
        };
        let mut upload = |mem: &mut M, offset: i32, rows: i32| {
            let args = [
                image_req,
                (offset >> 8).cast_unsigned().wrapping_add(tbp),
                u16_at(mem, 0xc) >> 6,
                psm,
                u16_at(mem, 0x10),
                u16_at(mem, 0x12),
                width.cast_unsigned(),
                rows.cast_unsigned(),
            ];
            call(mem, sym(mem, SET_DEF_LOAD_IMAGE), &args);
            call(mem, sym(mem, FLUSH_CACHE), &[0]);
            let pixels = offset.cast_unsigned().wrapping_add(0x20).wrapping_add(t);
            call(mem, sym(mem, EXEC_LOAD_IMAGE), &[image_req, pixels]);
            call(mem, sym(mem, SYNC_PATH), &[0, 0]);
        };
        let mut done = 0i32;
        if strips > 0 {
            let mut offset = 0i32;
            for _ in 0..strips {
                upload(mem, offset, rows);
                offset = offset.wrapping_add(0x40000);
            }
            done = strips;
        }
        if height > done.wrapping_mul(rows) {
            let left = height.wrapping_sub(done.wrapping_mul(rows));
            upload(mem, done.wrapping_shl(18), left);
        }
        if mem.read::<u16>(t.wrapping_add(8)) != 0 {
            let size = line.wrapping_mul(height).wrapping_add(7) & !7;
            let clut = t.wrapping_add(size.wrapping_add(0x10).wrapping_mul(2).cast_unsigned());
            let cbp = mem.read::<u32>(t.wrapping_add(0x1c)) >> 5 & 0x3fff;
            match i16_at(mem, 2) {
                4 => {
                    let args = [clut_req, cbp, 1, 0, 0, 0, 8, 2];
                    call(mem, sym(mem, SET_DEF_LOAD_IMAGE), &args);
                    call(mem, sym(mem, FLUSH_CACHE), &[0]);
                    call(mem, sym(mem, EXEC_LOAD_IMAGE), &[clut_req, clut]);
                    call(mem, sym(mem, SYNC_PATH), &[0, 0]);
                }
                8 => {
                    call(mem, sym(mem, LOAD_CLUT8), &[clut, cbp]);
                }
                _ => {
                    let msg = sym(mem, DEPTH_ERROR2);
                    call(mem, sym(mem, SCE_PRINTF), &[msg]);
                    return;
                }
            }
        }
        // the CBP the original shifts into the high word is dropped
        let tbw = u16_at(mem, 0xc) >> 6;
        let tw = log2_up(i32::from(mem.read::<u16>(t.wrapping_add(0xc))));
        let th = log2_up(i32::from(mem.read::<u16>(t.wrapping_add(0xe))));
        let tex0 = tbw << 14 | th.wrapping_shl(30) | tw.wrapping_shl(26) | psm << 20 | tbp;
        mem.write(rec, tex0);
    }

    /// `0x00807e90(out, src, n)`: `n` colours (`r g b a` bytes at `src`)
    /// to hue, saturation, value and alpha (`u16` each at `out`): a zero
    /// dword gives `h` = `s` = 0 (the first two only); else with `max`,
    /// `min` of `r g b`, `d` = `max - min`: `s` = `256 d / max`, `h` =
    /// `128 (g - b) / d` when `r` is `max`, `128 (b - r) / d + 0x100` when
    /// `g` is, else `128 (r - g) / d + 0x200` (0 and 0 when `d` is 0), `v`
    /// = `max`. Returns the end of `out`.
    #[allow(clippy::many_single_char_names)]
    pub fn op_rgb_to_hsv(mem: &mut impl Mem, out: u32, src: u32, n: u32) -> u32 {
        let mut out = out;
        let mut src = src;
        let mut left = n.cast_signed();
        while left > 0 {
            if mem.read::<u32>(src) == 0 {
                mem.write(out, 0u32);
            } else {
                let c = |o: u32| i32::from(mem.read::<u8>(src.wrapping_add(o)));
                let (r, g, b) = (c(0), c(1), c(2));
                let max = r.max(g).max(b);
                let min = r.min(g).min(b);
                let d = max - min;
                let (h, s) = if d == 0 {
                    (0, 0)
                } else {
                    let s = if max == 0 { 0 } else { (d << 8) / max };
                    let h = if r == max {
                        ((g - b) << 7) / d
                    } else if g == max {
                        ((b - r) << 7) / d + 0x100
                    } else {
                        ((r - g) << 7) / d + 0x200
                    };
                    (h, s)
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // u16 stores
                {
                    mem.write(out, h as u16);
                    mem.write(out.wrapping_add(4), max as u16);
                    mem.write(out.wrapping_add(2), s as u16);
                }
                let a = u16::from(mem.read::<u8>(src.wrapping_add(3)));
                mem.write(out.wrapping_add(6), a);
            }
            left -= 1;
            src = src.wrapping_add(4);
            out = out.wrapping_add(8);
        }
        out
    }

    /// `0x00a39bc0(set, n, v)`: a set of seven lists and `n` nodes (the
    /// lists `op_node_requeue` moves between): `+0xc` low three bits
    /// cleared, `+0`, `+4` = `v`, `+8` = `n`; list heads at `+0x10 +
    /// 0x38 k` (`k` 0..7: `+0x14`, `+0x18`, `+0x28`, `+0x2c` 0, previous
    /// and next itself, `+0x30` = `0x70000000`); nodes at `+0x1a8 + 0x38
    /// i`, owner `set`, chained from the first list's head and back, `+0x30`
    /// 0; `+0x198`..`+0x1a4` 0. Returns `set`.
    pub fn op_node_pool_init(mem: &mut impl Mem, set: u32, n: u32, v: u32) -> u32 {
        let at = |o: u32| set.wrapping_add(o);
        let f: u32 = mem.read(at(0xc));
        mem.write(at(0xc), f & !7);
        mem.write(at(4), v);
        mem.write(at(0), v);
        mem.write(at(8), n);
        for k in 0..7u32 {
            let h = 0x10 + 0x38 * k;
            for o in [0x14u32, 0x18, 0x28, 0x2c] {
                mem.write(at(h + o), 0u32);
            }
            mem.write(at(h + 0x1c), at(h));
            mem.write(at(h + 0x20), at(h));
            mem.write(at(h + 0x30), 0x7000_0000u32);
        }
        let head = at(0x10);
        let count = n.cast_signed();
        let mut i = 0i32;
        while i < count {
            let iu = i.cast_unsigned();
            let node = at(0x1a8u32.wrapping_add(iu.wrapping_mul(0x38)));
            mem.write(node.wrapping_add(0x18), set);
            let prev = if i == 0 {
                head
            } else {
                node.wrapping_sub(0x38)
            };
            mem.write(node.wrapping_add(0x20), prev);
            let next = if i == count.wrapping_sub(1) {
                head
            } else {
                node.wrapping_add(0x38)
            };
            mem.write(node.wrapping_add(0x1c), next);
            mem.write(node.wrapping_add(0x30), 0u32);
            i += 1;
        }
        mem.write(at(0x2c), at(0x1a8));
        let last = n.wrapping_mul(7).wrapping_add(0x2e).wrapping_mul(8);
        mem.write(at(0x30), at(last));
        for o in [0x198u32, 0x19c, 0x1a0, 0x1a4] {
            mem.write(at(o), 0u32);
        }
        set
    }

    /// `0x008888e0(obj)`: `u16 +0xfe` = 0, `u16 +0xf6` = `0x808`, bytes
    /// `+0xf0`..`+0xf5` 0xff and `+0xf8`..`+0xfd` 0. Returns `obj + 0xfe`
    /// (left in `eax`).
    pub fn op_slot_bytes_reset(mem: &mut impl Mem, obj: u32) -> u32 {
        let at = |o: u32| obj.wrapping_add(o);
        mem.write(at(0xfe), 0u16);
        mem.write(at(0xf6), 0x808u16);
        for i in 0..6u32 {
            mem.write(at(0xf0 + i), 0xffu8);
            mem.write(at(0xf8 + i), 0u8);
        }
        at(0xfe)
    }

    /// `0x00888890(n)`: `n` added to the dword at `0x01330278` when
    /// positive. Returns `n` (left in `eax`).
    pub fn op_counter_add(mem: &mut impl Mem, n: u32) -> u32 {
        if n.cast_signed() > 0 {
            let g = Global::<u32>::new(0x0133_0278);
            let v = g.get(mem);
            g.set(mem, v.wrapping_add(n));
        }
        n
    }

    /// `getAlphaForShpIgnoreAnFormatBaseOnSh(shp)` (`0x00716550`): the
    /// byte `+0x1c` when the byte `+0xf` is 3, else the byte at `shp +
    /// shp[1] + 4`.
    pub fn shp_alpha(mem: &impl Mem, shp: u32) -> u32 {
        let at = if mem.read::<u8>(shp.wrapping_add(0xf)) == 3 {
            shp.wrapping_add(0x1c)
        } else {
            let off: u32 = mem.read(shp.wrapping_add(4));
            off.wrapping_add(shp).wrapping_add(4)
        };
        u32::from(mem.read::<u8>(at))
    }

    /// `0x00a3ba10(node)`: the 16-byte slot of a node of the render object
    /// `[0x01a858b4]` (0x38-byte nodes from `+0x438`, slots from
    /// `+0x630`): `base + 16 ((node - base - 0x438) / 0x38 + 99)`, the
    /// division truncated.
    pub fn op_node_slot(mem: &impl Mem, node: u32) -> u32 {
        let g = Global::<u32>::new(0x01a8_58b4);
        let d = node
            .wrapping_sub(g.get(mem))
            .wrapping_sub(0x438)
            .cast_signed();
        let q = d / 0x38 + 0x63;
        q.cast_unsigned().wrapping_shl(4).wrapping_add(g.get(mem))
    }

    /// `rt_stack_reset_00a3abe0(w, h, x, y)` (`0x00a3abe0`, stdcall, name
    /// ours; a callback `FUN_00a35780` stores at `0x01a8582c`): on the
    /// render object `r` = `[0x01a858b4]`, clears the 8-entry stack at `r
    /// +0x9cc` (0x1c bytes each, `memset`), points its top `r +0xaac` at
    /// it, sets `+0x6d0`/`+0x6d8` = `w`, `+0x6d4`/`+0x6dc` = `h`, the
    /// floats `+0x7a4` = `x`, `+0x7a8` = `y` (`fld`/`fstp`), then calls
    /// `0x00a3cd30()`, whose `eax` it returns. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn rt_stack_reset<M: Mem>(
        mem: &mut M,
        w: u32,
        h: u32,
        x: u32,
        y: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::round_trip;
        let r = Global::<u32>::new(0x01a8_58b4).get(mem);
        let at = |o: u32| r.wrapping_add(o);
        let stack = at(0x9cc);
        call(mem, sym(mem, 0x0094_964c), &[stack, 0, 0xe0]);
        mem.write(at(0xaac), stack);
        mem.write(at(0x6d0), w);
        mem.write(at(0x6d4), h);
        mem.write(at(0x6d8), w);
        mem.write(at(0x6dc), h);
        let f = |v: u32| round_trip(f32::from_bits(v)).to_bits();
        mem.write(at(0x7a4), f(x));
        mem.write(at(0x7a8), f(y));
        call(mem, sym(mem, 0x00a3_cd30), &[])
    }

    /// `rt_stack_pop_00a3afb0(a)` (`0x00a3afb0`, stdcall, name ours; a
    /// callback `FUN_00a35780` stores at `0x01a85888`): pops the render
    /// object's stack (`r +0xaac` -= 0x1c, `r` = `[0x01a858b4]`); with
    /// `r +0x6d8` and `+0x6dc` both nonzero and the old top below `r
    /// +0xaac`, calls `0x00a3c570(a)`, `0x00a3cd30()`, `0x00a3c350()` and
    /// returns the word at the old top, else 0. Callees through `call`.
    pub fn rt_stack_pop<M: Mem>(
        mem: &mut M,
        a: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = Global::<u32>::new(0x01a8_58b4).get(mem);
        let at = |o: u32| r.wrapping_add(o);
        let top: u32 = mem.read(at(0xaac));
        mem.write(at(0xaac), top.wrapping_sub(0x1c));
        if mem.read::<u32>(at(0x6d8)) == 0 || mem.read::<u32>(at(0x6dc)) == 0 || top >= at(0xaac) {
            return 0;
        }
        call(mem, sym(mem, 0x00a3_c570), &[a]);
        call(mem, sym(mem, 0x00a3_cd30), &[]);
        call(mem, sym(mem, 0x00a3_c350), &[]);
        mem.read(top)
    }

    /// `0x00a3c0f0(w, h, levels)`: the bytes of a mip chain of 16-bit
    /// texels: `2 w h` summed over `levels`, `w` and `h` halved (arithmetic
    /// shift) each level, at least 1.
    #[must_use]
    pub fn op_mip_bytes(w: u32, h: u32, levels: u32) -> u32 {
        let (mut w, mut h) = (w.cast_signed(), h.cast_signed());
        let mut sum = 0i32;
        let mut n = levels.cast_signed();
        while n > 0 {
            sum = sum.wrapping_add(w.wrapping_mul(h).wrapping_mul(2));
            w = (w >> 1) + i32::from(w >> 1 == 0);
            h = (h >> 1) + i32::from(h >> 1 == 0);
            n -= 1;
        }
        sum.cast_unsigned()
    }

    fn sym(mem: &impl Mem, at: u32) -> u32 {
        Global::<u32>::new(at).ptr(mem).addr
    }

    /// `ClassDynamicGeometry_setupDynMesh(this)` (`0x006a3b30`, thiscall):
    /// `FUN_00673030` (thiscall on `[this +8]`, the engine's) of `[this
    /// +0x24]`, through `this_call(f, this, args)`. Returns 0.
    pub fn setup_dyn_mesh<M: Mem>(
        mem: &mut M,
        this: u32,
        this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        let mesh: u32 = mem.read(this.wrapping_add(0x24));
        let obj: u32 = mem.read(this.wrapping_add(8));
        this_call(mem, sym(mem, 0x0067_3030), obj, &[mesh]);
        0
    }

    /// `0x0080a470` (`opus_colour_scale2`, ours): the four `f32` arguments
    /// through the FPU (`fld`/`fstp`: a signalling NaN comes back quiet)
    /// to `graphicSetMaterialColourScale2` (through `call`); returns its
    /// `eax`.
    pub fn opus_colour_scale2<M: Mem>(
        mem: &mut M,
        rgba: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let a = rgba.map(x87::load);
        call(mem, sym(mem, 0x0064_3670), &a)
    }

    /// `0x00801ea0` (`opus_view_screen_matrix`, ours):
    /// `FFXSamp0ViewScreenMatrix(m, 512, 0.8, 0.87285, 2048, 2048, 1,
    /// 16777215, 1, 65536)` (the `.rdata` constants `0x00b41d9c`,
    /// `0x00b34254`, `0x00b582fc`, `0x00b51a20` twice, `fld1`,
    /// `0x00b51a24`, `fld1`, `0x00b43ddc`), through `call`; returns its
    /// `eax`.
    pub fn opus_view_screen_matrix<M: Mem>(
        mem: &mut M,
        m: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let c = |mem: &M, at: u32| x87::load(mem.read(sym(mem, at)));
        let one = 1.0f32.to_bits();
        let args = [
            m,
            c(mem, 0x00b4_1d9c),
            c(mem, 0x00b3_4254),
            c(mem, 0x00b5_82fc),
            c(mem, 0x00b5_1a20),
            c(mem, 0x00b5_1a20),
            one,
            c(mem, 0x00b5_1a24),
            one,
            c(mem, 0x00b4_3ddc),
        ];
        call(mem, sym(mem, 0x0093_d960), &args)
    }

    /// `graphicVFXAddScreenFadeEvent(r, g, b, a, alpha)` (`0x00644d40`): a
    /// screen fade draw event in `event` (the original's local: `+0` = 2,
    /// `+4` left as it is, `+8` the four colour bytes, `+0xc`
    /// `calculateVFXSP_AlphaFunction(alpha)`) handed to
    /// `PhyreScene_addVFXDrawEvent` (thiscall on `g_PhyreScene`, through
    /// `this_call`) unless `g_GraphicSkipFrame` is set. Returns that call's
    /// `eax`, or on a skipped frame the alpha function.
    pub fn graphic_vfx_add_screen_fade_event<M: Mem>(
        mem: &mut M,
        rgba: [u32; 4],
        alpha: u32,
        event: u32,
        this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the argument bytes
        let bytes = rgba.map(|c| c as u8);
        mem.write(event.wrapping_add(8), bytes);
        mem.write(event, 2u32);
        let f = crate::op::bridge::calculate_vfx_sp_alpha_function(alpha & 0xff);
        mem.write(event.wrapping_add(0xc), f);
        if mem.read::<u32>(sym(mem, 0x012f_b7d0)) != 0 {
            return f;
        }
        let scene: u32 = mem.read(sym(mem, 0x00cc_b9d8));
        this_call(mem, sym(mem, 0x0065_6940), scene, &[event])
    }

    /// `0x00711750` (`pppAkazaScrFltDraw`, ours): `graphicVFXAddCopyBufferEvent(0,
    /// 1)`; the instance `(lo, hi)` gets its render data when it has none
    /// (`pppCreateFrameBufferData_00715a40`, then `graphicVFXSetup`; either
    /// failing ends in `graphicVFXDestroy(lo, hi)`), then
    /// `updateVFXRenderData_AkazaScrFlt(lo, hi, a, b)` and
    /// `graphicVFXUpdateType4(lo, hi)`. All through `call`.
    pub fn ppp_akaza_scr_flt_draw<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        call(mem, sym(mem, 0x0064_4be0), &[0, 1]);
        if call(mem, sym(mem, 0x0064_53d0), &[lo, hi]) == 0
            && (call(mem, sym(mem, 0x0071_5a40), &[lo, hi]) == 0
                || call(mem, sym(mem, 0x0064_5780), &[lo, hi]) == 0)
        {
            call(mem, sym(mem, 0x0064_5280), &[lo, hi]);
            return;
        }
        call(mem, sym(mem, 0x0071_ce20), &[lo, hi, a, b]);
        call(mem, sym(mem, 0x0064_4db0), &[lo, hi]);
    }

    /// `0x007116d0` (`pppNoiseBlurDraw`, ours; `(a0..a6)`): `0x00cec19c` =
    /// 0; renderer instance `(6, 0)` gets its render data when it has none
    /// (`pppCreateD3D11FrameBufferData_00716050(6, 0, a2)`, then
    /// `graphicVFXSetup(6, 0)`; either failing ends in
    /// `graphicVFXDestroy(6, 0)`), then `updateVFXRenderData_NoiseBlur(6,
    /// 0, a0..a6)` and `graphicVFXUpdate(6, 0)`. All through `call`.
    pub fn ppp_noise_blur_draw<M: Mem>(
        mem: &mut M,
        a: [u32; 7],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        mem.write(sym(mem, 0x00ce_c19c), 0u32);
        if call(mem, sym(mem, 0x0064_53d0), &[6, 0]) == 0
            && (call(mem, sym(mem, 0x0071_6050), &[6, 0, a[2]]) == 0
                || call(mem, sym(mem, 0x0064_5780), &[6, 0]) == 0)
        {
            call(mem, sym(mem, 0x0064_5280), &[6, 0]);
            return;
        }
        let args = [6, 0, a[0], a[1], a[2], a[3], a[4], a[5], a[6]];
        call(mem, sym(mem, 0x0071_dbd0), &args);
        call(mem, sym(mem, 0x0064_58a0), &[6, 0]);
    }

    /// `create0706NoiseRenderDataBuffer(lo, hi)` (`0x007fa510`): the
    /// original's locals at `frame` (`+0` format 1, `+4` height, `+8`
    /// width, `+0xc` the name, `sprintf` of the format string at
    /// `0x00b45b3c` through the import slot `0x00b0c3dc`);
    /// `graphicVFXCreate(lo, hi, 0x20000000)`, then
    /// `graphicVFXCreateDataBuffer(lo, hi, 1, &width 0x700, &height 0xa80,
    /// name, &format)` (all through `call`); `0x012e9644` = 1. Returns 1.
    pub fn create_0706_noise_render_data_buffer<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (format, height, width, name) = (frame, frame + 4, frame + 8, frame + 0xc);
        mem.write(width, 0u32);
        mem.write(height, 0u32);
        mem.write(format, 1u32);
        let sprintf: u32 = mem.read(sym(mem, 0x00b0_c3dc));
        call(mem, sprintf, &[name, sym(mem, 0x00b4_5b3c)]);
        mem.write(width, 0x700u32);
        mem.write(height, 0xa80u32);
        call(mem, sym(mem, 0x0064_4de0), &[lo, hi, 0x2000_0000]);
        call(
            mem,
            sym(mem, 0x0064_5040),
            &[lo, hi, 1, width, height, name, format],
        );
        mem.write(sym(mem, 0x012e_9644), 1u32);
        1
    }

    /// `0x0069fcb0` (`DynGeo_ResetCurrentBuffer`, ours; thiscall `(this)`):
    /// `this +0x10` = `graphicDynGeoMemManagerGetCurrentBufferId()` (through
    /// `call`); that buffer's record (`[[this + 4 id] +0x94]`, read again
    /// for each step) gets `+4`, `+8` = 0 and its arrays zeroed (`memset`):
    /// `[+0x14]` 0x1f40 bytes, `[+0x1c]` 0xbb8, `[+0x10]` 0x1770, `[+0x18]`
    /// 0xfa0, `[+0xc]` 0x1770. Returns the last array (`memset`'s eax).
    pub fn dyn_geo_reset_current_buffer<M: Mem>(
        mem: &mut M,
        this: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = call(mem, sym(mem, 0x0063_ff50), &[]);
        mem.write(this.wrapping_add(0x10), id);
        let rec = |mem: &M| {
            let id: u32 = mem.read(this.wrapping_add(0x10));
            let b: u32 = mem.read(this.wrapping_add(id.wrapping_mul(4)));
            mem.read::<u32>(b.wrapping_add(0x94))
        };
        let r = rec(mem);
        mem.write(r.wrapping_add(4), 0u32);
        let r = rec(mem);
        mem.write(r.wrapping_add(8), 0u32);
        let mut last = 0;
        for (off, n) in [
            (0x14, 0x1f40),
            (0x1c, 0xbb8),
            (0x10, 0x1770),
            (0x18, 0xfa0),
            (0xc, 0x1770),
        ] {
            let dst: u32 = mem.read(rec(mem).wrapping_add(off));
            for i in (0..n).step_by(4) {
                mem.write(dst.wrapping_add(i), 0u32);
            }
            last = dst;
        }
        last
    }

    /// The renderer instance `(g_pppCurrentProgram, hi)` (the program
    /// sign-extended to 64 bits plus `hi` in the high word), read again
    /// each time.
    fn program_instance(mem: &impl Mem, hi: u32) -> [u32; 2] {
        let lo: u32 = mem.read(sym(mem, 0x0230_fd20));
        [
            lo,
            (lo.cast_signed() >> 31).cast_unsigned().wrapping_add(hi),
        ]
    }

    /// `0x00711830` (`pppKaGra001Draw`, ours; `(hi, a, rec)`): for the
    /// current program's renderer instance ([`program_instance`]) with
    /// render data `updateVFXRenderData_KaGra001(inst, a, rec)` and
    /// `graphicVFXUpdate(inst)`; without, it is made first
    /// (`FUN_00715dc0(inst, a, rec, f)`, `f` =
    /// `calculateVFXSP_AlphaFunction(byte rec +8)` | 1 when `rec +0x44` is
    /// set | 0x40 with bit 3 of byte `rec +0`, then `graphicVFXSetup`;
    /// either failing ends in `graphicVFXDestroy`). All through `call`.
    pub fn ppp_ka_gra001_draw<M: Mem>(
        mem: &mut M,
        hi: u32,
        a: u32,
        rec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let i = program_instance(mem, hi);
        if call(mem, sym(mem, 0x0064_53d0), &i) != 0 {
            let i = program_instance(mem, hi);
            call(mem, sym(mem, 0x0071_d520), &[i[0], i[1], a, rec]);
            let i = program_instance(mem, hi);
            call(mem, sym(mem, 0x0064_58a0), &i);
            return;
        }
        let mut f = crate::op::bridge::calculate_vfx_sp_alpha_function(u32::from(
            mem.read::<u8>(rec.wrapping_add(8)),
        ));
        f |= u32::from(mem.read::<u32>(rec.wrapping_add(0x44)) != 0);
        if mem.read::<u8>(rec) & 8 != 0 {
            f |= 0x40;
        }
        let i = program_instance(mem, hi);
        if call(mem, sym(mem, 0x0071_5dc0), &[i[0], i[1], a, rec, f]) == 0
            || call(mem, sym(mem, 0x0064_5780), &i) == 0
        {
            call(mem, sym(mem, 0x0064_5280), &i);
            return;
        }
        call(mem, sym(mem, 0x0071_d520), &[i[0], i[1], a, rec]);
        call(mem, sym(mem, 0x0064_58a0), &i);
    }

    /// `weaponShadowDrawInit(lo, hi, _, mode)` (`0x00814070`): the
    /// original's locals at `frame` as in
    /// [`create_0706_noise_render_data_buffer`] (format 0, height 0x300,
    /// width 0x102, the name `g_StrNoTexture`'s ten bytes);
    /// `graphicVFXCreate(lo, hi, f)` (`f` 0x10 for mode byte 0x42, 2 for
    /// 0x48, else 0), `graphicVFXCreateDataBuffer(lo, hi, 1, &width,
    /// &height, name, &format)`, `graphicVFXSetup`, `graphicVFXSetVisible(lo,
    /// hi, 1)`, whose eax it returns (all through `call`).
    pub fn weapon_shadow_draw_init<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        mode: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (format, height, width, name) = (frame, frame + 4, frame + 8, frame + 0xc);
        mem.write(width, 0u32);
        mem.write(height, 0u32);
        mem.write(format, 0u32);
        let f = match mode & 0xff {
            0x42 => 0x10,
            0x48 => 2,
            _ => 0,
        };
        call(mem, sym(mem, 0x0064_4de0), &[lo, hi, f]);
        let text: [u8; 10] = mem.read(sym(mem, 0x00b4_1b50));
        mem.write(name, text);
        mem.write(width, 0x102u32);
        mem.write(height, 0x300u32);
        call(
            mem,
            sym(mem, 0x0064_5040),
            &[lo, hi, 1, width, height, name, format],
        );
        call(mem, sym(mem, 0x0064_5780), &[lo, hi]);
        call(mem, sym(mem, 0x0064_5710), &[lo, hi, 1])
    }

    /// `0x006a52f0` (`VfxGeometry_Register`, ours; thiscall `(this, kind)`,
    /// `ret 4`): `r` = 0 for a `kind` with bit 30 (cleared), else the
    /// object's virtual `+0x28(kind)` (`v_call(f, this, args)`); bit 31
    /// becomes the draw entry's flag (the low byte of the original argument
    /// word), then cleared. With `[0x00ccca58]` = 1 any kind but 2 is 4, as
    /// is any with bit 8 of `this +0x3c`. With `this +0x24` set and `r` 0,
    /// the scene (`g_PhyreScene`) gets by kind: 0 `FUN_006565e0([this
    /// +0xc])` when byte `this +0x50` is set, else
    /// `PhyreScene_AddDrawEntry(this, flag)`; 1 `FUN_006a0d40`, 2
    /// `FUN_00656990`, 4 `FUN_006567e0` of `[this +0xc]` (thiscalls through
    /// `this_call`). Returns `r`.
    pub fn vfx_geometry_register<M: Mem>(
        mem: &mut M,
        this: u32,
        kind: u32,
        this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut k = kind;
        let mut r = 0;
        if k & 0x4000_0000 != 0 {
            k &= !0x4000_0000;
        } else {
            let vt: u32 = mem.read(this);
            let f: u32 = mem.read(vt.wrapping_add(0x28));
            r = this_call(mem, f, this, &[k]);
        }
        let flag = kind & 0xffff_ff00 | u32::from(k & 0x8000_0000 != 0);
        k &= 0x7fff_ffff;
        let one = mem.read::<u32>(sym(mem, 0x00cc_ca58)) == 1;
        if (one && k != 2) || mem.read::<u32>(this.wrapping_add(0x3c)) & 0x100 != 0 {
            k = 4;
        }
        if mem.read::<u32>(this.wrapping_add(0x24)) == 0 || r != 0 || k > 4 {
            return r;
        }
        let scene: u32 = mem.read(sym(mem, 0x00cc_b9d8));
        let item: u32 = mem.read(this.wrapping_add(0xc));
        match k {
            0 if mem.read::<u8>(this.wrapping_add(0x50)) != 0 => {
                this_call(mem, sym(mem, 0x0065_65e0), scene, &[item]);
            }
            0 => {
                this_call(mem, sym(mem, 0x0065_6c10), scene, &[this, flag]);
            }
            1 => {
                this_call(mem, sym(mem, 0x006a_0d40), scene, &[item]);
            }
            2 => {
                this_call(mem, sym(mem, 0x0065_6990), scene, &[item]);
            }
            4 => {
                this_call(mem, sym(mem, 0x0065_67e0), scene, &[item]);
            }
            _ => {}
        }
        r
    }

    /// `0x00716110` (`createZBufClearRenderData`, ours; `(lo, hi)`): the
    /// original's locals at `frame` (`+0` height 6, `+4` width 4, `+8`
    /// format 0, seven zero words from `+0xc`, the name at `+0x28`:
    /// `sprintf(name, g_StrNoTexture)` through the import slot
    /// `0x00b0c3dc`); `graphicVFXCreate(lo, hi, 0x40000000)`,
    /// `graphicVFXCreateDataBuffer(lo, hi, 1, &width, &height, name,
    /// &format)` (through `call`). Returns 1 when the instance has render
    /// data then (`graphicVFXGetRenderDataByInstance`), else 0.
    pub fn create_zbuf_clear_render_data<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (height, width, format, name) = (frame, frame + 4, frame + 8, frame + 0x28);
        mem.write(frame + 0xc, [0u32; 7]);
        mem.write(width, 0u32);
        mem.write(height, 0u32);
        mem.write(format, 0u32);
        let sprintf: u32 = mem.read(sym(mem, 0x00b0_c3dc));
        call(mem, sprintf, &[name, sym(mem, 0x00b4_1b50)]);
        mem.write(width, 4u32);
        mem.write(height, 6u32);
        call(mem, sym(mem, 0x0064_4de0), &[lo, hi, 0x4000_0000]);
        call(
            mem,
            sym(mem, 0x0064_5040),
            &[lo, hi, 1, width, height, name, format],
        );
        u32::from(call(mem, sym(mem, 0x0064_53d0), &[lo, hi]) != 0)
    }

    // ---- round 34 a ----

    /// `op_init_draw()` (`0x007ec410`): `ops_init_draw_work()`, then (a
    /// tail jump) `ops_init_spr()`, whose eax it returns. Both through
    /// `call`.
    pub fn op_init_draw<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, sym(mem, 0x007e_c890), &[]);
        call(mem, sym(mem, 0x007e_6c30), &[])
    }

    /// `op_ovl_effect_forced_free()` (`0x007fcd50`): `opus_end_all_sub(-1)`,
    /// then (a tail jump) `op_ovl_effect_free()`, whose eax it returns. Both
    /// through `call`.
    pub fn op_ovl_effect_forced_free<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, sym(mem, 0x007f_e6c0), &[u32::MAX]);
        call(mem, sym(mem, 0x007f_cd60), &[])
    }

    /// `0x00888910(works)` (`Pad_ResetSlots`, ours): the four 0x20-byte
    /// slot works from `works` each get `Pad_ResetWork(work)` (through
    /// `call`), then `u16 +2` and `u16 +0x1e` = 0. Returns 0 (the
    /// original's `eax`).
    pub fn pad_reset_slots<M: Mem>(
        mem: &mut M,
        works: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for k in 0..4u32 {
            let work = works.wrapping_add(0x20 * k);
            call(mem, sym(mem, 0x0088_8970), &[work]);
            mem.write(work.wrapping_add(2), 0u16);
            mem.write(work.wrapping_add(0x1e), 0u16);
        }
        0
    }

    /// `InitVFXZBufClearRenderData(lo, hi)` (`0x00711680`):
    /// `createZBufClearRenderData(lo, hi)`; when that gives nonzero,
    /// `graphicVFXSetup(lo, hi)`, whose nonzero eax is returned. Otherwise
    /// `graphicVFXDestroy(lo, hi)` and its eax. All through `call`.
    pub fn init_vfx_zbuf_clear_render_data<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if call(mem, sym(mem, 0x0071_6110), &[lo, hi]) != 0 {
            let r = call(mem, sym(mem, 0x0064_5780), &[lo, hi]);
            if r != 0 {
                return r;
            }
        }
        call(mem, sym(mem, 0x0064_5280), &[lo, hi])
    }

    /// `initVFXRenderDataBuffer_FakeEnvModel(a0, .., a8)` (`0x00716620`,
    /// Fahrenheit's name): the instance is `a8` with `a0` added to its
    /// sign-extended high word (`lo = a8`, `hi = a0 + (a8 >> 31)`).
    /// `0x00715af0(a0..a8)`, then `graphicVFXSetup(lo, hi)`; when both give
    /// nonzero, `updateVFXRenderData_FakeEnvModel(a0..a8)` and
    /// `graphicVFXUpdate(lo, hi)`, whose eax is returned; otherwise
    /// `graphicVFXDestroy(lo, hi)` and its eax. All through `call`.
    pub fn init_vfx_render_data_buffer_fake_env_model<M: Mem>(
        mem: &mut M,
        args: [u32; 9],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let lo = args[8];
        let hi = args[0].wrapping_add((lo.cast_signed() >> 31).cast_unsigned());
        if call(mem, sym(mem, 0x0071_5af0), &args) != 0
            && call(mem, sym(mem, 0x0064_5780), &[lo, hi]) != 0
        {
            call(mem, sym(mem, 0x0071_cf90), &args);
            return call(mem, sym(mem, 0x0064_58a0), &[lo, hi]);
        }
        call(mem, sym(mem, 0x0064_5280), &[lo, hi])
    }

    /// `InitGuildMapRenderData(lo, hi, a, b)` (`0x00710ed0`, Fahrenheit's
    /// name): `createGuildMapRenderDataBuffer(lo, hi, a, b)`; when that
    /// gives nonzero, `graphicGuidemapSetup(lo, hi)`, and the render data
    /// of the instance (`graphicVFXGetRenderDataByInstance`) has the word
    /// at `+8` of each of its `i32 +0x90` 0x6c-byte entries (at `+0x94`)
    /// divided by 3. With the set-up's nonzero result,
    /// `updateGuildMapRenderData(lo, hi, a, b)`, whose nonzero eax is
    /// returned. Otherwise `graphicVFXDestroy(lo, hi)` and its eax. All
    /// through `call`.
    pub fn init_guild_map_render_data<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [lo, hi, ..] = args;
        if call(mem, sym(mem, 0x0071_33e0), &args) != 0 {
            let ready = call(mem, sym(mem, 0x0064_1490), &[lo, hi]);
            let data = call(mem, sym(mem, 0x0064_53d0), &[lo, hi]);
            if data != 0 {
                let mut i = 0i32;
                while i < mem.read::<i32>(data.wrapping_add(0x90)) {
                    let entries: u32 = mem.read(data.wrapping_add(0x94));
                    let at = entries
                        .wrapping_add(i.cast_unsigned().wrapping_mul(0x6c))
                        .wrapping_add(8);
                    let v: u32 = mem.read(at);
                    mem.write(at, v / 3);
                    i += 1;
                }
            }
            if ready != 0 {
                let r = call(mem, sym(mem, 0x0071_7620), &args);
                if r != 0 {
                    return r;
                }
            }
        }
        call(mem, sym(mem, 0x0064_5280), &[lo, hi])
    }

    /// `create0706AccRenderDataBuffer(lo, hi, tex_lo, tex_hi)`
    /// (`0x007fa5c0`): the texture word `t = tex_hi:tex_lo` (a GS `TEX0`)
    /// names the texture: `graphicVFXIsTextureWithBpLoaded(t & 0x3fff, t >>
    /// 20 & 0x3f, 0, 0, 1 << (t >> 26 & 0xf), 1 << (t >> 30 & 0xf), name)`
    /// (16-bit shifts). Unless that is loaded (nonzero) it returns 0.
    /// Otherwise `graphicVFXCreate(lo, hi, 0x08000000)` and
    /// `graphicVFXCreateDataBuffer(lo, hi, 1, &0xe00, &0x1500, name,
    /// &format 1)`, and 1. The original's locals at `frame` (`+0` the
    /// format, `+4`, `+8` the two sizes, `+0xc` the 0x100-byte name);
    /// callees through `call`.
    pub fn create_0706_acc_render_data_buffer<M: Mem>(
        mem: &mut M,
        [lo, hi, tex_lo, tex_hi]: [u32; 4],
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (format, first, second, name) = (frame, frame + 4, frame + 8, frame + 0xc);
        let t = u64::from(tex_hi) << 32 | u64::from(tex_lo);
        #[allow(clippy::cast_possible_truncation)] // the 4-bit fields
        let size = |shift: u32| u32::from(1u16 << ((t >> shift) as u32 & 0xf));
        mem.write(second, 0u32);
        mem.write(first, 0u32);
        mem.write(format, 1u32);
        #[allow(clippy::cast_possible_truncation)] // the 6-bit field
        let psm = (t >> 20) as u32 & 0x3f;
        let args = [tex_lo & 0x3fff, psm, 0, 0, size(26), size(30), name];
        if call(mem, sym(mem, 0x0064_5450), &args) == 0 {
            return 0;
        }
        mem.write(second, 0xe00u32);
        mem.write(first, 0x1500u32);
        call(mem, sym(mem, 0x0064_4de0), &[lo, hi, 0x0800_0000]);
        call(
            mem,
            sym(mem, 0x0064_5040),
            &[lo, hi, 1, second, first, name, format],
        );
        1
    }

    /// `0x006d54c0` (`ShadowEntries_Init`, ours; thiscall `(this, n,
    /// buf)`, `ret 8`): `this +0x34` = `n`, `+0x38` = `buf`, a table of `n`
    /// pointers; the records follow it from `buf` + the table's size
    /// rounded up to 16 (`(4n - 1 & !0xf) + 0x10`). Each record is a
    /// 16-byte head (zeroed, its address into the table) and two 0xb0-byte
    /// entries `ShadowEntry_Init(entry, 1)` and `(entry, 2)` (thiscall,
    /// through `this_call`) whose results go to the head's `+0` and `+4`.
    /// A record address of 0 (wrapping) gets null instead, as the
    /// original's `new` checks. Returns the address after the last record.
    pub fn shadow_entries_init<M: Mem>(
        mem: &mut M,
        this: u32,
        n: u32,
        buf: u32,
        this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        let init = sym(mem, 0x006d_3560);
        let mut at = (n.wrapping_mul(4).wrapping_sub(1) & !0xf)
            .wrapping_add(0x10)
            .wrapping_add(buf);
        mem.write(this.wrapping_add(0x34), n);
        mem.write(this.wrapping_add(0x38), buf);
        let slot = |mem: &M, i: u32| {
            let table: u32 = mem.read(this.wrapping_add(0x38));
            let head: u32 = mem.read(table.wrapping_add(4 * i));
            head
        };
        for i in 0..n.cast_signed().max(0).cast_unsigned() {
            let head = if at == 0 {
                0
            } else {
                mem.write(at, [0u32; 4]);
                at
            };
            let table: u32 = mem.read(this.wrapping_add(0x38));
            mem.write(table.wrapping_add(4 * i), head);
            at = at.wrapping_add(0x10);
            let e = if at == 0 {
                0
            } else {
                this_call(mem, init, at, &[1])
            };
            let h = slot(mem, i);
            mem.write(h, e);
            at = at.wrapping_add(0xb0);
            let e = if at == 0 {
                0
            } else {
                this_call(mem, init, at, &[2])
            };
            let h = slot(mem, i);
            mem.write(h.wrapping_add(4), e);
            at = at.wrapping_add(0xb0);
        }
        at
    }

    /// `pppInitProtocolDriver(drv, a, b, c, e)` (`0x0076b2d0`; the
    /// arguments' names are guesses): the driver record `drv` gets `+4` =
    /// `a`, `+0x3c` = `b`, `+0x40` = `c`, `+0x44` = `e`, `+0x10` = `0x010d5640`, `+0x14` = `0x010d7640` (its two
    /// buffers), and `+8`, `+0xc`, `+0x24` = 0; `g_pppWaitDone` = 1; the
    /// twelve PS2 debug handlers (`SetDebugHandler`, a stub on PC) for
    /// causes 1..7 and 9..13; `0x010e7874` = `GetCurrentThreadId()` (the
    /// import thunk `0x006f2ec0`). Callees through `call`. Returns 1.
    pub fn ppp_init_protocol_driver<M: Mem>(
        mem: &mut M,
        [drv, mode, send, recv, user]: [u32; 5],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const HANDLERS: [(u32, u32); 12] = [
            (1, 0x0076_ac90),
            (2, 0x0076_ad30),
            (3, 0x0076_ad50),
            (4, 0x0076_ad70),
            (5, 0x0076_ad90),
            (6, 0x0076_adb0),
            (7, 0x0076_add0),
            (9, 0x0076_adf0),
            (10, 0x0076_acb0),
            (11, 0x0076_acd0),
            (12, 0x0076_acf0),
            (13, 0x0076_ad10),
        ];
        let at = |o: u32| drv.wrapping_add(o);
        mem.write(at(4), mode);
        mem.write(at(0x3c), send);
        mem.write(at(0x10), sym(mem, 0x010d_5640));
        mem.write(at(0x14), sym(mem, 0x010d_7640));
        mem.write(at(8), 0u32);
        mem.write(at(0x40), recv);
        mem.write(at(0xc), 0u32);
        mem.write(at(0x24), 0u32);
        mem.write(at(0x44), user);
        mem.write(sym(mem, 0x010e_7640), 1u32);
        for (cause, handler) in HANDLERS {
            let handler = sym(mem, handler);
            call(mem, sym(mem, 0x0062_f4d0), &[cause, handler]);
        }
        let id = call(mem, sym(mem, 0x006f_2ec0), &[]);
        mem.write(sym(mem, 0x010e_7874), id);
        1
    }

    // ---- round 34 c ----

    /// `checkPolyImg0434(lo, n, make)` (`0x00713160`): render data for
    /// `n` polygons of instance `(0, lo)`: none (`graphicVFXDestroy`, 0)
    /// for `n` = 0; with `make`, `graphicVFXCreate(0, lo, 2)` and
    /// `graphicVFXCreateDataBuffer(0, lo, 1, &6n, &6n, "NoTexture",
    /// &1)` (the original's locals at `frame`: `+0` the format, `+4`,
    /// `+8` the counts, `+0xc` the name copied from `g_StrNoTexture`). Then
    /// the instance's render data `r`
    /// (`graphicVFXGetRenderDataByInstance`, 0 when none): its `[+0x94]`
    /// gets `+4`, `+8` = `6n`, and
    /// `graphicDynGeoMemManagerGetBufferForSegment(r, 0, +4, +8)` must give
    /// a buffer (else 0); `0x00cec194` = 0 and 1. Callees through `call`.
    pub fn check_poly_img_0434<M: Mem>(
        mem: &mut M,
        [lo, n, make]: [u32; 3],
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if n == 0 {
            call(mem, sym(mem, 0x0064_5280), &[0, lo]);
            return 0;
        }
        let count = n.wrapping_mul(6);
        if make != 0 {
            let (format, first, second, name) = (frame, frame + 4, frame + 8, frame + 0xc);
            mem.write(first, count);
            mem.write(second, count);
            let text: [u8; 10] = mem.read(sym(mem, 0x00b4_1b50));
            mem.write(name, text);
            mem.write(format, 1u32);
            call(mem, sym(mem, 0x0064_4de0), &[0, lo, 2]);
            call(
                mem,
                sym(mem, 0x0064_5040),
                &[0, lo, 1, first, second, name, format],
            );
        }
        let data = call(mem, sym(mem, 0x0064_53d0), &[0, lo]);
        if data == 0 {
            return 0;
        }
        let rec = |mem: &M| mem.read::<u32>(data.wrapping_add(0x94));
        let r = rec(mem);
        mem.write(r.wrapping_add(4), count);
        let r = rec(mem);
        mem.write(r.wrapping_add(8), count);
        let r = rec(mem);
        let (v, i) = (mem.read(r.wrapping_add(4)), mem.read(r.wrapping_add(8)));
        if call(mem, sym(mem, 0x0063_fdb0), &[data, 0, v, i]) == 0 {
            return 0;
        }
        mem.write(sym(mem, 0x00ce_c194), 0u32);
        1
    }

    // ---- round 35 c ----

    /// `0x00c48c10`: the 64-bit instance [`op_zbuf_clear`] draws its z
    /// clear as, cycling through 1..=5.
    pub const ZBUF_CLEAR_INSTANCE: Global<u64> = Global::new(0x00c4_8c10);

    /// `op_zbuf_clear(z)` (`0x007ec7e0`): the clear depth `d` is 1.0 for
    /// `z` 0, else `(f32 z - [0x00b41d78]) / [0x00b41da8]` (`(z -
    /// 16770000) / -16769999`, `f32`); magic 0x9a first syncs the path
    /// (`graphicVFXSyncPath`). The render data of instance
    /// [`ZBUF_CLEAR_INSTANCE`] is set up (`InitVFXZBufClearRenderData`)
    /// when `graphicVFXGetRenderDataByInstance` has none, else updated
    /// (`updateVFXZBufClearRenderData(.., d)`, `graphicVFXPath3Update`);
    /// then the instance steps on, back to 1 past 5. Returns the stepped
    /// low word (left in eax). Callees through `call`.
    pub fn op_zbuf_clear<M: Mem>(
        mem: &mut M,
        z: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{div, load, store, sub};
        let id = ZBUF_CLEAR_INSTANCE.get(mem);
        #[allow(clippy::cast_possible_truncation)] // the two halves
        let (lo, hi) = (id as u32, (id >> 32) as u32);
        let mut depth = 1f32.to_bits();
        if z != 0 {
            let (a, b): (f64, f64) = (
                mem.read(sym(mem, 0x00b4_1d78)),
                mem.read(sym(mem, 0x00b4_1da8)),
            );
            let f = load(store(f64::from(z.cast_signed())));
            depth = store(div(sub(f, a), b));
        }
        if call(mem, sym(mem, 0x009d_a3b0), &[]) == 0x9a {
            call(mem, sym(mem, 0x0064_5840), &[]);
        }
        if call(mem, sym(mem, 0x0064_53d0), &[lo, hi]) == 0 {
            call(mem, sym(mem, 0x0071_1680), &[lo, hi]);
        } else {
            call(mem, sym(mem, 0x0071_e330), &[lo, hi, depth]);
            call(mem, sym(mem, 0x0064_5600), &[lo, hi]);
        }
        let next = ZBUF_CLEAR_INSTANCE.get(mem).wrapping_add(1);
        ZBUF_CLEAR_INSTANCE.set(mem, next);
        if next > 5 {
            ZBUF_CLEAR_INSTANCE.set(mem, 1);
        }
        #[allow(clippy::cast_possible_truncation)] // the low word
        let low = next as u32;
        low
    }

    /// `pppInitPartRender()` (`0x00761d60`): registers the part protocol
    /// (`pppInitProtocolDriver(0x00e524a8, 0xe012, pppPmcomDataInit,
    /// pppPmcomDataQueue, 0x00760020)`, `scePrintf` of the failure when it
    /// is negative), clears the `u16` `0x00d4e370`, sets `g_OpDebugMode`,
    /// prints the debug memory address (`dbgPrintf(0x00b51748,
    /// 0x02322780)`), `op_init_draw()`, clears the bytes `0x010d4cf8`,
    /// `0x00d5249f`, `0x00d5249e`, sets `0x0230fd38` = 1 and
    /// `g_pppStopStatus` = 0, `pppInitEnv(0x010d4cfc, 0, 0x00fd24f0,
    /// 0x100000)`, points `0x010d4d1c`..`d24` at `0x00d4ca40`, `0x00d4db40`,
    /// `0x00d4dd40`, and returns `pppOtFunc_InitEnv(g_pppOtList)`. Callees
    /// through `call`.
    pub fn ppp_init_part_render<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let args = [
            sym(mem, 0x00e5_24a8),
            0xe012,
            sym(mem, 0x0076_0080),
            sym(mem, 0x0075_ff00),
            sym(mem, 0x0076_0020),
        ];
        if call(mem, sym(mem, 0x0076_b2d0), &args).cast_signed() < 0 {
            call(mem, sym(mem, 0x0062_fbf0), &[sym(mem, 0x00b5_19ec)]);
        }
        mem.write(sym(mem, 0x00d4_e370), 0u16);
        crate::op::bridge::DEBUG_MODE.set(mem, 1);
        let text = [sym(mem, 0x00b5_1748), sym(mem, 0x0232_2780)];
        call(mem, sym(mem, 0x0062_f500), &text);
        call(mem, sym(mem, 0x007e_c410), &[]);
        for at in [0x010d_4cf8, 0x00d5_249f, 0x00d5_249e] {
            mem.write(sym(mem, at), 0u8);
        }
        mem.write(sym(mem, 0x0230_fd38), 1u32);
        crate::ppp::bridge::STOP.set(mem, 0);
        let env = [sym(mem, 0x010d_4cfc), 0, sym(mem, 0x00fd_24f0), 0x10_0000];
        call(mem, sym(mem, 0x0071_69d0), &env);
        for (at, to) in [
            (0x010d_4d1c, 0x00d4_ca40),
            (0x010d_4d20, 0x00d4_db40),
            (0x010d_4d24, 0x00d4_dd40),
        ] {
            let v = sym(mem, to);
            mem.write(sym(mem, at), v);
        }
        call(mem, sym(mem, 0x0071_6b70), &[sym(mem, 0x0230_fff0)])
    }

    // ---- the trail rings (`CopyFields_007e5f30` sets one up) ----
    //
    // A ring: `u16 +2` flags (0x8000: keep the lengths), `u16 +4` its size
    // `n`, `u16 +6` the head, `u16 +8` the points held, `u16 +0xa` a life,
    // and `n` 16-byte points from `+0x10` (x, y, z, and `w` the length of
    // the trail from the head to that point).

    /// The slot offset `(i % n) << 4` the pushes use, cut to 16 bits
    /// (`movzx` after the shift); 0 when `n` is 0.
    fn slot16(i: u32, n: u32) -> u32 {
        (i.checked_rem(n).unwrap_or(0) << 4) & 0xffff
    }

    /// Copies the 16 bytes at `src` to `dst`, a word at a time.
    fn copy4(mem: &mut impl Mem, dst: u32, src: u32) {
        for k in 0..4 {
            let w: u32 = mem.read(src.wrapping_add(4 * k));
            mem.write(dst.wrapping_add(4 * k), w);
        }
    }

    /// The trail lengths: the head point's `w` = 0, then for each older
    /// point, `w` = the previous `w` + the distance to the next newer
    /// point (each difference, the length and the sum stored as `f32`; the
    /// squares summed as x, y, then z). The original divides by the size
    /// without a check there, so with a size of 0 and two points it faults:
    /// this stops instead.
    #[allow(clippy::many_single_char_names)]
    fn trail_lengths(mem: &mut impl Mem, ring: u32) {
        use ffx_bridge::x87::{add, load, mul, sqrt, store, sub};
        let pts = ring.wrapping_add(0x10);
        let n = u32::from(mem.read::<u16>(ring.wrapping_add(4)));
        let mut h = u32::from(mem.read::<u16>(ring.wrapping_add(6)));
        let first = h.checked_rem(n).unwrap_or(0) << 4;
        mem.write(pts.wrapping_add(first).wrapping_add(0xc), 0u32);
        let mut sum = store(0.0);
        let mut k = 1u32;
        while k < u32::from(mem.read::<u16>(ring.wrapping_add(8))) {
            let n = u32::from(mem.read::<u16>(ring.wrapping_add(4)));
            let a = pts.wrapping_add(h.checked_rem(n).unwrap_or(0) << 4);
            h = h.wrapping_sub(1) & 0xffff;
            let Some(r) = h.checked_rem(n) else { return };
            let b = pts.wrapping_add(r << 4);
            let mut dv = [0f64; 3];
            for (o, v) in (0u32..).step_by(4).zip(dv.iter_mut()) {
                let x = sub(
                    load(mem.read(a.wrapping_add(o))),
                    load(mem.read(b.wrapping_add(o))),
                );
                *v = load(store(x));
            }
            let [dx, dy, dz] = dv;
            let len2 = add(add(mul(dx, dx), mul(dy, dy)), mul(dz, dz));
            let len = store(sqrt(len2));
            sum = store(add(load(len), load(sum)));
            mem.write(b.wrapping_add(0xc), sum);
            k += 1;
        }
    }

    /// `0x007e5f80` (`op_trail_push`, ours; `(ring, point, life)`, called
    /// by `0x0081b9b0`): `n` times (the size, read each time), the head
    /// advances and the 16 bytes at `point` go into its slot; the count
    /// grows up to the size, a non-zero low half of `life` goes to `+0xa`,
    /// and with flag 0x8000 the lengths are redone. Returns the size (left
    /// in `eax`).
    #[allow(clippy::similar_names)]
    pub fn op_trail_push(mem: &mut impl Mem, ring: u32, point: u32, life: u32) -> u32 {
        let mut i = 0u32;
        loop {
            let n = u32::from(mem.read::<u16>(ring.wrapping_add(4)));
            if i >= n {
                return n;
            }
            let head = mem.read::<u16>(ring.wrapping_add(6)).wrapping_add(1);
            mem.write(ring.wrapping_add(6), head);
            let n = u32::from(mem.read::<u16>(ring.wrapping_add(4)));
            let slot = slot16(u32::from(head), n);
            copy4(mem, ring.wrapping_add(0x10).wrapping_add(slot), point);
            let held = mem.read::<u16>(ring.wrapping_add(8)).wrapping_add(1);
            if held <= mem.read::<u16>(ring.wrapping_add(4)) {
                mem.write(ring.wrapping_add(8), held);
            }
            #[allow(clippy::cast_possible_truncation)] // the low half
            let life16 = life as u16;
            if life16 != 0 {
                mem.write(ring.wrapping_add(0xa), life16);
            }
            if mem.read::<u16>(ring.wrapping_add(2)) & 0x8000 != 0 {
                trail_lengths(mem, ring);
            }
            i += 1;
        }
    }

    /// `0x007e6110` (`op_trail_add`, ours; `(ring, point, fill, life)`,
    /// called by `0x008171a0`): with `fill` 0 a push as in
    /// [`op_trail_push`] (once); else, while the ring is not full, the
    /// point goes behind the oldest (`(head - held) % n`; the first one
    /// advances the head instead) and the count grows. Then the lengths
    /// with flag 0x8000. Returns 0.
    #[allow(clippy::similar_names)]
    pub fn op_trail_add(mem: &mut impl Mem, ring: u32, point: u32, fill: u32, life: u32) -> u32 {
        let pts = ring.wrapping_add(0x10);
        if fill == 0 {
            let head = mem.read::<u16>(ring.wrapping_add(6)).wrapping_add(1);
            mem.write(ring.wrapping_add(6), head);
            let n = u32::from(mem.read::<u16>(ring.wrapping_add(4)));
            copy4(mem, pts.wrapping_add(slot16(u32::from(head), n)), point);
            let held = mem.read::<u16>(ring.wrapping_add(8)).wrapping_add(1);
            if held <= mem.read::<u16>(ring.wrapping_add(4)) {
                mem.write(ring.wrapping_add(8), held);
            }
            #[allow(clippy::cast_possible_truncation)] // the low half
            let life16 = life as u16;
            if life16 != 0 {
                mem.write(ring.wrapping_add(0xa), life16);
            }
        } else {
            let held = mem.read::<u16>(ring.wrapping_add(8));
            let n = mem.read::<u16>(ring.wrapping_add(4));
            if held < n {
                let slot = if held == 0 {
                    let head = mem.read::<u16>(ring.wrapping_add(6)).wrapping_add(1);
                    mem.write(ring.wrapping_add(6), head);
                    0
                } else {
                    let back = mem.read::<u16>(ring.wrapping_add(6)).wrapping_sub(held);
                    slot16(u32::from(back), u32::from(n))
                };
                copy4(mem, pts.wrapping_add(slot), point);
                let held = mem.read::<u16>(ring.wrapping_add(8)).wrapping_add(1);
                mem.write(ring.wrapping_add(8), held);
            }
        }
        if mem.read::<u16>(ring.wrapping_add(2)) & 0x8000 != 0 {
            trail_lengths(mem, ring);
        }
        0
    }

    /// `checkEnvMdl0434(id, n, make, rd0, rd1)` (`0x00712ec0`): the two
    /// instances `(0, id)` and `(1, id)` (`id` sign-extended into the
    /// high word) of effect 0434's environment model. `n` 0 destroys both
    /// (`graphicVFXDestroy`) and returns 0. With `make`, each is created
    /// (`graphicVFXCreate(lo, id, 10)`, then `graphicVFXCreateDataBuffer(lo,
    /// id, 1, &3n, &3n, name, &1)`) with the texture name (instance 0:
    /// `"15281_19_0_0_128_128.dds.phyre"` (0x00b50130) once after
    /// `0x00cec198` is set, which clears it, else `"15280_..."`
    /// (0x00b50150); instance 1 `"13600_19_0_0_128_64.dds.phyre"`
    /// (0x00b50170)) replaced by the one `graphicVFXIsTextureLoaded(name,
    /// found, 0, 0, 0, 0)` finds. Then for each instance in turn: no render
    /// data, or no segment-0 buffer
    /// (`graphicDynGeoMemManagerGetBufferForSegment(d, 0, [d +0x94] +4,
    /// +8)`, after `[d +0x94] +0x24` = 1), returns 0; else `rd0` / `rd1`
    /// (when not null) = the render data. Success clears
    /// `g_PolyTris0434Count` and returns 1. All callees through `call`.
    ///
    /// The locals are below `frame` (the original's frame pointer): the
    /// counts at `frame - 0x208`, `- 0x214`, `- 0x20c`, the found name at
    /// `frame - 0x204`, the name at `frame - 0x104`.
    #[allow(clippy::too_many_arguments)]
    pub fn check_env_mdl_0434<M: Mem>(
        mem: &mut M,
        id: u32,
        n: u32,
        make: u32,
        rd0: u32,
        rd1: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = sym(mem, at);
            call(mem, f, args)
        };
        if n == 0 {
            go(mem, 0x0064_5280, &[0, id]);
            go(mem, 0x0064_5280, &[1, id]);
            return 0;
        }
        let local = |off: u32| frame.wrapping_sub(off);
        let (name, found) = (local(0x104), local(0x204));
        if make != 0 {
            let flag = sym(mem, 0x00ce_c198);
            mem.write(local(0x208), n.wrapping_mul(3));
            mem.write(local(0x214), n.wrapping_mul(3));
            mem.write(local(0x20c), 1u32);
            for lo in [0, 1] {
                let (src, len) = if lo == 1 {
                    (0x00b5_0170, 30)
                } else if mem.read::<u32>(flag) != 0 {
                    mem.write(flag, 0u32);
                    (0x00b5_0130, 31)
                } else {
                    (0x00b5_0150, 31)
                };
                let src = sym(mem, src);
                for i in 0..len {
                    let b: u8 = mem.read(src + i);
                    mem.write(name + i, b);
                }
                if go(mem, 0x0064_5400, &[name, found, 0, 0, 0, 0]) != 0 {
                    let mut i = 0;
                    loop {
                        let b: u8 = mem.read(found.wrapping_add(i));
                        mem.write(name.wrapping_add(i), b);
                        i += 1;
                        if b == 0 {
                            break;
                        }
                    }
                }
                go(mem, 0x0064_4de0, &[lo, id, 10]);
                let (c0, c1, one) = (local(0x208), local(0x214), local(0x20c));
                go(mem, 0x0064_5040, &[lo, id, 1, c0, c1, name, one]);
            }
        }
        for (lo, out) in [(0, rd0), (1, rd1)] {
            let d = go(mem, 0x0064_53d0, &[lo, id]);
            if d == 0 {
                return 0;
            }
            let bufs: u32 = mem.read(d.wrapping_add(0x94));
            mem.write(bufs.wrapping_add(0x24), 1u32);
            let bufs: u32 = mem.read(d.wrapping_add(0x94));
            let (v, i): (u32, u32) = (
                mem.read(bufs.wrapping_add(4)),
                mem.read(bufs.wrapping_add(8)),
            );
            if go(mem, 0x0063_fdb0, &[d, 0, v, i]) == 0 {
                return 0;
            }
            if out != 0 {
                mem.write(out, d);
            }
        }
        mem.write(sym(mem, 0x00ce_c194), 0u32);
        1
    }

    /// `0x00712690` (`setPolyTris0434`, ours; next to `checkEnvMdl0434` in
    /// the table at `0x00c659dc`) `(id, tris, setup)`: the render data of
    /// instances `(0, id)` and `(1, id)` (0 when either is missing). With
    /// `tris` each gets one triangle more, slot `n` = `0x00cec194` (then
    /// counted up): for corner `k` (0..2) of record `tris + 0x10 + 0x30 k`
    /// (instance 1: `+0xa0`) its `+0x10..+0x18` to the positions (`[+0x94]
    /// +0xc`, 12 bytes per vertex), `+0..+0xc` to the colours (`+0x14`, 16
    /// bytes), `-0x10..-0xc` to the texture coordinates (`+0x18`, 8 bytes)
    /// and the `u16` index `3n + k` to `+0x1c`. Without, both get the unit
    /// matrix (`FFXVu0UnitMatrix`) and are made visible, then set up
    /// (`graphicVFXSetup`) with `setup`, else updated (`graphicVFXUpdate`).
    /// Returns 1. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn set_poly_tris_0434<M: Mem>(
        mem: &mut M,
        id: u32,
        tris: u32,
        setup: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let get = sym(mem, 0x0064_53d0);
        let a = call(mem, get, &[0, id]);
        if a == 0 {
            return 0;
        }
        let b = call(mem, get, &[1, id]);
        if b == 0 {
            return 0;
        }
        let count = sym(mem, 0x00ce_c194);
        if tris != 0 {
            for (data, src) in [(a, tris.wrapping_add(0x10)), (b, tris.wrapping_add(0xa0))] {
                let bufs: u32 = mem.read(data.wrapping_add(0x94));
                let buf = |mem: &M, at: u32| mem.read::<u32>(bufs.wrapping_add(at));
                let n: u32 = mem.read(count);
                for k in 0..3u32 {
                    let v = n.wrapping_mul(3).wrapping_add(k);
                    let s = src.wrapping_add(0x30 * k);
                    let pos: [u32; 3] = mem.read(s.wrapping_add(0x10));
                    mem.write(buf(mem, 0xc).wrapping_add(v.wrapping_mul(12)), pos);
                    let col: [u32; 4] = mem.read(s);
                    mem.write(buf(mem, 0x14).wrapping_add(v.wrapping_mul(16)), col);
                    let uv: [u32; 2] = mem.read(s.wrapping_sub(0x10));
                    mem.write(buf(mem, 0x18).wrapping_add(v.wrapping_mul(8)), uv);
                    #[allow(clippy::cast_possible_truncation)] // the u16 index
                    mem.write(buf(mem, 0x1c).wrapping_add(v.wrapping_mul(2)), v as u16);
                }
            }
            let n: u32 = mem.read(count);
            mem.write(count, n.wrapping_add(1));
            return 1;
        }
        let unit = sym(mem, 0x006e_d9a0);
        call(mem, unit, &[a]);
        call(mem, unit, &[b]);
        let (visible, set, update) = (
            sym(mem, 0x0064_5710),
            sym(mem, 0x0064_5780),
            sym(mem, 0x0064_58a0),
        );
        for lo in [0, 1] {
            call(mem, visible, &[lo, id, 1]);
            call(mem, if setup != 0 { set } else { update }, &[lo, id]);
        }
        1
    }

    /// `0x007128d0` (`setPolyQuad0434`, ours; after `setPolyTris0434` in
    /// the table at `0x00c659dc`) `(id, quad, setup)`: the render data of
    /// instance `(0, id)` only (0 when missing). With `quad`, two
    /// triangles more in slot `n` = `0x00cec194` (then counted up): the
    /// six vertices `6n..6n+5` (`[+0x94] +0xc`, 12 bytes each) take the
    /// three words at `quad +0x30`, `+0x70`, `+0x50`, `+0x70`, `+0x90`,
    /// `+0x50`, their colours (`+0x14`, 16 bytes) the four at `+0x20`,
    /// `+0x60`, `+0x40`, `+0x60`, `+0x80`, `+0x40`, and the `u16` indices
    /// (`+0x1c`) are `6n..6n+5`. Without, the unit matrix
    /// (`FFXVu0UnitMatrix`), visible, then set up (`graphicVFXSetup`) with
    /// `setup`, else updated. Returns 1. Callees through `call`.
    pub fn set_poly_quad_0434<M: Mem>(
        mem: &mut M,
        id: u32,
        quad: u32,
        setup: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const POS: [u32; 6] = [0x30, 0x70, 0x50, 0x70, 0x90, 0x50];
        const COL: [u32; 6] = [0x20, 0x60, 0x40, 0x60, 0x80, 0x40];
        let get = sym(mem, 0x0064_53d0);
        let d = call(mem, get, &[0, id]);
        if d == 0 {
            return 0;
        }
        if quad == 0 {
            let unit = sym(mem, 0x006e_d9a0);
            call(mem, unit, &[d]);
            let visible = sym(mem, 0x0064_5710);
            call(mem, visible, &[0, id, 1]);
            let f = sym(mem, if setup != 0 { 0x0064_5780 } else { 0x0064_58a0 });
            call(mem, f, &[0, id]);
            return 1;
        }
        let bufs: u32 = mem.read(d.wrapping_add(0x94));
        let buf = |mem: &M, at: u32| mem.read::<u32>(bufs.wrapping_add(at));
        let count = sym(mem, 0x00ce_c194);
        let n: u32 = mem.read(count);
        for (k, (p, c)) in (0u32..).zip(POS.into_iter().zip(COL)) {
            let v = n.wrapping_mul(6).wrapping_add(k);
            let pos: [u32; 3] = mem.read(quad.wrapping_add(p));
            mem.write(buf(mem, 0xc).wrapping_add(v.wrapping_mul(12)), pos);
            let col: [u32; 4] = mem.read(quad.wrapping_add(c));
            mem.write(buf(mem, 0x14).wrapping_add(v.wrapping_mul(16)), col);
        }
        for k in 0..6u32 {
            let v = n.wrapping_mul(6).wrapping_add(k);
            #[allow(clippy::cast_possible_truncation)] // the u16 index
            mem.write(buf(mem, 0x1c).wrapping_add(v.wrapping_mul(2)), v as u16);
        }
        let n: u32 = mem.read(count);
        mem.write(count, n.wrapping_add(1));
        1
    }

    /// `0x007fa890` (`update0706NoiseRenderData`, ours; from `0x007fa750`)
    /// `(lo, hi, x)`: with the instance's render data `d` and its segment-0
    /// buffer (`graphicDynGeoMemManagerGetBufferForSegment(d, 0, [d +0x94]
    /// +4, +8)`; 0 without either), fills 448 quads (0x5400 bytes of
    /// positions): quad `i` spans `c` to `c' = f32(c + 0.00223214...)`
    /// (`c` from 0): corners `x` -1/1, `y` `f32((c - 0.5) x 2)` and
    /// `f32((c' - 0.5) x 2)`, `z` -0.9999; texture `u` = `r` and `f32(r +
    /// 1)` with `r` = `op_random(x)` (through `fcall`), `v` = `c` and `c'`;
    /// colours 0.5; indices `4i`, `4i+1`, `4i+2`, `4i+1`, `4i+2`, `4i+3`.
    /// Then the unit matrix (`FFXVu0UnitMatrix(d)`), `d +0x80` = 0, a
    /// ±9999999 box at `+0x98..+0xac`, `graphicVFXSetVisible(lo, hi, 1)`
    /// and `0x012e9640` = 1. Returns 1.
    #[allow(clippy::many_single_char_names)]
    pub fn update_0706_noise_render_data<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        x: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let d = call(mem, sym(mem, 0x0064_53d0), &[lo, hi]);
        if d == 0 {
            return 0;
        }
        let bufs: u32 = mem.read(d.wrapping_add(0x94));
        let (nv, ni) = (
            mem.read(bufs.wrapping_add(4)),
            mem.read(bufs.wrapping_add(8)),
        );
        if call(mem, sym(mem, 0x0063_fdb0), &[d, 0, nv, ni]) == 0 {
            return 0;
        }
        let half: f64 = mem.read(sym(mem, 0x00b9_2220));
        let two: f64 = mem.read(sym(mem, 0x00b9_2270));
        let step: f64 = mem.read(sym(mem, 0x00b5_83f8));
        let flip: u32 = mem.read(sym(mem, 0x00b9_2340));
        let near: u32 = mem.read(sym(mem, 0x00b4_7ca0));
        let colour: u32 = mem.read(sym(mem, 0x00b9_2194));
        let one = 1.0f32.to_bits();
        let mut c = 0u32;
        let rnd = sym(mem, 0x007e_6700);
        // fld/fstp of the argument into op_random's slot
        let x = ffx_bridge::x87::round_trip(f32::from_bits(x)).to_bits();
        for i in 0..448u32 {
            let bufs: u32 = mem.read(d.wrapping_add(0x94));
            let buf = |mem: &M, at: u32| mem.read::<u32>(bufs.wrapping_add(at));
            let (uv, col, idx, pos) = (
                buf(mem, 0x18).wrapping_add(0x20 * i),
                buf(mem, 0x14).wrapping_add(0x40 * i),
                buf(mem, 0x1c).wrapping_add(0xc * i),
                buf(mem, 0xc).wrapping_add(0x30 * i),
            );
            let r = store(fcall(mem, rnd, &[x]));
            for (at, v) in [(0x18, flip), (0, flip), (0x24, one), (0xc, one)] {
                mem.write(pos.wrapping_add(at), v);
            }
            for at in [0x2c, 0x20, 0x14, 8] {
                mem.write(pos.wrapping_add(at), near);
            }
            let cf = load(c);
            let y0 = store(mul(sub(cf, half), two));
            mem.write(pos.wrapping_add(0x10), y0);
            mem.write(pos.wrapping_add(4), y0);
            let n = add(cf, step);
            let y1 = store(mul(sub(n, half), two));
            mem.write(pos.wrapping_add(0x28), y1);
            mem.write(pos.wrapping_add(0x1c), y1);
            mem.write(uv.wrapping_add(0x10), r);
            mem.write(uv, r);
            let r1 = store(add(load(r), 1.0));
            mem.write(uv.wrapping_add(0x18), r1);
            mem.write(uv.wrapping_add(8), r1);
            mem.write(uv.wrapping_add(0xc), c);
            mem.write(uv.wrapping_add(4), c);
            let next = store(n);
            mem.write(uv.wrapping_add(0x1c), next);
            mem.write(uv.wrapping_add(0x14), next);
            mem.write(col, [colour; 16]);
            #[allow(clippy::cast_possible_truncation)] // 16-bit indices
            let b = (i * 4) as u16;
            for (k, v) in [(0u32, 0u16), (2, 2), (4, 2), (1, 1), (3, 1), (5, 3)] {
                mem.write(idx.wrapping_add(2 * k), b.wrapping_add(v));
            }
            c = next;
        }
        call(mem, sym(mem, 0x006e_d9a0), &[d]);
        mem.write(d.wrapping_add(0x80), 0u32);
        let lo_box: u32 = mem.read(sym(mem, 0x00b5_1660));
        let hi_box: u32 = mem.read(sym(mem, 0x00b5_1658));
        mem.write(d.wrapping_add(0x98), [lo_box; 3]);
        mem.write(d.wrapping_add(0xa4), [hi_box; 3]);
        call(mem, sym(mem, 0x0064_5710), &[lo, hi, 1]);
        mem.write(sym(mem, 0x012e_9640), 1u32);
        1
    }

    /// `0x007faad0` (`update0706AccRenderData_007faad0`, ours; from
    /// `0x007fa800`) `(lo, hi)`: [`update_0706_noise_render_data`]'s twin
    /// for the 0706 accumulation strip. With the instance's render data `d`
    /// and its segment-0 buffer (0 without either), fills 448 strips of
    /// two quads (0x60 bytes of positions each): strip `i` spans `c` to
    /// `c'` (`c` from 0, `c'` = `f32(c + 0.00223214...)` through an `f64`):
    /// corners `x` -1, 0 and 1, `y` `f32((c - 0.5) x 2)` and `f32((c' -
    /// 0.5) x 2)` (from the unrounded sum), `z` -0.9999; texture `u` = `r`
    /// and `f32(r + 1)` for each quad with its own `r` = `op_random(1)`
    /// (through `fcall`), `v` = `c` and `f32(c + 1/256)`; colours (0.375,
    /// 0.375, 0.375, 0.5); indices `8i + (0, 1, 2, 1, 2, 3, 4, 5, 6, 5, 6,
    /// 7)`. Then the unit matrix (`FFXVu0UnitMatrix(d)`), `d +0x80` = 0, a
    /// ±9999999 box at `+0x98..+0xac` and `graphicVFXSetVisible(lo, hi,
    /// 1)`. Returns 1.
    #[allow(clippy::many_single_char_names)]
    pub fn update_0706_acc_render_data<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let d = call(mem, sym(mem, 0x0064_53d0), &[lo, hi]);
        if d == 0 {
            return 0;
        }
        let bufs: u32 = mem.read(d.wrapping_add(0x94));
        let (nv, ni) = (
            mem.read(bufs.wrapping_add(4)),
            mem.read(bufs.wrapping_add(8)),
        );
        if call(mem, sym(mem, 0x0063_fdb0), &[d, 0, nv, ni]) == 0 {
            return 0;
        }
        let half: f64 = mem.read(sym(mem, 0x00b9_2220));
        let two: f64 = mem.read(sym(mem, 0x00b9_2270));
        let step: f64 = mem.read(sym(mem, 0x00b5_83f8));
        let k256: f64 = mem.read(sym(mem, 0x00b5_0198));
        let flip: u32 = mem.read(sym(mem, 0x00b9_2340));
        let near: u32 = mem.read(sym(mem, 0x00b4_7ca0));
        let grey: u32 = mem.read(sym(mem, 0x00b4_936c));
        let alpha: u32 = mem.read(sym(mem, 0x00b9_2194));
        let (one, zero) = (1.0f32.to_bits(), 0u32);
        let rnd = sym(mem, 0x007e_6700);
        let mut c = 0u32;
        for i in 0..448u32 {
            let bufs: u32 = mem.read(d.wrapping_add(0x94));
            let buf = |mem: &M, at: u32| mem.read::<u32>(bufs.wrapping_add(at));
            let (pos, uv, col, idx) = (
                buf(mem, 0xc).wrapping_add(0x60 * i),
                buf(mem, 0x18).wrapping_add(0x40 * i),
                buf(mem, 0x14).wrapping_add(0x80 * i),
                buf(mem, 0x1c).wrapping_add(0x18 * i),
            );
            let cf = load(c);
            let y0 = store(mul(sub(cf, half), two));
            let n = add(cf, step);
            let y1 = store(mul(sub(n, half), two));
            // (x, y, z) of the eight corners
            let corners = [
                [flip, y0, near],
                [zero, y0, near],
                [flip, y1, near],
                [zero, y1, near],
                [zero, y0, near],
                [one, y0, near],
                [zero, y1, near],
                [one, y1, near],
            ];
            mem.write(pos, corners);
            let v1 = store(add(cf, k256));
            for q in [0u32, 0x20] {
                let r = store(fcall(mem, rnd, &[one]));
                let r1 = store(add(load(r), 1.0));
                mem.write(uv.wrapping_add(q), [r, c, r1, c, r, v1, r1, v1]);
            }
            mem.write(col, [[grey, grey, grey, alpha]; 8]);
            #[allow(clippy::cast_possible_truncation)] // 16-bit indices
            let b = (i * 8) as u16;
            let quads = [0u16, 1, 2, 1, 2, 3, 4, 5, 6, 5, 6, 7].map(|k| b.wrapping_add(k));
            mem.write(idx, quads);
            // the next edge through the f64 slot
            c = store(n);
        }
        call(mem, sym(mem, 0x006e_d9a0), &[d]);
        mem.write(d.wrapping_add(0x80), 0u32);
        let lo_box: u32 = mem.read(sym(mem, 0x00b5_1660));
        let hi_box: u32 = mem.read(sym(mem, 0x00b5_1658));
        mem.write(d.wrapping_add(0x98), [lo_box; 3]);
        mem.write(d.wrapping_add(0xa4), [hi_box; 3]);
        call(mem, sym(mem, 0x0064_5710), &[lo, hi, 1]);
        1
    }

    /// `0x007fa800` (`opus_0706_acc_update`, ours) `(eff)`: the 0706
    /// accumulation strip's renderer instance `(2, 0x2c2)`: without render
    /// data it is made (`create0706AccRenderDataBuffer(2, 0x2c2, the
    /// 64-bit texture at eff)`, then `graphicVFXSetup`; either failing
    /// gives `graphicVFXDestroy`, whose eax is returned); then
    /// `update0706AccRenderData(2, 0x2c2)` and `graphicVFXDttUpdate(2,
    /// 0x2c2)`, whose eax is returned. All callees through `call`.
    pub fn opus_0706_acc_update<M: Mem>(
        mem: &mut M,
        eff: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const ID: [u32; 2] = [2, 0x2c2];
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = sym(mem, at);
            call(mem, f, args)
        };
        if go(mem, 0x0064_53d0, &ID) == 0 {
            let tex: [u32; 2] = mem.read(eff);
            if go(mem, 0x007f_a5c0, &[ID[0], ID[1], tex[0], tex[1]]) == 0
                || go(mem, 0x0064_5780, &ID) == 0
            {
                return go(mem, 0x0064_5280, &ID);
            }
        }
        go(mem, 0x007f_aad0, &ID);
        go(mem, 0x0064_5330, &ID)
    }

    /// `opp_0706_cmd_007fa6d0(obj, cmd)` (`0x007fa6d0`, name ours): the
    /// `opus_0706` effect's command by the object's kind (`u16 obj +0x21a &
    /// 0xf000`): 0x1000 `opus_0706_acc_update(op_oef2_a_get(oef, 2, i16
    /// cmd[1]), i16 cmd[2])` with the part's oef2 (`obj +0x360[rec +0x1e]`,
    /// `rec` = `obj +0x220`), `cmd + 6`; 0x2000 `opus_0706_noise_update(rec,
    /// i16 cmd[1])`, `cmd + 4`; else `cmd`. Callees through `call` (the
    /// updates take only their first word).
    pub fn opp_0706_cmd<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = sym(mem, at);
            call(mem, f, args)
        };
        let rec: u32 = mem.read(obj.wrapping_add(0x220));
        let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x360 + 4 * slot));
        let half =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        match mem.read::<u16>(obj.wrapping_add(0x21a)) & 0xf000 {
            0x1000 => {
                let n = half(mem, 2);
                let eff = go(mem, 0x007e_3810, &[oef, 2, n]);
                let k = half(mem, 4);
                go(mem, 0x007f_a800, &[eff, k]);
                cmd.wrapping_add(6)
            }
            0x2000 => {
                let k = half(mem, 2);
                go(mem, 0x007f_a750, &[rec, k]);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `opu_fake_env_model_draw_007117c0(id, a1..a7, slot)` (`0x007117c0`,
    /// name ours): the fake environment model of instance `(slot, id +
    /// (slot >> 31))`, for `slot` 0..2 and no skipped frame
    /// (`g_GraphicSkipFrame`), else it returns `slot`. With render data:
    /// `updateVFXRenderData_FakeEnvModel` with the nine arguments, then
    /// `graphicVFXUpdate`, whose `eax` is returned; without,
    /// `initVFXRenderDataBuffer_FakeEnvModel` with them (its `eax`).
    /// Callees through `call`.
    pub fn opu_fake_env_model_draw<M: Mem>(
        mem: &mut M,
        args: [u32; 9],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = sym(mem, at);
            call(mem, f, args)
        };
        let slot = args[8];
        // `cdq; add; adc`: the id less one for a negative slot
        let hi = args[0].wrapping_add((slot.cast_signed() >> 31).cast_unsigned());
        let skip: u32 = mem.read(sym(mem, 0x012f_b7d0));
        if skip != 0 || slot > 2 {
            return slot;
        }
        if go(mem, 0x0064_53d0, &[slot, hi]) != 0 {
            go(mem, 0x0071_cf90, &args);
            go(mem, 0x0064_58a0, &[slot, hi])
        } else {
            go(mem, 0x0071_6620, &args)
        }
    }

    /// `0x007fa750` (`opus_0706_noise_update`, ours) `(eff)`: the 0706
    /// noise's renderer instance `(1, 0x2c2)`. With render data, the
    /// update scale is `eff`'s `f32 +0x40` / 4096 / 16 x 12; without, the
    /// instance is made (`create0706NoiseRenderDataBuffer`, then
    /// `graphicVFXSetup`; either failing gives `graphicVFXDestroy` and
    /// returns its eax) and the scale is `+0x40` / 4096. Then
    /// `update0706NoiseRenderData(1, 0x2c2, scale)` (rounded to `f32`) and
    /// `graphicVFXDttUpdate(1, 0x2c2)`, whose eax is returned. All callees
    /// through `call`.
    pub fn opus_0706_noise_update<M: Mem>(
        mem: &mut M,
        eff: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        const ID: [u32; 2] = [1, 0x2c2];
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = sym(mem, at);
            call(mem, f, args)
        };
        let made = go(mem, 0x0064_53d0, &ID) != 0;
        if !made && (go(mem, 0x007f_a510, &ID) == 0 || go(mem, 0x0064_5780, &ID) == 0) {
            return go(mem, 0x0064_5280, &ID);
        }
        let mut s = mul(load(mem.read(eff.wrapping_add(0x40))), 0.000_244_140_625);
        if made {
            s = mul(mul(s, 0.0625), 12.0);
        }
        go(mem, 0x007f_a890, &[ID[0], ID[1], store(s)]);
        go(mem, 0x0064_5330, &ID)
    }
}
