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

/// The helpers over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::x87;

    fn rf(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
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
