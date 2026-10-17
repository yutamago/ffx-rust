//! The character envelope (skinning): vertex lists transformed by a bone's
//! matrix (`cdc_Mul3x4`) into the model's vertex buffer, in thousandths
//! (`sg_calcenvelope*`). The vertex lists come one after another: a count,
//! then the entries.

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // vertices, weights, indices
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_bridge::{Global, Host, Mem};

    use crate::scratch::{SCRATCH, Scratch};

    /// The VU0 accumulator `sg_calcenvelope_normW`'s translated code uses
    /// (`ACC.xyz`, three `f32`).
    pub const VU0_ACC: u32 = 0x00c8_f790;
    /// `g_ChrPacketState` (`u16`).
    pub const CHR_PACKET_STATE: Global<u16> = Global::new(0x0130_366c);
    /// The `u16` beside it that `Ch_EffectPacketCreateMain` sets.
    pub const CHR_PACKET_WORD: Global<u16> = Global::new(0x0130_3684);

    /// `Sg_SetChrPacketState(v)` (`0x00845c30`): `g_ChrPacketState = v`.
    pub fn set_chr_packet_state(mem: &mut impl Mem, v: u16) {
        CHR_PACKET_STATE.set(mem, v);
    }

    /// `Sg_SetChrPacketWord(v)` (`0x00845c40`): the `u16` at `0x01303684`.
    pub fn set_chr_packet_word(mem: &mut impl Mem, v: u16) {
        CHR_PACKET_WORD.set(mem, v);
    }

    /// `sg_Daddiu(dst, src, imm)` (`0x00845cc0`): the 64-bit `src` plus the
    /// sign-extended `imm` (MIPS `daddiu`); returns the low word.
    pub fn daddiu(mem: &mut impl Mem, dst: u32, src: u32, imm: u32) -> u32 {
        let x: u64 = mem.read(src);
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
        let r = x.wrapping_add(i64::from(imm as i32) as u64);
        mem.write(dst, r);
        #[allow(clippy::cast_possible_truncation)]
        let low = r as u32;
        low
    }

    /// `sg_SetFloat(dst, x)` (`0x00845cf0`): `x` through the x87 (a
    /// signalling NaN comes out quiet); returns `dst`.
    pub fn set_float(mem: &mut impl Mem, dst: u32, x: f32) -> u32 {
        mem.write(dst, x87::round_trip(x));
        dst
    }

    /// `sg_Sll(dst, src, n)` (`0x00845d70`): `src << (n & 31)` (MIPS
    /// `sll`); returns `dst`.
    pub fn sll(mem: &mut impl Mem, dst: u32, src: u32, n: u32) -> u32 {
        let x: u32 = mem.read(src);
        mem.write(dst, x << (n & 31));
        dst
    }

    /// `v * s` component by component as the three helpers compute it:
    /// `x` and `y` multiply registers, `z` is `s * [v.z]`; each plus
    /// `add[i]` (a memory operand) when given.
    fn scaled(mem: &impl Mem, v: u32, s: f32, add: Option<[f32; 3]>) -> [f32; 3] {
        use x87::Reg;
        let c = |k: u32| mem.read::<f32>(v + 4 * k);
        let s = Reg::load(s);
        let p = [Reg::load(c(0)) * s, Reg::load(c(1)) * s, s.mul_m32(c(2))];
        match add {
            None => p.map(Reg::store),
            Some(a) => [
                (p[0] + Reg::load(a[0])).store(),
                p[1].add_m32(a[1]).store(),
                p[2].add_m32(a[2]).store(),
            ],
        }
    }

    /// `sg_VmulScalar(dst, v, s)` (`0x00845e80`): `dst = v s`; returns
    /// `dst`.
    pub fn vmul_scalar(mem: &mut impl Mem, dst: u32, v: u32, s: f32) -> u32 {
        let r = scaled(mem, v, s, None);
        for (k, x) in (0u32..).zip(r) {
            mem.write(dst + 4 * k, x);
        }
        dst
    }

    /// `sg_VmaddScalar(dst, v, -, -, s)` (`0x00845e40`): `dst = ACC + v s`
    /// (VU0 `vmadd`); returns `dst`.
    pub fn vmadd_scalar(mem: &mut impl Mem, dst: u32, v: u32, s: f32) -> u32 {
        let acc = mem.rebase(VU0_ACC);
        // each component reads its accumulator lane after the previous
        // store, as the original
        for k in 0..3u32 {
            let a: [f32; 3] = mem.read(acc);
            let r = scaled(mem, v, s, Some(a));
            mem.write(dst + 4 * k, r[k as usize]);
        }
        dst
    }

    /// `sg_VmaddaScalar(-, v, -, s)` (`0x00845df0`): `ACC += v s` (VU0
    /// `vmadda`); returns `v`.
    pub fn vmadda_scalar(mem: &mut impl Mem, v: u32, s: f32) -> u32 {
        let acc = mem.rebase(VU0_ACC);
        vmadd_scalar(mem, acc, v, s);
        v
    }

    /// 1000 (`f64`).
    pub const THOUSAND: Global<f64> = Global::new(0x00b0_dba0);

    /// `cdc_Mul3x4(dst, m, v)` on the vertex `x, y, z` (each through
    /// `fld`/`fstp`): the transformed `f32` x, y, z.
    fn transform(mem: &mut impl Mem, m: u32, v: [u32; 3]) -> [f32; 3] {
        let v = v.map(|b| x87::round_trip(f32::from_bits(b)));
        let mut s = Scratch::new(mem);
        s.write(SCRATCH + 0x20, v);
        ffx_system::math::bridge::mul3x4(&mut s, SCRATCH, m, SCRATCH + 0x20);
        s.read(SCRATCH)
    }

    fn st(x: f64) -> f32 {
        f32::from_bits(x87::store(x))
    }

    fn ld(x: f32) -> f64 {
        x87::load(x.to_bits())
    }

    /// `sg_calcenvelopeW_asm_opt(lists, out, m)` (`0x006ee270`): three
    /// lists after one another. Rigid vertices (16 bytes: `x, y, z`,
    /// index) set `out[index]` to the transformed vertex x 1000 and w 1.0;
    /// first weighted vertices (20 bytes: `x, y, z`, weight, index) set it
    /// to the transformed vertex x weight x 1000 (each product stored as
    /// `f32`), w 1.0; further weighted vertices add that to `out[index]`
    /// (w + 0.0). Returns the end of the lists (where the next would
    /// start).
    pub fn calc_envelope_w_opt(mem: &mut impl Mem, lists: u32, out: u32, m: u32) -> u32 {
        let k = THOUSAND.get(mem);
        let slot = |i: u32| out.wrapping_add(i << 4);
        let mut p = lists;
        let n: u32 = mem.read(p);
        p = p.wrapping_add(4);
        for _ in 0..n {
            let v: [u32; 3] = mem.read(p);
            let i: u32 = mem.read(p.wrapping_add(0xc));
            p = p.wrapping_add(0x10);
            let r = transform(mem, m, v);
            let o = r.map(|x| st(x87::mul(ld(x), k)));
            mem.write(slot(i), [o[0], o[1], o[2], 1.0]);
        }
        for add in [false, true] {
            let n: u32 = mem.read(p);
            p = p.wrapping_add(4);
            for _ in 0..n {
                let v: [u32; 3] = mem.read(p);
                let w = ld(x87::round_trip(mem.read(p.wrapping_add(0xc))));
                let i: u32 = mem.read(p.wrapping_add(0x10));
                p = p.wrapping_add(0x14);
                let r = transform(mem, m, v);
                let q = r.map(|x| st(x87::mul(ld(st(x87::mul(ld(x), w))), k)));
                if add {
                    let cur: [f32; 4] = mem.read(slot(i));
                    let s = [0, 1, 2].map(|j| st(x87::add(ld(cur[j]), ld(q[j]))));
                    let sw = st(x87::add(ld(cur[3]), 0.0));
                    mem.write(slot(i), [s[0], s[1], s[2], sw]);
                } else {
                    mem.write(slot(i), [q[0], q[1], q[2], 1.0]);
                }
            }
        }
        p
    }

    /// 255 (`f64`).
    pub const TWO_FIFTY_FIVE: Global<f64> = Global::new(0x00b1_70c0);

    /// A packed vertex (`i16` x, y, z in thousandths) through
    /// `cdc_Mul3x4`: each coordinate `v / 1000` stored as `f32`.
    fn transform_packed(mem: &mut impl Mem, m: u32, at: u32, k: f64) -> [f32; 3] {
        let v: [i16; 3] = mem.read(at);
        let v = v.map(|c| x87::store(x87::div(f64::from(c), k)));
        transform(mem, m, v)
    }

    /// `sg_calcenvelope_asm_opt(lists, out, m)` (`0x006eee40`): the packed
    /// form. Rigid vertices (`u16` count, 2 bytes, then 8-byte entries:
    /// `i16` x, y, z / 1000, `u16` index) set `out[index]` to the
    /// transformed vertex x 1000, w 1.0. Then groups of first weighted
    /// vertices (`u16` count, `0xffff` ending them; `u16` base; 8-byte
    /// entries: `i16` x, y, z, `u8` weight / 255, `u8` index + base x 256)
    /// set `out[index]` to transformed x weight x 1000 (each product an
    /// `f32`), w 1.0. After 2 more bytes, groups of further weighted
    /// vertices (the same form) add to `out[index]`, w 1.0. Returns the end
    /// of the lists.
    pub fn calc_envelope_opt(mem: &mut impl Mem, lists: u32, out: u32, m: u32) -> u32 {
        let k = THOUSAND.get(mem);
        let c255 = TWO_FIFTY_FIVE.get(mem);
        let slot = |i: u32| out.wrapping_add(i << 4);
        let n = u32::from(mem.read::<u16>(lists));
        let mut p = lists.wrapping_add(4);
        for _ in 0..n {
            let i = u32::from(mem.read::<u16>(p.wrapping_add(6)));
            let r = transform_packed(mem, m, p, k);
            p = p.wrapping_add(8);
            let o = r.map(|x| st(x87::mul(ld(x), k)));
            mem.write(slot(i), [o[0], o[1], o[2], 1.0]);
        }
        for add in [false, true] {
            if add {
                // the first group's count follows 2 more bytes
                p = p.wrapping_add(2);
            }
            loop {
                let n = u32::from(mem.read::<u16>(p));
                p = p.wrapping_add(2);
                if n == 0xffff {
                    break;
                }
                let base = u32::from(mem.read::<u16>(p)) << 8;
                p = p.wrapping_add(2);
                for _ in 0..n {
                    let wt = u32::from(mem.read::<u8>(p.wrapping_add(6)));
                    let i = u32::from(mem.read::<u8>(p.wrapping_add(7))).wrapping_add(base);
                    let w = ld(st(x87::div(f64::from(wt), c255)));
                    let r = transform_packed(mem, m, p, k);
                    p = p.wrapping_add(8);
                    let q = r.map(|x| st(x87::mul(ld(st(x87::mul(ld(x), w))), k)));
                    let s = if add {
                        let cur: [f32; 3] = mem.read(slot(i));
                        [0, 1, 2].map(|j| st(x87::add(ld(q[j]), ld(cur[j]))))
                    } else {
                        q
                    };
                    mem.write(slot(i), [s[0], s[1], s[2], 1.0]);
                }
            }
        }
        p
    }

    /// `user_malloc`, called as the game's.
    pub const USER_MALLOC: u32 = 0x0068_7020;
    /// 0.0001 (`f64`, a weight's unit).
    pub const WEIGHT_UNIT: Global<f64> = Global::new(0x00b3_eb50);
    /// Where [`calc_envelope_w`] builds its list (unmapped addresses above
    /// the bridges' scratch, as the original's 32 KB stack frame).
    const LIST_FRAME: u32 = 0xfff0_0000;

    /// `sg_calcenvelopeW_asm(lists, out, mask, m)` (`0x006ed9e0`): the
    /// source form. Rigid vertices (`u16` count, 2 bytes, 8-byte entries:
    /// `i16` x, y, z in thousandths, `u16` index), then two weighted lists
    /// (`u16` count, 10-byte entries: x, y, z, index, `u16` weight in
    /// 1/10000), keeping only vertices whose `mask[index]` is set (all
    /// without a mask). The kept vertices go into a
    /// [`calc_envelope_w_opt`] list (floats: `v / 1000`, `w * 0.0001`, each
    /// stored as `f32`), which runs on them as it is built; with a mask the
    /// list is returned as a new block (`user_malloc`, then copied), else
    /// 0.
    pub fn calc_envelope_w(h: &mut impl Host, lists: u32, out: u32, mask: u32, m: u32) -> u32 {
        let k = THOUSAND.get(h);
        let unit = WEIGHT_UNIT.get(h);
        let coord = |c: i16| x87::store(x87::div(f64::from(c), k));
        let kept =
            |h: &_, i: u16| mask == 0 || Mem::read::<u8>(h, mask.wrapping_add(u32::from(i))) != 0;
        let mut list: Vec<u8> = vec![];
        let put = |list: &mut Vec<u8>, w: u32| list.extend(w.to_le_bytes());
        // rigid
        let n = u32::from(h.read::<u16>(lists));
        let mut p = lists.wrapping_add(4);
        let count_at = list.len();
        put(&mut list, 0);
        let mut kept_n = 0u32;
        for _ in 0..n {
            let v: [i16; 3] = h.read(p);
            let i: u16 = h.read(p.wrapping_add(6));
            p = p.wrapping_add(8);
            if kept(h, i) {
                kept_n += 1;
                for c in v {
                    put(&mut list, coord(c));
                }
                put(&mut list, u32::from(i));
            }
        }
        list[count_at..count_at + 4].copy_from_slice(&kept_n.to_le_bytes());
        // weighted, twice
        for _ in 0..2 {
            let n = u32::from(h.read::<u16>(p));
            p = p.wrapping_add(2);
            let count_at = list.len();
            put(&mut list, 0);
            let mut kept_n = 0u32;
            for _ in 0..n {
                let v: [i16; 3] = h.read(p);
                let i: u16 = h.read(p.wrapping_add(6));
                let w: u16 = h.read(p.wrapping_add(8));
                p = p.wrapping_add(10);
                if kept(h, i) {
                    kept_n += 1;
                    for c in v {
                        put(&mut list, coord(c));
                    }
                    put(&mut list, x87::store(x87::mul(f64::from(w), unit)));
                    put(&mut list, u32::from(i));
                }
            }
            list[count_at..count_at + 4].copy_from_slice(&kept_n.to_le_bytes());
        }
        let len = u32::try_from(list.len()).unwrap_or(u32::MAX);
        {
            let mut f = ffx_bridge::frame::Frame::new(h, LIST_FRAME, list.len());
            f.write_bytes(LIST_FRAME, &list);
            calc_envelope_w_opt(&mut f, LIST_FRAME, out, m);
        }
        if mask == 0 {
            return 0;
        }
        let at = h.rebase(USER_MALLOC);
        let dst = h.call(at, &[len]);
        h.write_bytes(dst, &list);
        dst
    }

    /// `sg_calcenvelope_asm(lists, out, mask, m)` (`0x006ee570`): the packed
    /// lists of [`calc_envelope_opt`] keeping only vertices whose
    /// `mask[index]` is set (all without a mask): rigid entries and group
    /// entries are copied as they are, each group header with its count
    /// replaced by the kept count, the terminators with the 2 bytes after
    /// them; [`calc_envelope_opt`] runs on the kept vertices. With a mask the
    /// new list is returned as a new block (`user_malloc`, then copied),
    /// else 0. The 2 bytes after the rigid count are the original's
    /// uninitialised stack; they are 0 here.
    pub fn calc_envelope(h: &mut impl Host, lists: u32, out: u32, mask: u32, m: u32) -> u32 {
        let kept = |h: &_, i: u32| mask == 0 || Mem::read::<u8>(h, mask.wrapping_add(i)) != 0;
        let mut list: Vec<u8> = vec![0; 4];
        let n = h.read::<u16>(lists);
        let mut p = lists.wrapping_add(4);
        let mut kept_n = 0u16;
        for _ in 0..n {
            let e: [u8; 8] = h.read(p);
            let i = u32::from(h.read::<u16>(p.wrapping_add(6)));
            p = p.wrapping_add(8);
            if kept(h, i) {
                kept_n = kept_n.wrapping_add(1);
                list.extend(e);
            }
        }
        list[0..2].copy_from_slice(&kept_n.to_le_bytes());
        for _ in 0..2 {
            loop {
                let head: [u8; 4] = h.read(p);
                let n = u16::from_le_bytes([head[0], head[1]]);
                if n == 0xffff {
                    // the terminator and the 2 bytes after it
                    list.extend(head);
                    p = p.wrapping_add(4);
                    break;
                }
                let base = u32::from(u16::from_le_bytes([head[2], head[3]])) << 8;
                let at = list.len();
                list.extend(head);
                p = p.wrapping_add(4);
                let mut kept_n = 0u16;
                for _ in 0..n {
                    let e: [u8; 8] = h.read(p);
                    p = p.wrapping_add(8);
                    if kept(h, u32::from(e[7]) + base) {
                        kept_n = kept_n.wrapping_add(1);
                        list.extend(e);
                    }
                }
                list[at..at + 2].copy_from_slice(&kept_n.to_le_bytes());
            }
        }
        let len = u32::try_from(list.len()).unwrap_or(u32::MAX);
        {
            let mut f = ffx_bridge::frame::Frame::new(h, LIST_FRAME, list.len());
            f.write_bytes(LIST_FRAME, &list);
            calc_envelope_opt(&mut f, LIST_FRAME, out, m);
        }
        if mask == 0 {
            return 0;
        }
        let at = h.rebase(USER_MALLOC);
        let dst = h.call(at, &[len]);
        h.write_bytes(dst, &list);
        dst
    }

    /// The emulated R5900 registers `sg_calcenvelope_normW` uses (16
    /// bytes each, `g_Ee_Gpr + 16 n`): r0, a0 (the list pointer), a1 (the
    /// output), a2 (the matrix), t0..t6, t8 (the list's end).
    pub const EE_R0: u32 = 0x00c8_f508;
    pub const EE_A0: u32 = 0x00c8_f548;
    pub const EE_A1: u32 = 0x00c8_f558;
    pub const EE_A2: u32 = 0x00c8_f568;
    pub const EE_T: [u32; 7] = [
        0x00c8_f588,
        0x00c8_f598,
        0x00c8_f5a8,
        0x00c8_f5b8,
        0x00c8_f5c8,
        0x00c8_f5d8,
        0x00c8_f5e8,
    ];
    pub const EE_T8: u32 = 0x00c8_f688;
    /// The emulated VU0 registers it uses: a word before Q (`0x00c8f784`),
    /// the accumulator, the blended vertex (`0x00c8f7a0`), the source
    /// vertex, the matrix rows (`0x00c8f7d0..f0`), the vertex as floats,
    /// the result, the normal scale (`0x00c8f840`).
    pub const VU_I: u32 = 0x00c8_f784;
    pub const VU_BLEND: u32 = 0x00c8_f7a0;
    pub const VU_SRC: u32 = 0x00c8_f7c0;
    pub const VU_ROWS: [u32; 3] = [0x00c8_f7d0, 0x00c8_f7e0, 0x00c8_f7f0];
    pub const VU_VERTEX: u32 = 0x00c8_f810;
    pub const VU_RESULT: u32 = 0x00c8_f820;
    pub const VU_SCALE: u32 = 0x00c8_f840;
    /// `g_Vu0_Vf00`.
    pub const VU_VF00: u32 = 0x00c0_a004;
    /// The word the translated code saves the VU word in while it loads
    /// the scale.
    pub const SAVED_WORD: u32 = 0x0130_36a8;
    /// The normal scale, 0.0001 (`f32` in `.rdata`).
    pub const NORMAL_SCALE: Global<f32> = Global::new(0x00b9_2154);

    /// The skinning step shared by the three lists: `VU_VERTEX` times the
    /// matrix rows (`sg_VmulScalar` by x, `sg_VmaddaScalar` by y,
    /// `sg_VmaddScalar` by z into `dst`).
    fn rows(mem: &mut impl Mem, r: &impl Fn(u32) -> u32, dst: u32) {
        let v: [f32; 4] = mem.read(r(VU_VERTEX));
        vmul_scalar(mem, r(VU0_ACC), r(VU_ROWS[0]), v[0]);
        let v: [f32; 4] = mem.read(r(VU_VERTEX));
        vmadda_scalar(mem, r(VU_ROWS[1]), v[1]);
        let v: [f32; 4] = mem.read(r(VU_VERTEX));
        vmadd_scalar(mem, dst, r(VU_ROWS[2]), v[2]);
    }

    /// `t_k` = the 64-bit sign- or zero-extension of a list `u16`.
    fn load_half(mem: &mut impl Mem, reg: u32, at: u32, signed: bool) {
        let h: u16 = mem.read(at);
        let v = if signed {
            i64::from(h.cast_signed()).cast_unsigned()
        } else {
            u64::from(h)
        };
        mem.write(reg, v);
    }

    /// The header of the second and third lists: the count at `p` (into
    /// t0), t2 = count 8 + 2 count, a0 = `p + 2`, t8 = a0 + t2. True when
    /// the count is r0's (none).
    fn list_header(mem: &mut impl Mem, r: &impl Fn(u32) -> u32, p: u32) -> bool {
        use ffx_vfx::op::bridge::op_add64;
        let [t0, _, t2, ..] = EE_T.map(r);
        load_half(mem, t0, p, false);
        sll(mem, t2, t0, 3);
        let n: u32 = mem.read(t0);
        op_add64(mem, t2, t2, n);
        op_add64(mem, t2, t2, n);
        let a0 = r(EE_A0);
        mem.write(a0, p.wrapping_add(2));
        mem.write(a0 + 4, 0u32);
        let len: u32 = mem.read(t2);
        op_add64(mem, r(EE_T8), a0, len);
        let r0: [u32; 2] = mem.read(r(EE_R0));
        let hi: u32 = mem.read(t0 + 4);
        n == r0[0] && hi == r0[1]
    }

    /// Whether a0 (both words) has reached t8.
    fn at_end(mem: &impl Mem, r: &impl Fn(u32) -> u32) -> bool {
        let a: [u32; 2] = mem.read(r(EE_A0));
        let e: [u32; 2] = mem.read(r(EE_T8));
        a == e
    }

    /// One weighted entry's common part (10 bytes at `p`: `i16` x, y, z,
    /// `u16` weight, `u16` index): the vertex into `VU_VERTEX` with its w
    /// scaled by `VU_SCALE`'s x, skinned into `VU_BLEND`; t4 becomes the
    /// output slot's address.
    fn weighted(mem: &mut impl Mem, r: &impl Fn(u32) -> u32, p: u32) {
        use ffx_vfx::op::bridge::{copy16, op_vec_w_scale};
        let [t0, t1, t2, t3, t4, t5, t6] = EE_T.map(r);
        load_half(mem, t0, p, true);
        load_half(mem, t1, p + 2, true);
        load_half(mem, t2, p + 4, true);
        // the third list fills t3 and t4 after the first copy, the second
        // before: the copy reads neither
        copy_fields_7ecb20(mem, t5, t1, t0);
        load_half(mem, t3, p + 6, false);
        load_half(mem, t4, p + 8, false);
        copy_fields_7ecb20(mem, t6, t3, t2);
        copy_fields_845d00(mem, t0, t6, t5);
        copy16(mem, r(VU_SRC), t0);
        ffx_vfx::vec::bridge::ivec4_to_fvec4(mem, r(VU_VERTEX), r(VU_SRC));
        let s: f32 = mem.read(r(VU_SCALE));
        op_vec_w_scale(mem, r(VU_VERTEX), r(VU_VERTEX), s);
        rows(mem, r, r(VU_BLEND));
        sll(mem, t4, t4, 4);
        let out: u32 = mem.read(r(EE_A1));
        daddiu(mem, t4, t4, out);
    }

    /// `CopyFields_007ecb20(dst, a, b)`: MIPS `pextlw` (`dst` = `b.x,
    /// a.x, b.y, a.y`).
    fn copy_fields_7ecb20(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let a: [u32; 4] = mem.read(a);
        let b: [u32; 4] = mem.read(b);
        mem.write(dst + 4, a[0]);
        mem.write(dst + 8, b[1]);
        mem.write(dst + 0xc, a[1]);
        mem.write(dst, b[0]);
    }

    /// `sg_CopyFields_00845d00(dst, a, b)`: MIPS `pcpyld` (`dst` = `b.x,
    /// b.y, a.x, a.y`).
    fn copy_fields_845d00(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let a: [u32; 4] = mem.read(a);
        let b: [u32; 4] = mem.read(b);
        mem.write(dst + 8, a[0]);
        mem.write(dst + 4, b[1]);
        mem.write(dst + 0xc, a[1]);
        mem.write(dst, b[0]);
    }

    /// `sg_calcenvelope_normW(lists, out, m)` (`0x00845ec0`): the vertex
    /// normals' envelope, the PS2's R5900 and VU0 code translated register
    /// by register (the emulated registers in memory, every step as the
    /// original leaves it).
    ///
    /// a0..a2 take the arguments; the matrix's three rows go to
    /// `VU_ROWS`, 0.0001 through the VU word into t0 and from there to
    /// `VU_SCALE`; the result starts as `vf00`. Three lists follow, each a
    /// `u16` count then entries:
    /// - rigid (8 bytes: `i16` x, y, z, `u16` index): `out[index]` =
    ///   `VU_ROWS x (x, y, z)` (the w of `VU_RESULT` left as it was);
    /// - first weights (10 bytes: x, y, z, weight, index): `out[index]` =
    ///   `VU_ROWS x v` scaled by `weight 0.0001`;
    /// - further weights: that added to `out[index]`.
    ///
    /// Entries are 16 bytes in `out` (`index 16`). The loops end when a0
    /// meets t8 (both words), as the translated `bne` does.
    #[allow(clippy::too_many_lines)]
    pub fn calc_envelope_norm_w(mem: &mut impl Mem, lists: u32, out: u32, m: u32) {
        use ffx_vfx::op::bridge::{copy16, op_add64, op_word_copy, opus_vec3_scale, ppp_vec4_copy};
        let base = mem.rebase(0);
        let r = move |a: u32| a.wrapping_add(base);
        let [t0, _, _, t3, t4, t5, _] = EE_T.map(r);
        let a0 = r(EE_A0);
        mem.write(a0, lists);
        mem.write(r(EE_A1), out);
        mem.write(r(EE_A2), m);
        for (k, row) in (0u32..).zip(VU_ROWS) {
            copy16(mem, r(row), m.wrapping_add(0x10 * k));
        }
        let (word, saved) = (r(VU_I), r(SAVED_WORD));
        let w: u32 = mem.read(word);
        mem.write(saved, w);
        set_float(mem, word, NORMAL_SCALE.get(mem));
        op_word_copy(mem, t0, word);
        let w: u32 = mem.read(saved);
        mem.write(word, w);
        copy16(mem, r(VU_SCALE), t0);
        ppp_vec4_copy(mem, r(VU_RESULT), r(VU_VF00));
        // the rigid list
        load_half(mem, t0, lists, false);
        sll(mem, t0, t0, 3);
        let n: u32 = mem.read(t0);
        let t8 = r(EE_T8);
        op_add64(mem, t8, a0, n);
        let end: u32 = mem.read(t8);
        mem.write(t8, [end.wrapping_add(4), 0]);
        mem.write(a0, [lists.wrapping_add(4), 0]);
        let mut p = lists.wrapping_add(4);
        if !at_end(mem, &r) {
            loop {
                load_half(mem, t0, p, true);
                load_half(mem, r(EE_T[1]), p + 2, true);
                load_half(mem, r(EE_T[2]), p + 4, true);
                copy_fields_7ecb20(mem, t5, r(EE_T[1]), t0);
                copy_fields_845d00(mem, t0, r(EE_T[2]), t5);
                copy16(mem, r(VU_SRC), t0);
                load_half(mem, t3, p + 6, false);
                crate::vec::bridge::int_vec_to_float(mem, r(VU_VERTEX), r(VU_SRC));
                let a: u32 = mem.read(a0);
                mem.write(a0, [a.wrapping_add(8), 0]);
                rows(mem, &r, r(VU_RESULT));
                sll(mem, t3, t3, 4);
                let o: u32 = mem.read(r(EE_A1));
                daddiu(mem, t3, t3, o);
                let dst: u32 = mem.read(t3);
                copy16(mem, dst, r(VU_RESULT));
                p = mem.read(a0);
                if at_end(mem, &r) {
                    break;
                }
            }
        }
        // the first weights
        if !list_header(mem, &r, p) {
            loop {
                p = mem.read(a0);
                weighted(mem, &r, p);
                let a: u32 = mem.read(a0);
                mem.write(a0, [a.wrapping_add(0xa), 0]);
                let v: [f32; 4] = mem.read(r(VU_VERTEX));
                opus_vec3_scale(mem, r(VU_RESULT), r(VU_BLEND), v[3]);
                let dst: u32 = mem.read(t4);
                copy16(mem, dst, r(VU_RESULT));
                if at_end(mem, &r) {
                    break;
                }
            }
        }
        // the further weights
        p = mem.read(a0);
        if !list_header(mem, &r, p) {
            loop {
                p = mem.read(a0);
                weighted(mem, &r, p);
                let src: u32 = mem.read(t4);
                copy16(mem, r(VU_SRC), src);
                let v: [f32; 4] = mem.read(r(VU_VERTEX));
                opus_vec3_scale(mem, r(VU_RESULT), r(VU_BLEND), v[3]);
                let k: [f32; 4] = mem.read(r(VU_SRC));
                ffx_vfx::vec::bridge::vec3_add_scalars(
                    mem,
                    r(VU_RESULT),
                    r(VU_RESULT),
                    [k[0], k[1], k[2]],
                );
                let a: u32 = mem.read(a0);
                mem.write(a0, [a.wrapping_add(0xa), 0]);
                let dst: u32 = mem.read(t4);
                copy16(mem, dst, r(VU_RESULT));
                if at_end(mem, &r) {
                    break;
                }
            }
        }
    }
}
