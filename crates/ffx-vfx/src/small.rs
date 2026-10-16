//! The op/opus layer's one-line helpers: word and vector copies, 64-bit
//! masks, float steps, the MSVC container accessors the opus trees use,
//! and similar leaves.

/// The helpers over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::x87;

    fn rf(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
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
}
