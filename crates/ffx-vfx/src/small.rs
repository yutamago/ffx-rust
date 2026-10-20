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
}
