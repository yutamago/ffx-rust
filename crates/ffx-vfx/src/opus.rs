//! The "opus" effect objects (`opus_*`, and the `opu_*`/`opp_*` script
//! commands that step them).
//!
//! An opus object has a draw list at `+0x1c` (`i16` part numbers, ended by
//! a negative one) and its 0x100-byte parts at `+0x20`; a part's group
//! byte is at `+0x1e` and its draw kind at `+0xbb`. The script commands
//! take the object and the command's address and return the next
//! command's.

/// `opus_encount0`'s particle grid, `nz` planes of `ny` rows of `nx`
/// particles in the order the original lays them out: each is `[x, y, z,
/// 1, t, 0, 0, 1]`, `x` from 0 down by 64 along a row, `y` from 0 down by
/// 64 per row, `z` from 0 down by 64 per plane, and `t` from 2112 up by 64
/// per particle. Each step is the x87's: the `f32` value plus 64.0 in 53
/// bits, rounded to `f32` where the original stores it.
pub fn encount0_grid(nx: u16, ny: u16, nz: u16, mut put: impl FnMut([f32; 8])) {
    #[allow(clippy::cast_possible_truncation)] // the original's fstp
    let step = |v: f32, d: f64| (f64::from(v) + d) as f32;
    let (mut t, mut z) = (2112.0f32, 0.0f32);
    for _ in 0..nz {
        let mut y = 0.0f32;
        for _ in 0..ny {
            let mut x = 0.0f32;
            for _ in 0..nx {
                put([x, y, z, 1.0, t, 0.0, 0.0, 1.0]);
                t = step(t, 64.0);
                x = step(x, -64.0);
            }
            y = step(y, -64.0);
        }
        z = step(z, -64.0);
    }
}

/// The objects where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use crate::et::bridge::{EFF, OPUS_BATTLE};
    use crate::x87::{ADD, MUL, Reg};

    /// `g_OpusWeapons`: the four weapon objects.
    pub const WEAPONS: Global<u32> = Global::new(0x0119_fea0);
    /// `g_OpusMonsters`: the eight monster objects.
    pub const MONSTERS: Global<u32> = Global::new(0x0119_fed0);

    /// `opus_encount0_init(obj)` (`0x007f8480`): `f32 +0x20` = 16, `+0x24`
    /// = 32, `+0x28` = 0, and the 0x20-byte particles from `+0x30` laid
    /// out by [`super::encount0_grid`] for the `u16` counts `+2` (x), `+4`
    /// (y) and `+6` (z). Returns `eax` as left: the z count (0 for none).
    pub fn opus_encount0_init(mem: &mut impl Mem, obj: u32) -> u32 {
        mem.write(obj.wrapping_add(0x20), 16.0f32);
        mem.write(obj.wrapping_add(0x24), 32.0f32);
        mem.write(obj.wrapping_add(0x28), 0.0f32);
        let count = |at: u32| mem.read::<u16>(obj.wrapping_add(at));
        let (nx, ny, nz) = (count(2), count(4), count(6));
        let mut at = obj.wrapping_add(0x30);
        super::encount0_grid(nx, ny, nz, |p| {
            mem.write(at, p);
            at = at.wrapping_add(0x20);
        });
        u32::from(nz)
    }

    /// `opus_mimic_check(chr)` (`0x0080f630`): the `g_EtChr` record of
    /// character `chr` (`opus_get_chr_status_index(chr, 0, 0)`, the record
    /// before the table for none) when its number (`+4`) is one of the
    /// mimic models 0x10fb, 0x10fc, 0x10fd or 0x1100; else 0.
    pub fn opus_mimic_check(mem: &mut impl Mem, chr: u32) -> u32 {
        let i = crate::et::bridge::opus_get_chr_status_index(mem, chr, 0, 0, &mut |_, _, _| 0);
        let rec = crate::et::bridge::CHR
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(0x20));
        let no: u32 = mem.read(rec.wrapping_add(4));
        if matches!(no, 0x10fb..=0x10fd | 0x1100) {
            rec
        } else {
            0
        }
    }

    /// `Ch_GetRootGlobalMatrix(chr, out)`, called through `call`.
    const CH_GET_ROOT_GLOBAL_MATRIX: u32 = 0x0083_3f80;

    /// `opus_chara_root_m_get(out, chr)` (`0x00805400`): a character's
    /// original matrix (`Ch_GetOrgMatrix`: the pointer at `chr + 0x10c`)
    /// copied to `out` when it has one (`op_m_copy`), else its root
    /// global matrix (`Ch_GetRootGlobalMatrix(chr, out)`). Returns `eax`
    /// as left: the original matrix's address, or the callee's result.
    pub fn opus_chara_root_m_get<M: Mem>(
        mem: &mut M,
        out: u32,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let org: u32 = mem.read(chr.wrapping_add(0x10c));
        if org != 0 {
            crate::op::bridge::op_m_copy(mem, out, org);
            return org;
        }
        let f = Global::<u32>::new(CH_GET_ROOT_GLOBAL_MATRIX).ptr(mem).addr;
        call(mem, f, &[chr, out])
    }

    /// `Ch_OptposGetElement(chr, i, out)`, called through `call`.
    const CH_OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
    /// `Ch_ElemGetBaseMatrix(chr, e, out)`, called through `call`.
    const CH_ELEM_GET_BASE_MATRIX: u32 = 0x0083_3b80;

    /// `opus_get_bone_matrix(out, chr, bone)` (`0x00805430`): the base
    /// matrix of a character's element into `out` (`Ch_ElemGetBaseMatrix`).
    /// With bit 0x8000 the bone is an option position (`bone & 0xff`):
    /// `Ch_OptposGetElement` gives its element and an offset (16 bytes at
    /// `frame`, the original's local), and `out + 0x30` becomes that offset
    /// through the matrix (`op_apply_m_w1`). Otherwise the emulated EE
    /// register `g_Ee_Gpr` (`0x00c8f508..`, 16 bytes) is copied to `out +
    /// 0x40`. Returns `eax` as left: `0x00c8f93c` (`op_apply_m_w1`'s last
    /// load), or `0x00c8f514`; no caller reads it.
    pub fn opus_get_bone_matrix<M: Mem>(
        mem: &mut M,
        out: u32,
        chr: u32,
        bone: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = Global::<u32>::new(CH_ELEM_GET_BASE_MATRIX).ptr(mem).addr;
        if bone & 0x8000 != 0 {
            let f = Global::<u32>::new(CH_OPTPOS_GET_ELEMENT).ptr(mem).addr;
            let e = call(mem, f, &[chr, bone & 0xff, frame]);
            call(mem, base, &[chr, e, out]);
            crate::op::bridge::op_apply_m_w1(mem, out.wrapping_add(0x30), out, frame);
            return Global::<u32>::new(0x00c8_f93c).get(mem);
        }
        call(mem, base, &[chr, bone, out]);
        let gpr: [u32; 4] = mem.read(Global::<u32>::new(0x00c8_f508).ptr(mem).addr);
        mem.write(out.wrapping_add(0x40), gpr);
        gpr[3]
    }

    /// `MsGetChrScale(chr, out)`, called through `call`.
    const MS_GET_CHR_SCALE: u32 = 0x0079_5720;
    /// `Ch_GetScaleX/Y/Z(h)` and `Ch_GetDataScale(h)`, called through
    /// `fcall` (they return in `ST0`).
    const CH_GET_SCALE: [u32; 3] = [0x0082_ad30, 0x0082_ad40, 0x0082_ad50];
    const CH_GET_DATA_SCALE: u32 = 0x0082_ab60;

    /// `opus_rt_m_chara(m, chr, k, angles, trans, battle)` (`0x008062c0`):
    /// `m` becomes a character's scale matrix, then turned and moved.
    /// The scale (`frame - 0x20`, the original's 16-byte local) is the
    /// battle character's (`MsGetChrScale(chr, local)`) when `battle` is set
    /// and `chr < 0x20` (signed), else `Ch_GetScaleX/Y/Z` of its handle
    /// (`g_EtChrHandles[chr]`); it is scaled by `Ch_GetDataScale * k *
    /// 0.001f * (1/256)` (53 bits, stored as an `f32`, `op_xyz_scale`) and
    /// set as `m`'s diagonal (`op_unit_m_scale2`). Then `angles` and
    /// `trans` (either may be 0) through `op_rt_m`, `op_rot_m` or
    /// `op_trans_m`. No caller reads `eax`.
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)] // the original's six, the local and two calls
    pub fn opus_rt_m_chara<M: Mem>(
        mem: &mut M,
        m: u32,
        chr: u32,
        k: f32,
        angles: u32,
        trans: u32,
        battle: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use crate::op::bridge as op;
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let v = frame.wrapping_sub(0x20);
        #[allow(clippy::cast_possible_wrap)] // cmp eax, 0x20; jge
        if battle == 0 || chr as i32 >= 0x20 {
            for (i, f) in CH_GET_SCALE.into_iter().enumerate() {
                let f = Global::<u32>::new(f).ptr(mem).addr;
                // fstp m32
                let x = ffx_bridge::x87::store(fcall(mem, f, &[h]));
                #[allow(clippy::cast_possible_truncation)] // 0..3
                mem.write(v.wrapping_add(4 * i as u32), x);
            }
        } else {
            let f = Global::<u32>::new(MS_GET_CHR_SCALE).ptr(mem).addr;
            call(mem, f, &[chr, v]);
        }
        let f = Global::<u32>::new(CH_GET_DATA_SCALE).ptr(mem).addr;
        // an fld m32 in the callee: exact
        let ds = f32::from_bits(ffx_bridge::x87::store(fcall(mem, f, &[h])));
        let milli: f64 = mem.read(Global::<u32>::new(0x00b4_7c68).ptr(mem).addr);
        let byte: f64 = mem.read(Global::<u32>::new(0x00b5_0198).ptr(mem).addr);
        let s = Reg::load_f32(ds)
            .mem(MUL, k)
            .constant(MUL, milli)
            .constant(MUL, byte)
            .store();
        // fstp [ebp - 0x24]; fld; fstp [esp]
        let s = crate::x87::load_f32(s);
        op::op_xyz_scale(mem, v, v, s.to_bits());
        op::op_unit_m_scale2(mem, m, v);
        match (angles, trans) {
            (0, 0) => {}
            (0, t) => {
                op::op_trans_m(mem, m, m, t);
            }
            (a, 0) => op::op_rot_m(mem, m, m, a),
            (a, t) => op::op_rt_m_zyx(mem, m, m, a, t),
        }
    }

    /// `0x0080e160(pk, mats, obj, unit, tex)` and `0x0080e230` (`clear`
    /// false): the 0x50-byte `op1_mdc_draw` packet header at `pk` for an
    /// opus model unit: `+0` the unit's `u16 +0xc0`, `+4` `obj +0x28`, `u16
    /// +8` the unit's `+0xc2`, `+0xc` 0, `+0x10` 0, `+0x18` RGB 0x80 with the
    /// unit's `+0xcb` as A, `+0x1c` the unit's `+0xc8` with its top byte
    /// cleared, `+0x20`/`+0x24`/`+0x28`/`+0x2c` the four 0x40-byte matrices
    /// of `mats` (`+0xc0`, `+0`, `+0x40`, `+0x80`), `+0x34` `obj +0x54`,
    /// `+0x38` 0, `+0x40` the unit's `+0xc4`, `+0x44` 0, `u16 +0x4c`
    /// 0x8080. `0x0080e160` also clears `+0x14` and `+0x3c`. Returns the
    /// model, `op_oef2_a_get(obj +0x60 [unit +0x1e], 4, tex)`, or 0 when
    /// `tex` is -1.
    #[allow(clippy::many_single_char_names)] // packet fields
    pub fn opus_mdc_packet_set(
        mem: &mut impl Mem,
        pk: u32,
        mats: u32,
        obj: u32,
        unit: u32,
        tex: u32,
        clear: bool,
    ) -> u32 {
        let at = |o: u32| pk.wrapping_add(o);
        let c: u32 = mem.read(unit.wrapping_add(0xc8));
        let a: u8 = mem.read(unit.wrapping_add(0xcb));
        mem.write(at(0x1c), c & 0x00ff_ffff);
        mem.write(at(0x18), [0x80u8, 0x80, 0x80, a]);
        if clear {
            mem.write(at(0x14), 0u32);
        }
        mem.write(at(0x10), 0u32);
        let n: u16 = mem.read(unit.wrapping_add(0xc0));
        mem.write(at(0), u32::from(n));
        let v: u32 = mem.read(obj.wrapping_add(0x28));
        mem.write(at(4), v);
        mem.write(at(0xc), 0u32);
        let v: u32 = mem.read(unit.wrapping_add(0xc4));
        mem.write(at(0x40), v);
        mem.write(at(0x38), 0u32);
        if clear {
            mem.write(at(0x3c), 0u32);
        }
        mem.write(at(0x4c), 0x8080u16);
        let v: u32 = mem.read(obj.wrapping_add(0x54));
        mem.write(at(0x34), v);
        let v: u16 = mem.read(unit.wrapping_add(0xc2));
        mem.write(at(8), v);
        mem.write(at(0x28), mats.wrapping_add(0x40));
        mem.write(at(0x2c), mats.wrapping_add(0x80));
        mem.write(at(0x24), mats);
        mem.write(at(0x20), mats.wrapping_add(0xc0));
        mem.write(at(0x44), 0u32);
        if tex == u32::MAX {
            return 0;
        }
        let g = u32::from(mem.read::<u8>(unit.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x60 + 4 * g));
        crate::op::bridge::op_oef2_a_get(mem, oef, 4, tex)
    }

    /// `opus_no_get(obj, unit, no)`, called through `call`.
    const OPUS_NO_GET: u32 = 0x0080_1060;

    /// `0x0080e900(pk, mats, obj, unit, nos)`: the four lights of an
    /// `op1_mdc_draw` packet (`0x0080e030`). Both light matrices, `pk +0x24`
    /// (colours) and `pk +0x28` (positions), start as four rows of the
    /// emulated EE register `g_Ee_Gpr` (`0x00c8f508`, 16 bytes); then for
    /// each unit number `u16 nos[1..=4]` that is not 0, the light unit
    /// (`opus_no_get(obj, unit, no)`, through `call`) gives row `i`: its
    /// position (`+0x90..`) with `w` = `+0x4c * 0.0625`, and its colour from
    /// `+0xc8`: R, G, B each `+0x3c * (1/32768) * c` (53 bits), A `a *
    /// (1/256)`. Last, `pk +0x4c` and `+0x4d` are `ftol(unit +0x30) / 2` and
    /// `ftol(unit +0x4c) / 2`, at most 0x80 and negative as 0. No caller
    /// reads `eax`.
    #[allow(clippy::many_single_char_names)] // packet fields
    pub fn opus_mdc_light_set<M: Mem>(
        mem: &mut M,
        pk: u32,
        obj: u32,
        unit: u32,
        nos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let col: u32 = mem.read(pk.wrapping_add(0x24));
        let pos: u32 = mem.read(pk.wrapping_add(0x28));
        let gpr = Global::<u32>::new(0x00c8_f508).ptr(mem).addr;
        for m in [col, pos] {
            for row in 0..4u32 {
                for w in 0..4u32 {
                    let v: u32 = mem.read(gpr + 4 * w);
                    mem.write(m.wrapping_add(0x10 * row + 4 * w), v);
                }
            }
        }
        let get = Global::<u32>::new(OPUS_NO_GET).ptr(mem).addr;
        let k16: f64 = mem.read(Global::<u32>::new(0x00b9_21e0).ptr(mem).addr);
        let k15: f64 = mem.read(Global::<u32>::new(0x00b1_70b0).ptr(mem).addr);
        let k8: f64 = mem.read(Global::<u32>::new(0x00b5_0198).ptr(mem).addr);
        for i in 0..4u32 {
            let no: u16 = mem.read(nos.wrapping_add(2 + 2 * i));
            if no == 0 {
                continue;
            }
            let l = call(mem, get, &[obj, unit, u32::from(no)]);
            let (p, c) = (pos.wrapping_add(0x10 * i), col.wrapping_add(0x10 * i));
            let xyz: [u32; 3] = mem.read(l.wrapping_add(0x90));
            mem.write(p, xyz);
            let r: f32 = mem.read(l.wrapping_add(0x4c));
            mem.write(
                p.wrapping_add(0xc),
                Reg::load_f32(r).constant(MUL, k16).store(),
            );
            let rgba: [u8; 4] = mem.read(l.wrapping_add(0xc8));
            let s: f32 = mem.read(l.wrapping_add(0x3c));
            // (s * 1/32768) at 53 bits, times the channel (exact as f32)
            let sk = Reg::load_f32(s).constant(MUL, k15);
            for (j, &ch) in rgba[..3].iter().enumerate() {
                let v = sk.mem(MUL, f32::from(ch)).store();
                #[allow(clippy::cast_possible_truncation)] // 0..3
                mem.write(c.wrapping_add(4 * j as u32), v);
            }
            let a = Reg::load_f32(f32::from(rgba[3])).constant(MUL, k8).store();
            mem.write(c.wrapping_add(0xc), a);
        }
        for (src, dst) in [(0x30u32, 0x4cu32), (0x4c, 0x4d)] {
            let x: f32 = mem.read(unit.wrapping_add(src));
            let v =
                ffx_bridge::x87::ftol(mem, crate::x87::Reg::load_f32(x).value()).cast_signed() / 2;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=0x80
            let b = v.clamp(0, 0x80) as u8;
            mem.write(pk.wrapping_add(dst), b);
        }
    }

    /// `opus_get_weapon_index(obj)` (`0x00801000`): its slot, or -1.
    pub fn opus_get_weapon_index(mem: &impl Mem, obj: u32) -> i32 {
        (0..4)
            .find(|&i| WEAPONS.ptr(mem).at(i).get(mem) == obj)
            .map_or(-1, u32::cast_signed)
    }

    /// `g_OpusWeaponNos`: the weapon numbers of the four weapon objects
    /// (bytes, 0xff free).
    pub const WEAPON_NOS: Global<u8> = Global::new(0x0119_fe9c);

    /// `opus_get_weapon_index_by_no(no)` (`0x00801030`): the slot whose
    /// weapon number is `no`, or -1.
    pub fn opus_get_weapon_index_by_no(mem: &impl Mem, no: u32) -> i32 {
        (0..4)
            .find(|&i| u32::from(WEAPON_NOS.ptr(mem).at(i).get(mem)) == no)
            .map_or(-1, u32::cast_signed)
    }

    /// `opus_get_weapon_by_no(no)` (`0x00800fd0`): the object of the slot
    /// whose weapon number is `no`, or 0.
    pub fn opus_get_weapon_by_no(mem: &impl Mem, no: u32) -> u32 {
        (0..4)
            .find(|&i| u32::from(WEAPON_NOS.ptr(mem).at(i).get(mem)) == no)
            .map_or(0, |i| WEAPONS.ptr(mem).at(i).get(mem))
    }

    /// `opus_get_monster_index(obj)` (`0x00800e80`): its slot, or -1 (also
    /// for null).
    pub fn opus_get_monster_index(mem: &impl Mem, obj: u32) -> i32 {
        if obj == 0 {
            return -1;
        }
        (0..8)
            .find(|&i| MONSTERS.ptr(mem).at(i).get(mem) == obj)
            .map_or(-1, u32::cast_signed)
    }

    /// `opus_draw_off(obj, part)` (`0x00800d10`): takes the first `part`
    /// (16 bits) out of the draw list.
    pub fn opus_draw_off(mem: &mut impl Mem, obj: u32, part: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original compares 16 bits
        let part = part as i16;
        let mut p = Ptr::<i16>::new(mem.read(obj.wrapping_add(0x1c)));
        loop {
            let v = p.get(mem);
            if v < 0 {
                return;
            }
            if v == part {
                break;
            }
            p = p.at(1);
        }
        loop {
            let next = p.at(1).get(mem);
            if next < 0 {
                break;
            }
            p.set(mem, next);
            p = p.at(1);
        }
        p.set(mem, -1);
    }

    /// `opus_get_osu_by_draw(obj, like, kind, any)` (`0x00800f60`): the
    /// first drawn part of draw kind `kind` (a byte) and, unless `any`, of
    /// `like`'s group; 0 if none. A null `obj` means the battle's object
    /// with its first part and `any`.
    pub fn opus_get_osu_by_draw(mem: &impl Mem, obj: u32, like: u32, kind: u32, any: u32) -> u32 {
        let (obj, like, any) = if obj == 0 {
            let b = OPUS_BATTLE.get(mem);
            (b, mem.read(b.wrapping_add(0x20)), 1)
        } else {
            (obj, like, any)
        };
        let group = mem.read::<u8>(like.wrapping_add(0x1e));
        let parts = mem.read::<u32>(obj.wrapping_add(0x20));
        let mut list = Ptr::<i16>::new(mem.read(obj.wrapping_add(0x1c)));
        #[allow(clippy::cast_possible_truncation)] // the original compares a byte
        let kind = kind as u8;
        loop {
            let n = list.get(mem);
            if n < 0 {
                return 0;
            }
            let part = (i32::from(n) << 8).cast_unsigned().wrapping_add(parts);
            if mem.read::<u8>(part.wrapping_add(0xbb)) == kind
                && (any != 0 || mem.read::<u8>(part.wrapping_add(0x1e)) == group)
            {
                return part;
            }
            list = list.at(1);
        }
    }

    /// `opus_fullauto_no_get(obj)` (`0x00800df0`): steps the `i16` at `+2`,
    /// which wraps to -0x8000 from -16 up, and returns the value before
    /// the step, zero-extended (`opu_et_unit` takes it as its number).
    pub fn opus_fullauto_no_get(mem: &mut impl Mem, obj: u32) -> u32 {
        let p = Ptr::<i16>::new(obj.wrapping_add(2));
        let old = p.get(mem);
        let v = old.wrapping_add(1);
        p.set(mem, if v >= -16 { i16::MIN } else { v });
        u32::from(old.cast_unsigned())
    }

    /// `opu_wait(obj, cmd)` (`0x0080c500`): `+0x210` = the `u16` at
    /// `+0x21a` over 512.
    pub fn opu_wait(mem: &mut impl Mem, obj: u32, cmd: u32) -> u32 {
        let v = mem.read::<u16>(obj.wrapping_add(0x21a)) >> 9;
        #[allow(clippy::cast_possible_truncation)] // below 128
        mem.write(obj.wrapping_add(0x210), v as u8);
        cmd.wrapping_add(2)
    }

    /// `opu_part_kill(host, cmd)` (`0x0080b7d0`): the host's part driver
    /// (`[host +0x350]`) is told through its function at `+0x54` to end the
    /// current unit (`host +0x220`). Returns `cmd + 2`.
    pub fn opu_part_kill<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let driver: u32 = mem.read(host.wrapping_add(0x350));
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let f: u32 = mem.read(driver.wrapping_add(0x54));
        call(mem, f, &[unit]);
        cmd.wrapping_add(2)
    }

    /// `opu_local_link(host, cmd)` (`0x00801a50`): for flag nibble 0 (the
    /// top four bits of `u16 host +0x21a`) the unit's link `u16 +0xb8` =
    /// the command's `+2` and bit 7 of its byte `+0x19` cleared; for 1 the
    /// same with the bit set; other nibbles change nothing. Returns `cmd +
    /// 4`.
    pub fn opu_local_link(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let kind = mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000;
        if kind == 0 || kind == 0x1000 {
            let v: u16 = mem.read(cmd.wrapping_add(2));
            let b: u8 = mem.read(unit.wrapping_add(0x19));
            let b = if kind == 0 { b & 0x7f } else { b | 0x80 };
            mem.write(unit.wrapping_add(0x19), b);
            mem.write(unit.wrapping_add(0xb8), v);
        }
        cmd.wrapping_add(4)
    }

    /// `opu_etc(host, cmd)` (`0x00816610`): the unit's work area at `+0xa8`
    /// plus the command's `i16 +2` takes a value from `+4` by the flag
    /// nibble: 0 a byte, 1 a `u16` (both return `cmd + 6`), 2 a word
    /// (`cmd + 8`). Other nibbles write nothing and return `cmd` (the
    /// interpreter would run it again).
    pub fn opu_etc(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let off = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let at = off.wrapping_add(unit).wrapping_add(0xa8);
        match mem.read::<u16>(host.wrapping_add(0x21a)) >> 12 {
            0 => {
                let v: u8 = mem.read(cmd.wrapping_add(4));
                mem.write(at, v);
                cmd.wrapping_add(6)
            }
            1 => {
                let v: u16 = mem.read(cmd.wrapping_add(4));
                mem.write(at, v);
                cmd.wrapping_add(6)
            }
            2 => {
                let v: u32 = mem.read(cmd.wrapping_add(4));
                mem.write(at, v);
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }

    /// `opu_jloop(host, cmd)` (`0x00816870`): the unit's loop counter
    /// (byte `+0xb4` + the flag nibble, set by `opu_sloop`) down by one;
    /// while not 0 back by the command's `i16 +2` (from `cmd`), else `cmd +
    /// 4`.
    pub fn opu_jloop(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let k = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)) >> 12);
        let at = k.wrapping_add(unit).wrapping_add(0xb4);
        let c = mem.read::<u8>(at).wrapping_sub(1);
        mem.write(at, c);
        if c == 0 {
            return cmd.wrapping_add(4);
        }
        i32::from(mem.read::<i16>(cmd.wrapping_add(2)))
            .cast_unsigned()
            .wrapping_add(cmd)
    }

    /// `opu_unit_max(host, cmd)` (`0x00817530`): the unit count `u16 host
    /// +0x334` becomes the command's `u16 +2`; each unit from the old count
    /// up to the new one (0x100 bytes from `[host +0x320]`, while the
    /// difference as an `i16` stays positive) has its four program
    /// pointers cleared. Returns `cmd + 4`.
    pub fn opu_unit_max(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let n: u16 = mem.read(cmd.wrapping_add(2));
        let old: u16 = mem.read(host.wrapping_add(0x334));
        let units: u32 = mem.read(host.wrapping_add(0x320));
        let mut at = (u32::from(old) << 8).wrapping_add(units);
        let mut left = n.wrapping_sub(old).cast_signed();
        while left > 0 {
            mem.write(at, [0u32; 4]);
            at = at.wrapping_add(0x100);
            left -= 1;
        }
        mem.write(host.wrapping_add(0x334), n);
        cmd.wrapping_add(4)
    }

    /// `0x00800ab0(dst, src)` and `0x00800af0(src, dst)`: 0x400 bytes
    /// copied word by word upward (the save and restore of a 1 KB block
    /// around `0x00826950` in `opu_motion_etc`). Return `src + 0x400`.
    pub fn copy_1k(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        for i in 0..0x100u32 {
            let v: u32 = mem.read(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), v);
        }
        src.wrapping_add(0x400)
    }

    /// The `f64` constant 1/16 (`0x00b921e0`).
    const SIXTEENTH: f64 = 0.0625;

    /// `ftol(angle) & 0xfff` and the two table entries the `opf_cir*`
    /// callbacks read: `0x00c43bf0[i + 0x400]` (= `g_pppSinTable[i]`) and
    /// `g_pppSinTable[(i + 0x400) & 0xfff]`.
    fn opf_sin_cos(mem: &impl Mem, angle: f32) -> (f32, f32) {
        let a = ffx_bridge::x87::ftol(mem, Reg::load_f32(angle).value());
        let i = (a & 0xfff) + 0x400;
        let t1 = f(mem, g(mem, 0x00c4_3bf0).wrapping_add(4 * i));
        let t2 = f(mem, g(mem, 0x00c4_4bf0).wrapping_add(4 * (i & 0xfff)));
        (t1, t2)
    }

    /// The circle of `opf_cir`, `opf_cir2` and `opf_cir_y` around the
    /// base unit `b`: angle `+0x48`, radius `+0x40`, height `+0x44`:
    /// `+0x90 = t1 r / 16 + b.x`, `+0x98 = t2 r / 16 + b.z`, `+0x94 =
    /// y / 16` plus `b.y` when `add_y`.
    fn opf_circle(mem: &mut impl Mem, unit: u32, base: u32, add_y: bool) {
        let (t1, t2) = opf_sin_cos(mem, f(mem, unit.wrapping_add(0x48)));
        let radius = f(mem, unit.wrapping_add(0x40));
        let x = Reg::load_f32(t1)
            .mem(MUL, radius)
            .constant(MUL, SIXTEENTH)
            .reg(ADD, Reg::load_f32(f(mem, base.wrapping_add(0x90))))
            .store();
        mem.write(unit.wrapping_add(0x90), x);
        let z = Reg::load_f32(t2)
            .mem(MUL, radius)
            .constant(MUL, SIXTEENTH)
            .mem(ADD, f(mem, base.wrapping_add(0x98)))
            .store();
        mem.write(unit.wrapping_add(0x98), z);
        let y = Reg::Num(SIXTEENTH).mem(MUL, f(mem, unit.wrapping_add(0x44)));
        let y = if add_y {
            y.mem(ADD, f(mem, base.wrapping_add(0x94)))
        } else {
            y
        };
        mem.write(unit.wrapping_add(0x94), y.store());
    }

    /// `opf_cir(host, ptr)` (`0x00814c90`): the unit circles the unit
    /// `opus_no_get(host +0x300, unit, u16 +0xa0)` (see [`opf_circle`]).
    /// Returns `ptr`.
    pub fn opf_cir(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let b = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        opf_circle(mem, unit, b, true);
        ptr
    }

    /// `opf_cir_y(host, ptr)` (`0x00814f00`): as [`opf_cir`] with the
    /// height not offset by the base's.
    pub fn opf_cir_y(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let b = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        opf_circle(mem, unit, b, false);
        ptr
    }

    /// `opf_cir2(host, ptr)` (`0x00814d20`): as [`opf_cir`] around the unit
    /// of `u16 +0xa0`; then with the horizontal distance `d` from it to the
    /// unit of `u16 +0xa2` (`sqrt |dx^2 + dz^2|`, each step rounded to
    /// `f32`), bit 0x2000 of `u16 +0xb0` is cleared when `+0x30 / 16 + d`
    /// (`f32`) is above `+0x40 / 16`, else set (also for a NaN).
    pub fn opf_cir2(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let n0 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let n1 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa2)));
        let base = opus_no_get(mem, list, unit, n0);
        let other = opus_no_get(mem, list, unit, n1);
        opf_circle(mem, unit, base, true);
        let dx = crate::x87::sub(
            f(mem, other.wrapping_add(0x90)),
            f(mem, base.wrapping_add(0x90)),
        );
        let dz = crate::x87::sub(
            f(mem, other.wrapping_add(0x98)),
            f(mem, base.wrapping_add(0x98)),
        );
        let (dz, dx) = (Reg::load_f32(dz), Reg::load_f32(dx));
        let d2 = dx.reg(MUL, dx).reg(ADD, dz.reg(MUL, dz)).store();
        // fabs, _CIsqrt, fstp m32
        let dist = match Reg::load_f32(d2) {
            Reg::Num(v) => Reg::Num(ffx_bridge::x87::sqrt(v.abs())),
            Reg::Nan(bits) => Reg::Nan(bits & 0x7fff_ffff),
        }
        .store();
        let reach = Reg::load_f32(f(mem, unit.wrapping_add(0x30)))
            .constant(MUL, SIXTEENTH)
            .mem(ADD, dist)
            .store();
        let lim = Reg::load_f32(f(mem, unit.wrapping_add(0x40))).constant(MUL, SIXTEENTH);
        let above =
            matches!((Reg::load_f32(reach), lim), (Reg::Num(v), Reg::Num(limit)) if v > limit);
        let fl: u16 = mem.read(unit.wrapping_add(0xb0));
        let fl = if above { fl & 0xdfff } else { fl | 0x2000 };
        mem.write(unit.wrapping_add(0xb0), fl);
        ptr
    }

    /// The circle of `opf_cir_gw` and `opf_cirz_gw`: the angle, radius and
    /// height are the unit's floats at the signed byte offsets `+0xa0`,
    /// `+0xa1` and `+0xa2` (0.0 for a 0 offset); `in_x` puts the sine
    /// term on `+0x90`/`+0x40` (else on `+0x94`/`+0x44`).
    fn opf_circle_gw(mem: &mut impl Mem, unit: u32, in_x: bool) {
        let at = |mem: &dyn Fn(u32) -> i8, o: u32| {
            i32::from(mem(unit.wrapping_add(o)))
                .cast_unsigned()
                .wrapping_add(unit)
        };
        let byte = |a: u32| mem.read::<i8>(a);
        let (pa, pb) = (at(&byte, 0xa0), at(&byte, 0xa1));
        let pc = (byte(unit.wrapping_add(0xa2)) != 0).then(|| at(&byte, 0xa2));
        let angle = crate::x87::load_f32(f(mem, pa));
        let r = crate::x87::load_f32(f(mem, pb));
        let h = pc.map_or(0.0, |p| crate::x87::load_f32(f(mem, p)));
        let (t1, t2) = opf_sin_cos(mem, angle);
        // the sine term's axis, the cosine term's axis, the height's axis
        let (s_at, c_at, h_at) = if in_x {
            (0x40u32, 0x48u32, 0x44u32)
        } else {
            (0x44, 0x40, 0x48)
        };
        let v = Reg::load_f32(t1)
            .reg(MUL, Reg::load_f32(r))
            .reg(ADD, Reg::load_f32(f(mem, unit.wrapping_add(s_at))))
            .constant(MUL, SIXTEENTH)
            .store();
        mem.write(unit.wrapping_add(s_at + 0x50), v);
        let v = Reg::load_f32(r)
            .mem(MUL, t2)
            .mem(ADD, f(mem, unit.wrapping_add(c_at)))
            .constant(MUL, SIXTEENTH)
            .store();
        mem.write(unit.wrapping_add(c_at + 0x50), v);
        let v = Reg::load_f32(f(mem, unit.wrapping_add(h_at)))
            .mem(ADD, h)
            .constant(MUL, SIXTEENTH)
            .store();
        mem.write(unit.wrapping_add(h_at + 0x50), v);
    }

    /// `opf_cir_gw(host, ptr)` (`0x00814e50`): the unit circles its own
    /// `+0x40` (x) and `+0x48` (z): `+0x90 = (t1 r + x) / 16`, `+0x98 =
    /// (r t2 + z) / 16`, `+0x94 = (+0x44 + h) / 16`, the angle, radius and
    /// height `h` read at the unit's signed byte offsets `+0xa0..+0xa2`
    /// (see [`opf_circle_gw`]). Returns `ptr`.
    pub fn opf_cir_gw(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        opf_circle_gw(mem, unit, true);
        ptr
    }

    /// `opf_cirz_gw(host, ptr)` (`0x00814fa0`): as [`opf_cir_gw`] with the
    /// circle in y and x: `+0x94` from `+0x44` and the sine, `+0x90` from
    /// `+0x40` and the cosine, the height on `+0x98` from `+0x48`.
    pub fn opf_cirz_gw(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        opf_circle_gw(mem, unit, false);
        ptr
    }

    /// `opf_sph(host, ptr)` (`0x008155b0`): the unit on a sphere around the
    /// unit `opus_no_get(host +0x300, unit, u16 +0xa0)`: the host's matrix
    /// (`+0`) = identity turned by the unit's angles `+0x30` and moved to
    /// that unit's `+0x90` (`op_unit_m`, `op_rt_m`); `host +0x40` = the
    /// unit's `+0x40` / 16 (`op_xyz_scale`) through that matrix
    /// (`op_apply_m_w1`), copied to the emulated `v0` (`0x00c8f528`) and
    /// to the unit's `+0x90..+0x98` (the first two at the byte offset
    /// `g_pppColorConShift`, which is then cleared). Returns `ptr`.
    pub fn opf_sph(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        use crate::op::bridge as op;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let base = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        crate::small::bridge::op_unit_m(mem, host);
        op::op_rt_m_zyx(
            mem,
            host,
            host,
            unit.wrapping_add(0x30),
            base.wrapping_add(0x90),
        );
        let k: u32 = mem.read(g(mem, 0x00b5_8354));
        let v = host.wrapping_add(0x40);
        op::op_xyz_scale(mem, v, unit.wrapping_add(0x40), crate::x87::load(k));
        op::op_apply_m_w1(mem, v, host, v);
        let w: [u32; 4] = mem.read(v);
        mem.write(g(mem, 0x00c8_f528), w);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        mem.write(shift.wrapping_add(unit).wrapping_add(0x90), w[0]);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        let w1: u32 = mem.read(g(mem, 0x00c8_f52c));
        mem.write(shift.wrapping_add(unit).wrapping_add(0x94), w1);
        let w2: u32 = mem.read(g(mem, 0x00c8_f530));
        crate::rand::bridge::COLOR_CON_SHIFT.set(mem, 0);
        mem.write(unit.wrapping_add(0x98), w2);
        ptr
    }

    /// `opf_spl_gw(host, ptr)` (`0x00815670`): the unit on the spline of
    /// the unit `opus_no_get(host +0x300, unit, u16 +0xa0)` (its record at
    /// `+0xbc`: `i16` point count `+0`, flag byte `+7`, points `+8`, tangents
    /// `+0xc`) at `t` = the unit's float at the signed byte offset `+0xa2`
    /// / 256 (rounded to `f32`): `op_spline_xyz` into the host's `+0`, then
    /// scaled into the unit's `+0x90` by 16 when the flag is set, else 1
    /// (`op_xyz_scale`). Returns 0.
    pub fn opf_spl_gw(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        use crate::op::bridge as op;
        let _ = ptr;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let base = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        let sp: u32 = mem.read(base.wrapping_add(0xbc));
        let n = i32::from(mem.read::<i16>(sp)).cast_unsigned();
        let off = i32::from(mem.read::<i8>(unit.wrapping_add(0xa2))).cast_unsigned();
        let c: f64 = mem.read(g(mem, 0x00b5_0198));
        let t = Reg::load_f32(f(mem, off.wrapping_add(unit)))
            .constant(MUL, c)
            .store();
        let (pts, tan): (u32, u32) = (mem.read(sp.wrapping_add(8)), mem.read(sp.wrapping_add(0xc)));
        op::op_spline_xyz(mem, host, crate::x87::load_f32(t), pts, tan, n);
        let k = if mem.read::<u8>(sp.wrapping_add(7)) != 0 {
            crate::x87::load(mem.read(g(mem, 0x00b9_231c)))
        } else {
            1.0f32.to_bits()
        };
        op::op_xyz_scale(mem, unit.wrapping_add(0x90), host, k);
        0
    }

    /// The step `opf_rel` and `opf_rel2` share, in the emulated VU
    /// registers: `0x00c8f8d0` = the unit's `+0x40..+0x4c`, `0x00c8f8f0` =
    /// the base point `from` (16 bytes), `0x00c8f900` = `to +0x90..`;
    /// `0x00c8f970` = `to - from` (x, y, z), its `w` side `0x00c8f980..98c`
    /// = 1/256 and `0x00c8f52c..534`; `restoreVf00Register`; then each of
    /// x, y, z times the unit's `+0x40`, times 1/256, plus `from`, each
    /// rounded to `f32`, into `0x00c8f970..978` and `v0` (`0x00c8f528..530`,
    /// `0x00c8f534` = `0x00c8f97c`). The first two also go to the unit's
    /// `+0x90`/`+0x94` at the byte offset `g_pppColorConShift`. Returns
    /// the new `v0` z.
    fn opf_rel_step(mem: &mut impl Mem, unit: u32, from: u32, to: u32) -> u32 {
        let k256 = 0x3b80_0000u32;
        let v0 = g(mem, 0x00c8_f528);
        mem.write(v0, k256);
        let w: [u32; 4] = mem.read(unit.wrapping_add(0x40));
        mem.write(g(mem, 0x00c8_f8d0), w);
        let w: [u32; 4] = mem.read(from);
        mem.write(g(mem, 0x00c8_f8f0), w);
        let w: [u32; 4] = mem.read(to.wrapping_add(0x90));
        mem.write(g(mem, 0x00c8_f900), w);
        let (vf, base, dst) = (
            g(mem, 0x00c8_f970),
            g(mem, 0x00c8_f8f0),
            g(mem, 0x00c8_f900),
        );
        let d0 = crate::x87::sub(f(mem, dst), f(mem, base));
        mem.write(vf, d0);
        let d1 = crate::x87::sub(f(mem, dst + 4), f(mem, base + 4));
        let t: u32 = mem.read(v0 + 4);
        mem.write(vf + 0x14, t);
        let t: u32 = mem.read(v0 + 8);
        mem.write(vf + 0x18, t);
        mem.write(vf + 4, d1);
        let d2 = crate::x87::sub(f(mem, dst + 8), f(mem, base + 8));
        let t: u32 = mem.read(v0 + 0xc);
        mem.write(vf + 0x10, k256);
        mem.write(vf + 0x1c, t);
        mem.write(vf + 8, d2);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        for scale in [g(mem, 0x00c8_f8d0), vf + 0x10] {
            let k = Reg::load(mem.read(scale));
            for i in 0..3 {
                let v = k.mem(MUL, f(mem, vf + 4 * i)).store();
                mem.write(vf + 4 * i, v);
            }
        }
        for i in 0..3 {
            let v = crate::x87::add(f(mem, vf + 4 * i), f(mem, base + 4 * i));
            mem.write(vf + 4 * i, v);
            mem.write(v0 + 4 * i, v);
        }
        let t: u32 = mem.read(vf + 0xc);
        mem.write(v0 + 0xc, t);
        let x: u32 = mem.read(vf);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        mem.write(shift.wrapping_add(unit).wrapping_add(0x90), x);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        let y: u32 = mem.read(v0 + 4);
        mem.write(shift.wrapping_add(unit).wrapping_add(0x94), y);
        mem.read(v0 + 8)
    }

    /// `opf_rel(host, ptr)` (`0x00815120`): the unit placed between the
    /// units of `u16 +0xa0` and `+0xa2` (`opus_no_get`): their `+0x90`
    /// difference times the unit's `+0x40` / 256 from the first
    /// ([`opf_rel_step`]); `+0x98` from `v0`, `g_pppColorConShift`
    /// cleared. Returns `ptr`.
    pub fn opf_rel(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let n0 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let n1 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa2)));
        let base = opus_no_get(mem, list, unit, n0);
        let other = opus_no_get(mem, list, unit, n1);
        let z = opf_rel_step(mem, unit, base.wrapping_add(0x90), other);
        crate::rand::bridge::COLOR_CON_SHIFT.set(mem, 0);
        mem.write(unit.wrapping_add(0x98), z);
        ptr
    }

    /// `opf_rel2(host, ptr)` (`0x00815350`): as [`opf_rel`] from the point
    /// the host keeps at `+0` (`+0`, `+4`, `+8` = the unit's `+0xa4`,
    /// `+0x44`, `+0x48`) toward the unit of `u16 +0xa0`, with `v0`'s `y`
    /// 0 to start. Afterwards `v0` = (old `v0` z, `0x00c8f510`, `v0` w,
    /// `0x00c8f514`), the unit's `+0x98` = that z. Returns `ptr`.
    pub fn opf_rel2(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let n0 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let other = opus_no_get(mem, host.wrapping_add(0x300), unit, n0);
        for (k, o) in [0xa4u32, 0x44, 0x48].into_iter().enumerate() {
            let v: u32 = mem.read(unit.wrapping_add(o));
            #[allow(clippy::cast_possible_truncation)] // 0..3
            mem.write(host.wrapping_add(4 * k as u32), crate::x87::load(v));
        }
        let v0 = g(mem, 0x00c8_f528);
        mem.write(v0 + 4, 0u32);
        let z = opf_rel_step(mem, unit, host, other);
        let gpr = g(mem, 0x00c8_f508);
        let (a, b): (u32, u32) = (mem.read(gpr + 8), mem.read(gpr + 0xc));
        let w: u32 = mem.read(v0 + 0xc);
        mem.write(v0 + 8, w);
        mem.write(v0 + 4, a);
        mem.write(v0 + 0xc, b);
        mem.write(v0, z);
        mem.write(unit.wrapping_add(0x98), z);
        crate::rand::bridge::COLOR_CON_SHIFT.set(mem, 0);
        ptr
    }

    /// `opu_vzero(host, cmd)` (`0x0081afa0`): clears parts of the unit's
    /// motion by the flag nibble: 0 the position `+0x30` and `+0x40`
    /// (16 bytes each) to the emulated `r0` (`g_Ee_Gpr`, `0x00c8f508`),
    /// 1 `+0x40..+0x48` and 2 `+0x30..+0x38` to 0; after those three the
    /// unit's motion callback runs (`0x00c492c8[byte +0xba]`, through
    /// `call` with `(host, cmd)`). 4 sets `+0x50..+0x8c` to `r0`, 5 zeroes
    /// `+0x60..+0x68` and `+0x80..+0x88`, 6 `+0x50..+0x58` and
    /// `+0x70..+0x78`; other nibbles change nothing. Returns `cmd + 2`.
    pub fn opu_vzero<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let gpr = g(mem, 0x00c8_f508);
        let zero3 = |mem: &mut M, at: u32| mem.write(unit.wrapping_add(at), [0u32; 3]);
        let copy = |mem: &mut M, at: u32| {
            let r0: [u32; 4] = mem.read(gpr);
            mem.write(unit.wrapping_add(at), r0);
        };
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            k @ (0 | 0x1000 | 0x2000) => {
                match k {
                    0 => {
                        copy(mem, 0x30);
                        copy(mem, 0x40);
                    }
                    0x1000 => zero3(mem, 0x40),
                    _ => zero3(mem, 0x30),
                }
                let cb = u32::from(mem.read::<u8>(unit.wrapping_add(0xba)));
                let f: u32 = mem.read(g(mem, 0x00c4_92c8) + 4 * cb);
                call(mem, f, &[host, cmd]);
            }
            0x4000 => {
                for at in [0x50, 0x60, 0x70, 0x80] {
                    copy(mem, at);
                }
            }
            0x5000 => {
                zero3(mem, 0x60);
                zero3(mem, 0x80);
            }
            0x6000 => {
                zero3(mem, 0x50);
                zero3(mem, 0x70);
            }
            _ => {}
        }
        cmd.wrapping_add(2)
    }

    /// `op_spline_xyz_table(x, pts, n)` (`0x007ea2d0`), called through
    /// `call`.
    const OP_SPLINE_XYZ_TABLE: u32 = 0x007e_a2d0;
    /// `0x007e7330(dst, a, b)`, called through `call`.
    const OP_RT_TO: u32 = 0x007e_7330;

    /// `opu_spl(host, cmd)` (`0x0081b560`): spline motion, by the flag
    /// nibble.
    /// - 0: a new spline record for the unit (`+0xbc`; the old one freed,
    ///   `op_free`/`op_malloc` on the heap `[host +0x3a8]`): `n` (`i16
    ///   cmd+2`) points of 16 bytes after a 16-byte head (`+0` = `n`,
    ///   `+8` the points, `+0xc` the knots after them, `+7` = 1 when the
    ///   `u16 cmd+6` is 0), each point the `+0x40` (or, with that flag,
    ///   `+0x90`) x, y, z of the unit `opus_no_get` gives for the `u16`s
    ///   at `cmd + i16 cmd+4`; then `op_spline_xyz_table(knots, points,
    ///   n)`. Returns `cmd + 8`.
    /// - 1: the unit's `+0x40` = the spline of the unit `u16 cmd+2` (0:
    ///   itself) at `t = +0x4c / 256` (`op_spline_xyz` into the host's
    ///   `+0`, scaled by 16 when `u16 cmd+4` is set, else 1), and `+0x90`
    ///   = `+0x40` / 16. Returns `cmd + 6`.
    /// - 2: as 1, and a second point at `t + +0x6c` (or `t + 1e-6` for a
    ///   zero `+0x6c`) into the host's `+0x10` (scaled the same only with
    ///   the flag, then / 16); `0x007e7330(host +0x40, +0x90, host +0x10)`
    ///   and the unit's matrix `+0xc0` = `host +0x280` times `host
    ///   +0x40` (`op_mul_m`). Returns `cmd + 6`.
    /// - 3: the motion callback `+0xba` = 0xa, `u16 +0xa0`/`+0xa2` =
    ///   `cmd+2`/`+4`. Returns `cmd + 6`.
    /// - Any other nibble returns `cmd`; the original also pops the
    ///   caller's `esi`, `edi` and `ebx` off the wrong stack slots there
    ///   (not reproduced).
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names,
        clippy::redundant_guards
    )]
    pub fn opu_spl<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::op::bridge as op;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let kind = mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000;
        let k256: f64 = mem.read(g(mem, 0x00b5_0198));
        let k16: u32 = mem.read(g(mem, 0x00b9_231c));
        let k16 = crate::x87::load(k16);
        let k_16: u32 = mem.read(g(mem, 0x00b5_8354));
        let k_16 = crate::x87::load(k_16);
        // the unit `u16 cmd+2` names (0: the unit itself) and its spline
        let source = |mem: &M| {
            let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
            if no == 0 {
                unit
            } else {
                opus_no_get(mem, list, unit, no)
            }
        };
        let at_t = |mem: &mut M, sp: u32, out: u32, t: f32| {
            let n = i32::from(mem.read::<i16>(sp)).cast_unsigned();
            let pts: u32 = mem.read(sp.wrapping_add(8));
            let x: u32 = mem.read(sp.wrapping_add(0xc));
            op::op_spline_xyz(mem, out, crate::x87::load_f32(t), pts, x, n);
        };
        match kind {
            0x1000 => {
                let s = source(mem);
                let sp: u32 = mem.read(s.wrapping_add(0xbc));
                let t = Reg::load_f32(f(mem, unit.wrapping_add(0x4c)))
                    .constant(MUL, k256)
                    .store();
                at_t(mem, sp, host, t);
                let k = if mem.read::<u16>(cmd.wrapping_add(4)) != 0 {
                    k16
                } else {
                    1.0f32.to_bits()
                };
                op::op_xyz_scale(mem, unit.wrapping_add(0x40), host, k);
                op::op_xyz_scale(mem, unit.wrapping_add(0x90), unit.wrapping_add(0x40), k_16);
                cmd.wrapping_add(6)
            }
            0 => {
                let heap: u32 = mem.read(host.wrapping_add(0x3a8));
                let old: u32 = mem.read(unit.wrapping_add(0xbc));
                op::op_free(mem, heap, old, call);
                let n: u16 = mem.read(cmd.wrapping_add(2));
                let count = i32::from(n.cast_signed());
                let heap: u32 = mem.read(host.wrapping_add(0x3a8));
                let size = (count.cast_unsigned() << 5).wrapping_add(0x10);
                let rec = op::op_malloc(mem, heap, size, call);
                mem.write(unit.wrapping_add(0xbc), rec);
                mem.write(rec, n);
                let pts = rec.wrapping_add(0x10);
                mem.write(
                    rec.wrapping_add(0xc),
                    (count.cast_unsigned() << 4).wrapping_add(pts),
                );
                mem.write(rec.wrapping_add(8), pts);
                let nos = i32::from(mem.read::<i16>(cmd.wrapping_add(4)))
                    .cast_unsigned()
                    .wrapping_add(cmd);
                let flag: u16 = mem.read(cmd.wrapping_add(6));
                let from = if flag == 0 { 0x40u32 } else { 0x90 };
                let mut i = 0i16;
                while i32::from(i) < count {
                    let k = i32::from(i).cast_unsigned();
                    let no = u32::from(mem.read::<u16>(nos.wrapping_add(2 * k)));
                    let s = opus_no_get(mem, list, unit, no);
                    for c in 0..3u32 {
                        let v: u32 = mem.read(s.wrapping_add(from + 4 * c));
                        mem.write(pts.wrapping_add(16 * k + 4 * c), crate::x87::load(v));
                    }
                    i = i.wrapping_add(1);
                }
                let (x, p): (u32, u32) = (
                    mem.read(rec.wrapping_add(0xc)),
                    mem.read(rec.wrapping_add(8)),
                );
                let f = Global::<u32>::new(OP_SPLINE_XYZ_TABLE).ptr(mem).addr;
                call(mem, f, &[x, p, count.cast_unsigned()]);
                mem.write(rec.wrapping_add(7), u8::from(flag == 0));
                cmd.wrapping_add(8)
            }
            0x2000 => {
                let s = source(mem);
                let sp: u32 = mem.read(s.wrapping_add(0xbc));
                let a = crate::x87::load_f32(f(mem, unit.wrapping_add(0x4c)));
                let t = Reg::load_f32(a).constant(MUL, k256).store();
                at_t(mem, sp, host, t);
                let e: f32 = f(mem, unit.wrapping_add(0x6c));
                let a2 = match Reg::load_f32(e) {
                    Reg::Num(z) if z == 0.0 => {
                        let c: f64 = mem.read(g(mem, 0x00b3_68a8));
                        Reg::load_f32(a).constant(crate::x87::ADD, c).store()
                    }
                    _ => Reg::load_f32(e).mem(crate::x87::ADD, a).store(),
                };
                let t2 = Reg::load_f32(a2).constant(MUL, k256).store();
                let second = host.wrapping_add(0x10);
                at_t(mem, sp, second, t2);
                if mem.read::<u16>(cmd.wrapping_add(4)) == 0 {
                    op::op_xyz_scale(mem, unit.wrapping_add(0x40), host, 1.0f32.to_bits());
                } else {
                    op::op_xyz_scale(mem, unit.wrapping_add(0x40), host, k16);
                    op::op_xyz_scale(mem, second, second, k16);
                }
                op::op_xyz_scale(mem, unit.wrapping_add(0x90), unit.wrapping_add(0x40), k_16);
                op::op_xyz_scale(mem, second, second, k_16);
                let m = host.wrapping_add(0x40);
                let f = Global::<u32>::new(OP_RT_TO).ptr(mem).addr;
                call(mem, f, &[m, unit.wrapping_add(0x90), second]);
                op::op_mul_m(mem, unit.wrapping_add(0xc0), host.wrapping_add(0x280), m);
                cmd.wrapping_add(6)
            }
            0x3000 => {
                mem.write(unit.wrapping_add(0xba), 0xau8);
                let a: [u16; 2] = mem.read(cmd.wrapping_add(2));
                mem.write(unit.wrapping_add(0xa0), a);
                cmd.wrapping_add(6)
            }
            _ => cmd,
        }
    }

    /// `opus_get_bone_vertex(out, chr, bone)` (`0x008054d0`): with bit
    /// 0x8000 the option position `bone & 0x7fff` (`Ch_OptposGet(chr, i,
    /// out)`), else the element's position (`Ch_ElemGetPos(chr, bone, 0,
    /// out)`), through `call`. Returns the callee's result.
    pub fn opus_get_bone_vertex<M: Mem>(
        mem: &mut M,
        out: u32,
        chr: u32,
        bone: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if bone & 0x8000 != 0 {
            let f = g(mem, 0x0083_54f0);
            call(mem, f, &[chr, bone & 0x7fff, out])
        } else {
            let f = g(mem, 0x0083_3d10);
            call(mem, f, &[chr, bone, 0, out])
        }
    }

    /// `opus_cheight_get(chr, battle)` (`0x00800b70`): the character's
    /// height, `MsGetChrHeight(chr)` in battle, else
    /// `Ch_GetFixHeight(g_EtChrHandles[chr])` (through `fcall`), rounded to
    /// `f32` (returned in `ST0`).
    pub fn opus_cheight_get<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        let v = if battle != 0 {
            let f = g(mem, 0x0079_50f0);
            fcall(mem, f, &[chr])
        } else {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
            let f = g(mem, 0x0082_aba0);
            fcall(mem, f, &[h])
        };
        f32::from_bits(crate::x87::load(ffx_bridge::x87::store(v)))
    }

    /// `opus_get_chr_pos(chr, out)` (`0x0081b1e0`): `out` = the emulated
    /// `r0` (16 bytes), then the character's position
    /// (`Ch_GetPos(g_EtChrHandles[chr], out, out + 4, out + 8)`, through
    /// `call`) when it has a handle. Returns the callee's result, or `chr`.
    pub fn opus_get_chr_pos<M: Mem>(
        mem: &mut M,
        chr: u32,
        out: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(out, r0);
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        if h == 0 {
            return chr;
        }
        let f = g(mem, 0x0082_ac90);
        call(mem, f, &[h, out, out.wrapping_add(4), out.wrapping_add(8)])
    }

    /// `opf_normal(host, ptr)` (`0x00815050`): the motion callback for a
    /// plain unit (slot 0 of `0x00c492c8`): the emulated VU registers
    /// `0x00c8f934..93c` take `0x00c8f53c..544`, `0x00c8f538` and
    /// `0x00c8f930` = 1/16, and the unit's `+0x40..+0x4c` times 1/16 go to
    /// `0x00c8f8e0..8ec` and the unit's `+0x90..+0x9c`. Returns `ptr`.
    pub fn opf_normal(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let saved: [u32; 3] = mem.read(g(mem, 0x00c8_f53c));
        mem.write(g(mem, 0x00c8_f934), saved);
        let k = 0x3d80_0000u32;
        mem.write(g(mem, 0x00c8_f538), k);
        mem.write(g(mem, 0x00c8_f930), k);
        for i in 0..4u32 {
            let x: f32 = mem.read(unit.wrapping_add(0x40 + 4 * i));
            let v = crate::x87::mul(x, f32::from_bits(k));
            mem.write(g(mem, 0x00c8_f8e0) + 4 * i, v);
            mem.write(unit.wrapping_add(0x90 + 4 * i), v);
        }
        ptr
    }

    /// `0x008178f0(host, cmd)` (`opu_eff_flag`): the flags word `+0xc` of
    /// the unit's `g_EtEff` record (`u16 unit +0x14`) against the mask
    /// `i16 +4 << 16 | i16 +2` (both sign-extended), by the flag nibble's
    /// low three bits: 0 set (`cmd + 6`), 1 clear (`cmd + 6`), 2/3 jump by
    /// `i16 +6` when any/none is set, else `cmd + 8`, 4/5 wait (`host
    /// +0x210` = 1, returning `cmd`) while any/none is set, else `cmd + 6`;
    /// 6 and 7 return `cmd`.
    pub fn opu_eff_flag(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let e = u32::from(mem.read::<u16>(unit.wrapping_add(0x14)));
        let at = EFF.ptr(mem).at(e).addr.wrapping_add(0xc);
        let hi = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
        let lo = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let mask = hi << 16 | lo;
        let kind = (i32::from(mem.read::<i16>(host.wrapping_add(0x21a))) >> 12) & 7;
        let flags: u32 = mem.read(at);
        let any = flags & mask != 0;
        let jump = || {
            i32::from(mem.read::<i16>(cmd.wrapping_add(6)))
                .cast_unsigned()
                .wrapping_add(cmd)
        };
        match kind {
            0 => {
                mem.write(at, flags | mask);
                cmd.wrapping_add(6)
            }
            1 => {
                mem.write(at, flags & !mask);
                cmd.wrapping_add(6)
            }
            2 | 3 if any == (kind == 2) => jump(),
            2 | 3 => cmd.wrapping_add(8),
            4 | 5 if any == (kind == 4) => {
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd
            }
            4 | 5 => cmd.wrapping_add(6),
            _ => cmd,
        }
    }

    /// `fldz; fld m32; fsubr st, st(1); fstp m32`: `0 - x` in place.
    fn neg0(mem: &mut impl Mem, at: u32) {
        let x: f32 = mem.read(at);
        let r = Reg::load_f32(0.0).mem(crate::x87::SUB, x).store();
        mem.write(at, r);
    }

    /// `opu_v_rev(host, cmd)` (`0x00819a20`): negates (as `0 - x`, so +0
    /// stays +0) the flagged x, y, z, w (flags 0x1000..0x8000) of the
    /// unit's vector at the `i16` offset `0x00c49388[cmd+2 & 0xf]`; with
    /// bit 15 of `cmd+2` also the flagged x, y, z of the unit's `+0x90`.
    /// Returns `cmd + 4`.
    pub fn opu_v_rev(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let fl: u16 = mem.read(host.wrapping_add(0x21a));
        let arg: u16 = mem.read(cmd.wrapping_add(2));
        let off: i16 = mem.read(g(mem, 0x00c4_9388) + 2 * u32::from(arg & 0xf));
        let v = unit.wrapping_add(i32::from(off).cast_unsigned());
        for (k, bit) in [0x1000u16, 0x2000, 0x4000, 0x8000].into_iter().enumerate() {
            if fl & bit != 0 {
                #[allow(clippy::cast_possible_truncation)] // 0..4
                neg0(mem, v.wrapping_add(4 * k as u32));
            }
        }
        if arg & 0x8000 != 0 {
            for (k, bit) in [0x1000u16, 0x2000, 0x4000].into_iter().enumerate() {
                if fl & bit != 0 {
                    #[allow(clippy::cast_possible_truncation)] // 0..3
                    neg0(mem, unit.wrapping_add(0x90 + 4 * k as u32));
                }
            }
        }
        cmd.wrapping_add(4)
    }

    /// `0x008182f0(host, cmd)` (`opu_v_stop`): the unit's velocity to stop
    /// in `n` frames (`u16 +2`, as `f32`): for flag bits 9..11 = 0 each
    /// flagged x, y, z, w (0x1000..0x8000) of `+0x70` = `-(+0x50 / n)`, for
    /// 1 those of `+0x80` = `-(+0x60 / n)` (`fchs`, so a NaN's sign flips
    /// too). Returns `cmd + 4`.
    pub fn opu_v_stop(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        const DIV: crate::x87::Op = |a, b| a / b;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        #[allow(clippy::cast_precision_loss)] // fild, fstp m32
        let n = f32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let fl: u16 = mem.read(host.wrapping_add(0x21a));
        let (from, to) = match fl & 0xe00 {
            0 => (0x50u32, 0x70u32),
            0x200 => (0x60, 0x80),
            _ => return cmd.wrapping_add(4),
        };
        for (k, bit) in [0x1000u16, 0x2000, 0x4000, 0x8000].into_iter().enumerate() {
            // each flag is read again from the host
            if mem.read::<u16>(host.wrapping_add(0x21a)) & bit != 0 {
                #[allow(clippy::cast_possible_truncation)] // 0..4
                let k = 4 * k as u32;
                let x: f32 = mem.read(unit.wrapping_add(from + k));
                let q = Reg::load_f32(x).reg(DIV, Reg::load_f32(n)).store();
                mem.write(unit.wrapping_add(to + k), q.to_bits() ^ 0x8000_0000);
            }
        }
        cmd.wrapping_add(4)
    }

    /// `0x0081b4d0(host, cmd)` (`opu_motion_cb`): the unit's motion
    /// callback byte `+0xba` = byte `cmd+2`, `u16 +0xa0`/`+0xa2` = `cmd+4`
    /// /`+6`; for callbacks 8 and 9 the unit's `+0x40..+0x48` = its
    /// `+0x90..+0x98` times 16, for 0xb `+0xa4`, `+0x44`, `+0x48` take
    /// `+0x90..+0x98` (through the FPU). Returns `cmd + 8`.
    pub fn opu_motion_cb(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let cb: u8 = mem.read(cmd.wrapping_add(2));
        mem.write(unit.wrapping_add(0xba), cb);
        let a: [u16; 2] = mem.read(cmd.wrapping_add(4));
        mem.write(unit.wrapping_add(0xa0), a);
        let cb: u8 = mem.read(unit.wrapping_add(0xba));
        match cb {
            8 | 9 => {
                for k in 0..3u32 {
                    let x: f32 = mem.read(unit.wrapping_add(0x90 + 4 * k));
                    let v = Reg::load_f32(x).constant(MUL, 16.0).store();
                    mem.write(unit.wrapping_add(0x40 + 4 * k), v);
                }
            }
            0xb => {
                for (from, to) in [(0x90u32, 0xa4u32), (0x94, 0x44), (0x98, 0x48)] {
                    let x: u32 = mem.read(unit.wrapping_add(from));
                    mem.write(unit.wrapping_add(to), crate::x87::load(x));
                }
            }
            _ => {}
        }
        cmd.wrapping_add(8)
    }

    /// `opu_et_no(obj, cmd)` (`0x008165d0`): the part at `+0x220` takes its
    /// effect's number (the effect's `u16` at `+0x1a`, bits 4-14) plus the
    /// command's `i16`.
    pub fn opu_et_no(mem: &mut impl Mem, obj: u32, cmd: u32) -> u32 {
        let part = mem.read::<u32>(obj.wrapping_add(0x220));
        let e = u32::from(mem.read::<u16>(part.wrapping_add(0x14)));
        let eff = mem.read::<u32>(EFF.ptr(mem).at(e).addr.wrapping_add(4));
        let v = (mem.read::<u16>(eff.wrapping_add(0x1a)) & 0x7ff0)
            .wrapping_add(mem.read::<u16>(cmd.wrapping_add(2)));
        mem.write(part.wrapping_add(0x1a), v);
        cmd.wrapping_add(4)
    }

    /// `opp_anmm_fin_en_bone3x(_, _, cmd, part)` (`0x00810ed0`): mode 5,
    /// and the command's `i16` as a float at `+0x40`.
    pub fn opp_anmm_fin_en_bone3x(mem: &mut impl Mem, cmd: u32, part: u32) -> u32 {
        let v = mem.read::<i16>(cmd.wrapping_add(2));
        mem.write(part.wrapping_add(0x4e), 5u8);
        mem.write(part.wrapping_add(0x40), f32::from(v));
        cmd.wrapping_add(4)
    }

    /// `opus_monster_index(obj)` (`0x007fe540`): the slot of `obj` among
    /// the 8 `g_OpusMonsters` (null included), or -1.
    pub fn opus_monster_index(mem: &impl Mem, obj: u32) -> i32 {
        (0..8)
            .find(|&i| MONSTERS.ptr(mem).at(i).get(mem) == obj)
            .map_or(-1, u32::cast_signed)
    }

    /// `g_OpusMonsterFlags` (`0x0119fef0`): a byte per monster slot.
    pub const MONSTER_FLAGS: Global<u8> = Global::new(0x0119_fef0);

    fn monster_variant(mem: &impl Mem, slot: u32) -> Option<u32> {
        let f: u8 = mem.read(MONSTER_FLAGS.ptr(mem).addr.wrapping_add(slot));
        (f != 0).then_some(if f & 1 != 0 { 0 } else { 2 })
    }

    /// `opus_monster_clut_pos(slot)` (`0x00800e20`): 0 without flags,
    /// else 0x3fc0 (flag bit 0) or 0x3fd0.
    pub fn opus_monster_clut_pos(mem: &impl Mem, slot: u32) -> u32 {
        monster_variant(mem, slot).map_or(0, |k| k * 8 + 0x3fc0)
    }

    /// `opus_monster_tex_pos(slot)` (`0x00800e50`): 0 without flags,
    /// else 0x3f40 (flag bit 0) or 0x3fc0.
    pub fn opus_monster_tex_pos(mem: &impl Mem, slot: u32) -> u32 {
        monster_variant(mem, slot).map_or(0, |k| (k + 0xfd) << 6)
    }

    /// `opus_osu_init_group(i, n)` (`0x00801480`): the first 16 bytes of the
    /// first `n` 0x100-byte parts of opus object `i` (the table at
    /// `g_OpusBattle`) zeroed.
    pub fn opus_osu_init_group(mem: &mut impl Mem, i: u32, n: u32) {
        let obj: u32 = mem.read(OPUS_BATTLE.ptr(mem).addr.wrapping_add(4 * (i & 0xffff)));
        let mut p: u32 = mem.read(obj.wrapping_add(0x20));
        for _ in 0..(n & 0xffff) {
            mem.write(p, [0u32; 4]);
            p = p.wrapping_add(0x100);
        }
    }

    /// The table `opus_no_get` reads a unit's weapon number from (32-byte
    /// entries, a pointer at `+4`; `0x012a4104`).
    pub const NO_TABLE: Global<u32> = Global::new(0x012a_4104);

    /// `opus_no_get(list, unit, no)` (`0x00801060`): the 0x100-byte unit
    /// of the list (parts at `list +0x20`) that `no` (16 bits) names
    /// relative to `unit`:
    /// - 0xff00: `unit` itself.
    /// - 0xffff, or another negative `-k` outside the ranges below: `k`
    ///   steps up the parent chain (`u16 +0x1c`); that unit, or the first
    ///   part when its first word is null (`is_ptr32_null`).
    /// - 0xb001..=0xcfff: `no + 0x4000 + unit +0x1a`; 0xfe00..=0xfeff: the
    ///   low nibble over `+0x1a & 0x7ff0` of the unit `NO_TABLE[unit
    ///   +0x14]` points at; a non-negative `no` as it is: the unit with
    ///   that number (`u16 +0x1a`) and `unit`'s group (`+0x1e`) in the
    ///   index lists `+0x14`, `+0x10`, `+0x18` (`i16`, ended by a negative
    ///   one), else the first part.
    pub fn opus_no_get(mem: &impl Mem, list: u32, unit: u32, no: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 argument
        let mut n = no as u16;
        let base = |mem: &dyn Fn(u32) -> u32| mem(list.wrapping_add(0x20));
        let rd = |a: u32| mem.read::<u32>(a);
        let up = |at: u32| {
            u32::from(mem.read::<u16>(at.wrapping_add(0x1c)))
                .wrapping_shl(8)
                .wrapping_add(base(&rd))
        };
        if n & 0x8000 != 0 {
            if n == 0xff00 {
                return unit;
            }
            let steps = if n == 0xffff {
                Some(1)
            } else if n.wrapping_add(0x4fff) <= 0x1ffe {
                n = n
                    .wrapping_add(0x4000)
                    .wrapping_add(mem.read::<u16>(unit.wrapping_add(0x1a)));
                None
            } else if n & 0xff00 == 0xfe00 {
                let i = u32::from(mem.read::<u16>(unit.wrapping_add(0x14)));
                let e: u32 = mem.read(NO_TABLE.ptr(mem).addr.wrapping_add(32 * i));
                n = (mem.read::<u16>(e.wrapping_add(0x1a)) & 0x7ff0) | (n & 0xf);
                None
            } else {
                Some(n.wrapping_neg())
            };
            if let Some(k) = steps {
                let mut u = unit;
                for _ in 0..k {
                    u = up(u);
                }
                return if ffx_system::stub::is_ptr32_null(rd(u)) == 0 {
                    u
                } else {
                    base(&rd)
                };
            }
        }
        let want = i32::from(n.cast_signed());
        let group: u8 = mem.read(unit.wrapping_add(0x1e));
        for at in [0x14u32, 0x10, 0x18] {
            let mut e: u32 = mem.read(list.wrapping_add(at));
            loop {
                let i: i16 = mem.read(e);
                if i < 0 {
                    break;
                }
                let u = (i32::from(i) << 8).cast_unsigned().wrapping_add(base(&rd));
                if i32::from(mem.read::<u16>(u.wrapping_add(0x1a))) == want
                    && mem.read::<u8>(u.wrapping_add(0x1e)) == group
                {
                    return u;
                }
                e = e.wrapping_add(2);
            }
        }
        base(&rd)
    }

    /// `opus_parent_get(obj, unit)` (`0x00801f00`): the matrix (`+0xc0`)
    /// of the unit `u16 unit +0xb8` names, among the object's units (parts
    /// at `obj +0x320`, index lists `+0x314`, `+0x310`, `+0x318`): 0 gives
    /// the object's own matrix (`obj + 0x280`), 0xff00 `unit`'s, a
    /// negative `-k` the unit `k` steps up the parent chain (`u16
    /// +0x1c`), 0xfe00..=0xfeff the number from `NO_TABLE` as in
    /// `opus_no_get`, a non-negative one the unit with that number (`u16
    /// +0x1a`), else the first unit's.
    pub fn opus_parent_get(mem: &impl Mem, obj: u32, unit: u32) -> u32 {
        let mut n: u16 = mem.read(unit.wrapping_add(0xb8));
        let base = || mem.read::<u32>(obj.wrapping_add(0x320));
        if n == 0 {
            return obj.wrapping_add(0x280);
        }
        if n & 0x8000 != 0 {
            if n == 0xff00 {
                return unit.wrapping_add(0xc0);
            }
            if n & 0xff00 == 0xfe00 && n != 0xffff {
                let i = u32::from(mem.read::<u16>(unit.wrapping_add(0x14)));
                let e: u32 = mem.read(NO_TABLE.ptr(mem).addr.wrapping_add(32 * i));
                n = (mem.read::<u16>(e.wrapping_add(0x1a)) & 0x7ff0) | (n & 0xf);
            } else {
                let mut u = unit;
                for _ in 0..n.wrapping_neg() {
                    u = u32::from(mem.read::<u16>(u.wrapping_add(0x1c)))
                        .wrapping_shl(8)
                        .wrapping_add(base());
                }
                return u.wrapping_add(0xc0);
            }
        }
        let want = i32::from(n.cast_signed());
        for at in [0x314u32, 0x310, 0x318] {
            let mut e: u32 = mem.read(obj.wrapping_add(at));
            loop {
                let i: i16 = mem.read(e);
                if i < 0 {
                    break;
                }
                let u = (i32::from(i) << 8).cast_unsigned().wrapping_add(base());
                if i32::from(mem.read::<u16>(u.wrapping_add(0x1a))) == want {
                    return u.wrapping_add(0xc0);
                }
                e = e.wrapping_add(2);
            }
        }
        base().wrapping_add(0xc0)
    }

    /// `opu_end(obj, cmd)` (`0x0080c3f0`): unless the object is held
    /// (`u16 +0x216`), its part (`+0x220`) is cleared (first 16 bytes);
    /// if its `g_EtEff` record (`EFF[u16 part +0x14]`) still points at it
    /// with a type nibble of 4, the object waits (`+0x210` = 0x78) and
    /// `cmd` comes back; otherwise unit `u16 +0x214` of the list `obj +
    /// 0x300` is shut down (`opus_prog_off`, `opus_draw_off`,
    /// `yonExtMapDeleteId`, through `call`) and `obj +0x218` decremented.
    /// Then `+0x210` = 1 and 0 is returned.
    pub fn opu_end<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u16>(obj.wrapping_add(0x216)) == 0 {
            let part: u32 = mem.read(obj.wrapping_add(0x220));
            mem.write(part, [0u32; 4]);
            let i = u32::from(mem.read::<u16>(part.wrapping_add(0x14)));
            let e = EFF.ptr(mem).at(i).addr;
            if mem.read::<u32>(e + 4) == part && mem.read::<u32>(e) & 0xf000_0000 == 0x4000_0000 {
                mem.write(obj.wrapping_add(0x210), 0x78u8);
                return cmd;
            }
            let list = obj.wrapping_add(0x300);
            let no = u32::from(mem.read::<u16>(obj.wrapping_add(0x214)));
            for (f, args) in [(0x0080_14c0u32, [list, no]), (0x0080_0d10, [list, no])] {
                let f = Global::<u32>::new(f).ptr(mem).addr;
                call(mem, f, &args);
            }
            let id = i32::from(mem.read::<i16>(obj.wrapping_add(0x214))).cast_unsigned();
            let f = Global::<u32>::new(0x0080_7830).ptr(mem).addr;
            call(mem, f, &[id]);
            let n: u16 = mem.read(obj.wrapping_add(0x218));
            mem.write(obj.wrapping_add(0x218), n.wrapping_sub(1));
        }
        mem.write(obj.wrapping_add(0x210), 1u8);
        0
    }

    /// `opus_chr_width_00800cd0(chr, battle)` (`0x00800cd0`): a character's
    /// width / 6: `MsGetChrWidth(chr)` when `battle` is set, else
    /// `Ch_GetThickness(g_EtChrHandles[chr])` (both float results, through
    /// `fcall`), stored as `f32` before and after the division. Returns
    /// it in `ST0`.
    pub fn opus_chr_width<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        use crate::x87::Reg;
        let w = if battle == 0 {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
            let f = Global::<u32>::new(0x0082_ad70).ptr(mem).addr;
            fcall(mem, f, &[h])
        } else {
            let f = Global::<u32>::new(0x0079_5a20).ptr(mem).addr;
            fcall(mem, f, &[chr])
        };
        let div: crate::x87::Op = |a, b| a / b;
        let six: f64 = mem.read(Global::<u32>::new(0x00b3_eb88).ptr(mem).addr);
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let w = w as f32;
        Reg::load_f32(w).constant(div, six).store()
    }

    /// `opus_clip_off_all_in_battle()` (`0x00800bb0`): when
    /// `MsBattleChrStart()` is not 0, every character handle in the first
    /// 32 `g_EtChrHandles` slots gets `Ch_EnClip(h, 0)` (through `call`).
    /// Returns what the original leaves in `eax`: the last call's result,
    /// or the last handle read.
    pub fn opus_clip_off_all_in_battle<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(0x0078_0cb0).ptr(mem).addr;
        let mut eax = call(mem, f, &[]);
        if eax == 0 {
            return 0;
        }
        let clip = Global::<u32>::new(0x0082_aa30).ptr(mem).addr;
        for i in 0..32 {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(i).get(mem);
            eax = h;
            if h != 0 {
                eax = call(mem, clip, &[h, 0]);
            }
        }
        eax
    }

    /// `opus_cscale_get(chr, battle)` (`0x00800c10`): the larger of a
    /// character's height and width (`MsGetChrHeight`/`MsGetChrWidth(chr)`
    /// in battle, else `Ch_GetFixHeight`/`Ch_GetThickness` of
    /// `g_EtChrHandles[chr]`; float results through `fcall`, each stored
    /// as `f32`; the height on a tie or NaN) divided by the `f64` at
    /// `0x00b587d8`. Returns it in `ST0`.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_cscale_get<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        use crate::x87::Reg;
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let (fa, fb, arg) = if battle == 0 {
            (0x0082_aba0u32, 0x0082_ad70u32, h)
        } else {
            (0x0079_50f0, 0x0079_5a20, chr)
        };
        let fa = Global::<u32>::new(fa).ptr(mem).addr;
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let a = fcall(mem, fa, &[arg]) as f32;
        let fb = Global::<u32>::new(fb).ptr(mem).addr;
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let b = fcall(mem, fb, &[arg]) as f32;
        let a = crate::x87::load_f32(a);
        let b = crate::x87::load_f32(b);
        let m = if b > a { b } else { a };
        let div: crate::x87::Op = |x, y| x / y;
        let k: f64 = mem.read(Global::<u32>::new(0x00b5_87d8).ptr(mem).addr);
        Reg::load_f32(m).constant(div, k).store()
    }

    /// `opus_osu_init(out)` (`0x00801200`): the 0x100 bytes at `out`
    /// filled with `g_Ee_Gpr`'s `r0` (16 bytes at a time, read anew each
    /// time), then `+0xc8` = `0x80808080`. Returns `r0`'s last word (left
    /// in `eax`).
    pub fn opus_osu_init(mem: &mut impl Mem, out: u32) -> u32 {
        let r0 = crate::ppp::bridge::R0.ptr(mem).addr;
        let mut last = 0;
        for i in 0..0x40u32 {
            last = mem.read::<u32>(r0 + 4 * (i % 4));
            mem.write(out.wrapping_add(4 * i), last);
        }
        mem.write(out.wrapping_add(0xc8), 0x8080_8080u32);
        last
    }

    /// `opus_osu_copy_00800eb0(set, dst, src)`: `src`'s words `+0x30`..
    /// `+0x4c` and `+0x90`..`+0x9c`, `u16 +0x14` and bytes `+0x18`,
    /// `+0x1e`, `+0xba`, `+0xb3` into `dst`, and `dst.u16[+0x1c]` =
    /// `(src - set[+0x20]) >> 8` (`src`'s index among `set`'s 0x100-byte
    /// records). Returns the byte `+0xb3` (left in `eax`).
    pub fn opus_osu_copy(mem: &mut impl Mem, set: u32, dst: u32, src: u32) -> u32 {
        for o in (0x30u32..0x50).step_by(4).chain((0x90u32..0xa0).step_by(4)) {
            let x: u32 = mem.read(src.wrapping_add(o));
            mem.write(dst.wrapping_add(o), x);
        }
        let base: u32 = mem.read(set.wrapping_add(0x20));
        #[allow(clippy::cast_possible_truncation)] // the u16 store
        mem.write(dst.wrapping_add(0x1c), (src.wrapping_sub(base) >> 8) as u16);
        let b: u8 = mem.read(src.wrapping_add(0x1e));
        mem.write(dst.wrapping_add(0x1e), b);
        let h: u16 = mem.read(src.wrapping_add(0x14));
        mem.write(dst.wrapping_add(0x14), h);
        let mut last = 0;
        for o in [0x18u32, 0xba, 0xb3] {
            last = mem.read::<u8>(src.wrapping_add(o));
            mem.write(dst.wrapping_add(o), last);
        }
        u32::from(last)
    }

    /// `0x00801780(set, id, list)`: `id` into the first free (negative)
    /// `i16` of `set`'s list `[set +0x10 + 4 list]`, among `u16 +0x34` - 1
    /// entries (the index an `i16`), and `u16 +0x38` up by one: 0; -1 when
    /// none is free.
    pub fn opus_list_add(mem: &mut impl Mem, set: u32, id: u16, list: u32) -> u32 {
        let n = u32::from(mem.read::<u16>(set.wrapping_add(0x34)).wrapping_sub(1));
        let t: u32 = mem.read(set.wrapping_add(0x10).wrapping_add(list.wrapping_mul(4)));
        let mut i = 0i16;
        while i32::from(i) < n.cast_signed() {
            let at = t.wrapping_add(i32::from(i).cast_unsigned().wrapping_mul(2));
            if mem.read::<i16>(at) < 0 {
                mem.write(at, id);
                let c: u16 = mem.read(set.wrapping_add(0x38));
                mem.write(set.wrapping_add(0x38), c.wrapping_add(1));
                return 0;
            }
            i = i.wrapping_add(1);
        }
        u32::MAX
    }

    /// `opus_prog_on(set, list)` (`0x00801700`): like
    /// [`opus_list_add`], but the id put into the first free (negative)
    /// `i16` of list `list` is the first part (0x100 bytes from `[set +
    /// 0x20]`, among the same `u16 +0x34` - 1) whose first word is 0
    /// (`is_ptr32_null`), and `u16 +0x38` up by one. Both indexes are
    /// `i16`. Returns that part's number in `ax` over the list address's
    /// high half (`eax` as left); -1 when no list entry or no part is
    /// free.
    pub fn opus_prog_on(mem: &mut impl Mem, set: u32, list: u32) -> u32 {
        let count =
            u32::from(mem.read::<u16>(set.wrapping_add(0x34)).wrapping_sub(1)).cast_signed();
        let table: u32 = mem.read(set.wrapping_add(0x10).wrapping_add(list.wrapping_mul(4)));
        let idx = |i: i16| i32::from(i).cast_unsigned();
        let mut i = 0i16;
        while i32::from(i) < count {
            let at = table.wrapping_add(idx(i).wrapping_mul(2));
            if mem.read::<i16>(at) < 0 {
                let mut k = 0i16;
                while i32::from(k) < count {
                    let parts: u32 = mem.read(set.wrapping_add(0x20));
                    let first: u32 = mem.read(parts.wrapping_add(idx(k) << 8));
                    if first == 0 {
                        mem.write(at, k);
                        let c: u16 = mem.read(set.wrapping_add(0x38));
                        mem.write(set.wrapping_add(0x38), c.wrapping_add(1));
                        return table & 0xffff_0000 | u32::from(k.cast_unsigned());
                    }
                    k = k.wrapping_add(1);
                }
                return u32::MAX;
            }
            i = i.wrapping_add(1);
        }
        u32::MAX
    }

    fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    fn f(mem: &impl Mem, a: u32) -> f32 {
        mem.read(a)
    }

    /// `opu_main_host`'s 64-byte copy through the emulated EE registers
    /// `v0`..`a1` (`0x00c8f528`.., [`crate::md::bridge::SPR_BOUNCE`]):
    /// words 1..15 (and word 0 with `w0`) into the registers, then all of
    /// them to `dst`, word 0 straight from `src`.
    fn ee_copy64(mem: &mut impl Mem, src: u32, dst: u32, w0: bool) {
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let first: u32 = mem.read(src);
        if w0 {
            mem.write(regs, first);
        }
        for i in 1..16u32 {
            let x: u32 = mem.read(src.wrapping_add(4 * i));
            mem.write(regs + 4 * i, x);
        }
        mem.write(dst, first);
        for i in 1..16u32 {
            let x: u32 = mem.read(regs + 4 * i);
            mem.write(dst.wrapping_add(4 * i), x);
        }
    }

    /// The emulated VU registers `opu_main_host` integrates a unit's
    /// motion in (`0x00c8f8d0`..`0x00c8f93c`).
    const VF: u32 = 0x00c8_f8d0;

    /// `opu_main_host`'s motion step of unit record `u` (and its draw
    /// callback): position/velocity/acceleration in the emulated VU
    /// registers, unless `+0xb2` bit 1 (or the host's `+0x212` is set):
    /// velocity += acceleration, position += velocity; then, unless
    /// `+0xb2` bit 2, `+0x90`.. = the second position row / 16 and the
    /// callback `0x00c492c8[+0xba]` (1..11) with `(host, ptr)`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn motion<M: Mem>(
        mem: &mut M,
        host: u32,
        u: u32,
        ptr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let vf = g(mem, VF);
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let r = |o: u32| vf + (o - VF);
        let at = |o: u32| u.wrapping_add(o);
        let flags = mem.read::<u8>(at(0xb2));
        mem.write(regs, 0x3d80_0000u32);
        for i in 0..8u32 {
            let x: u32 = mem.read(at(0x30 + 4 * i));
            mem.write(r(0x00c8_f8d0) + 4 * i, x);
        }
        for (d, s) in [(0x00c8_f934u32, 4u32), (0x00c8_f938, 8)] {
            let x: u32 = mem.read(regs + s);
            mem.write(r(d), x);
        }
        let x: u32 = mem.read(regs + 0xc);
        mem.write(r(0x00c8_f930), 0x3d80_0000u32);
        mem.write(r(0x00c8_f93c), x);
        if flags & 2 == 0 {
            let k = i32::from(mem.read::<i16>(host.wrapping_add(0x212)));
            mem.write(regs, k);
            for i in 0..4u32 {
                let x: u32 = mem.read(at(0x50 + 4 * i));
                mem.write(r(0x00c8_f8f0) + 4 * i, x);
            }
            if k == 0 {
                for i in 0..12u32 {
                    let x: u32 = mem.read(at(0x60 + 4 * i));
                    mem.write(r(0x00c8_f900) + 4 * i, x);
                }
                // velocity (+0x50, 8 floats) += acceleration (+0x70)
                for i in 0..8u32 {
                    let v = r(0x00c8_f8f0) + 4 * i;
                    let a = r(0x00c8_f910) + 4 * i;
                    let x = crate::x87::add(f(mem, v), f(mem, a));
                    mem.write(v, x);
                }
                for i in 0..8u32 {
                    let x: u32 = mem.read(r(0x00c8_f8f0) + 4 * i);
                    mem.write(at(0x50 + 4 * i), x);
                }
                // position (+0x30, 8 floats) += velocity
                for i in 0..8u32 {
                    let p = r(0x00c8_f8d0) + 4 * i;
                    let v = r(0x00c8_f8f0) + 4 * i;
                    let x = crate::x87::add(f(mem, p), f(mem, v));
                    mem.write(p, x);
                }
                for i in 0..8u32 {
                    let x: u32 = mem.read(r(0x00c8_f8d0) + 4 * i);
                    mem.write(at(0x30 + 4 * i), x);
                }
            }
        }
        if mem.read::<u8>(at(0xb2)) & 4 != 0 {
            return;
        }
        let k = Reg::load_f32(f(mem, r(0x00c8_f930)));
        let p = r(0x00c8_f8e0);
        let x0 = Reg::load_f32(f(mem, p)).reg(MUL, k).store();
        mem.write(p, x0);
        let x1 = Reg::load_f32(f(mem, p + 4)).reg(MUL, k).store();
        mem.write(p + 4, x1);
        let x2 = k.mem(MUL, f(mem, p + 8)).store();
        mem.write(p + 8, x2);
        let x3 = k.mem(MUL, f(mem, p + 12)).store();
        mem.write(p + 12, x3);
        for i in 0..4u32 {
            let x: u32 = mem.read(p + 4 * i);
            mem.write(at(0x90 + 4 * i), x);
        }
        let cb = mem.read::<u8>(at(0xba));
        if cb != 0 && cb < 0xc {
            let table = g(mem, 0x00c4_92c8);
            let fp: u32 = mem.read(table + 4 * u32::from(cb));
            call(mem, fp, &[host, ptr]);
        }
    }

    /// `opu_main_host(battle, list)` (`0x0080cc00`): runs the opus units
    /// of battle `battle` (`g_OpusBattle[battle]`, `g_VfxPass` = 1 for
    /// battle 1). The host record comes from the SPR stack
    /// (`op_spr_alloc(0x400)`, [`super::bridge`]'s `g_OpusBattleList`
    /// while it runs): the listener (`0x0113fcd0`, 64 bytes) at `+0x280`,
    /// the block at `0x0113fd10` at `+0x2c0`, the battle's first 0x100
    /// bytes at `+0x300` (copied back at the end), `g_Ee_Gpr`'s `r0` at
    /// `+0x210` with `+0x21c` = `list`, the unit order `+0x224` =
    /// `[+0x310 + 4 list]` (`i16`s ending at -1), `+0x21e` = 1 when
    /// `MsBattleChrStart`.
    ///
    /// The order is walked twice, the second time (`+0x21d` = 0xff) only
    /// for slots marked to run once more. Each unit (0x100 bytes from
    /// `+0x320`) has four program slots (`+0` pointers, `+0x10` wait
    /// bytes, `+0x1f` mark bits): a slot runs its commands (`i16`, low
    /// byte the handler in `0x00c48ec8`, or with 0x100 the battle's
    /// overlay table `[[battle +0x50] +0x7c]`) until a handler sets the
    /// host's `+0x210`, which then adds `+0x210 - 1` to the wait. After the
    /// first pass's slots comes the unit's [`motion`] step (straight away
    /// with `+0x21f`).
    ///
    /// Callees through `call`. Returns the last copied word (`eax` after
    /// `op_spr_free`).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_main_host<M: Mem>(
        mem: &mut M,
        battle: u32,
        list: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let battle = battle & 0xffff;
        let pass = g(mem, 0x0113_335c);
        if battle == 1 {
            mem.write(pass, 1u32);
        }
        let alloc = g(mem, 0x007e_6a00);
        let host = call(mem, alloc, &[0x400]);
        let ob = OPUS_BATTLE.ptr(mem).addr;
        let b: u32 = mem.read(ob + 4 * battle);
        let src1 = g(mem, 0x0113_fcd0);
        ee_copy64(mem, src1, host.wrapping_add(0x280), false);
        let hl = g(mem, 0x012a_12fc);
        let src2 = g(mem, 0x0113_fd10);
        ee_copy64(mem, src2, host.wrapping_add(0x2c0), true);
        for k in 0..4u32 {
            ee_copy64(
                mem,
                b.wrapping_add(0x40 * k),
                host.wrapping_add(0x300 + 0x40 * k),
                true,
            );
        }
        mem.write(hl, host);
        let h = |o: u32| host.wrapping_add(o);
        let r0 = crate::ppp::bridge::R0.ptr(mem).addr;
        for i in 0..4u32 {
            let x: u32 = mem.read(r0 + 4 * i);
            mem.write(h(0x210 + 4 * i), x);
        }
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        let lb = list as u8;
        mem.write(h(0x21c), lb);
        let order: u32 = mem.read(h(0x310 + 4 * u32::from(lb)));
        mem.write(h(0x224), order);
        let start = g(mem, 0x0078_0cb0);
        if call(mem, start, &[]) != 0 {
            mem.write(h(0x21e), 1u8);
        }
        let get = g(mem, 0x00a4_4650);
        let set = g(mem, 0x00a4_4700);
        let handlers = g(mem, 0x00c4_8ec8);
        let cur = g(mem, 0x012f_4118);
        let mut ptr = b;
        loop {
            let order: u32 = mem.read(h(0x224));
            let idx = i32::from(mem.read::<i16>(h(0x218)));
            let mut v: u16 = mem.read(order.wrapping_add(idx.cast_unsigned().wrapping_mul(2)));
            while v == 0xffff {
                if mem.read::<u8>(h(0x21d)) != 0 {
                    return finish(mem, host, b, pass, call);
                }
                mem.write(h(0x218), 0u16);
                mem.write(h(0x21d), 0xffu8);
                v = mem.read(order);
            }
            let units: u32 = mem.read(h(0x320));
            let u = (i32::from(v.cast_signed()).cast_unsigned() << 8).wrapping_add(units);
            mem.write(h(0x214), v);
            mem.write(h(0x220), u);
            mem.write(h(0x216), 0u16);
            let mut run_motion = false;
            loop {
                if mem.read::<u8>(h(0x21f)) != 0 && mem.read::<u8>(h(0x21d)) == 0 {
                    run_motion = true;
                    break;
                }
                let s = i32::from(mem.read::<i16>(h(0x216)));
                let su = s.cast_unsigned();
                let slot_at = u.wrapping_add(su.wrapping_mul(4));
                let sp: u32 = mem.read(slot_at);
                ptr = call(mem, get, &[sp]);
                'slot: {
                    if ptr == 0 {
                        break 'slot;
                    }
                    let bit = 1u8.wrapping_shl(su & 0x1f);
                    let bit32 = 1u32.wrapping_shl(su & 0x1f);
                    let marks: u8 = mem.read(u.wrapping_add(0x1f));
                    if mem.read::<u8>(h(0x21d)) == 0 {
                        if u32::from(marks) & bit32 != 0 {
                            break 'slot;
                        }
                    } else {
                        if u32::from(marks) & bit32 == 0 {
                            break 'slot;
                        }
                        mem.write(u.wrapping_add(0x1f), !bit & marks & 0xf);
                    }
                    let s = i32::from(mem.read::<i16>(h(0x216))).cast_unsigned();
                    let wait_at = u.wrapping_add(0x10).wrapping_add(s);
                    let w: u8 = mem.read(wait_at);
                    if w != 0 {
                        mem.write(wait_at, w.wrapping_sub(1));
                        break 'slot;
                    }
                    loop {
                        let c = if ptr == 0 {
                            0u32
                        } else {
                            i32::from(mem.read::<i16>(ptr)).cast_unsigned()
                        };
                        mem.write(cur, ptr);
                        #[allow(clippy::cast_possible_truncation)] // the u16 store
                        mem.write(h(0x21a), c as u16);
                        let fp = if c & 0x100 != 0 {
                            let e: u32 = mem.read(b.wrapping_add(0x50));
                            let t: u32 = mem.read(e.wrapping_add(0x7c));
                            if t == 0 {
                                let fmt = g(mem, 0x00b5_8aa0);
                                let dbg = g(mem, 0x0062_f500);
                                let cx =
                                    i32::from(u16::try_from(c & 0xffff).unwrap_or(0).cast_signed())
                                        .cast_unsigned();
                                call(mem, dbg, &[fmt, cx]);
                                break;
                            }
                            mem.read::<u32>(t.wrapping_add(4 * (c & 0xff)))
                        } else {
                            let fp: u32 = mem.read(handlers + 4 * (c & 0xff));
                            if fp == 0 {
                                break;
                            }
                            fp
                        };
                        ptr = call(mem, fp, &[host, ptr]);
                        if mem.read::<u8>(h(0x210)) != 0 {
                            break;
                        }
                    }
                    let d: u8 = mem.read(h(0x210));
                    let s = i32::from(mem.read::<i16>(h(0x216))).cast_unsigned();
                    let wait_at = u.wrapping_add(0x10).wrapping_add(s);
                    let w: u8 = mem.read(wait_at);
                    mem.write(wait_at, w.wrapping_add(d.wrapping_sub(1)));
                    mem.write(h(0x210), 0u8);
                    if ptr != 0 && mem.read::<u8>(h(0x21d)) != 0 {
                        let s = mem.read::<u8>(h(0x216));
                        let m: u8 = mem.read(u.wrapping_add(0x1f));
                        mem.write(
                            u.wrapping_add(0x1f),
                            m | 1u8.wrapping_shl(u32::from(s & 0x1f)),
                        );
                    }
                    let saved = call(mem, set, &[ptr]);
                    let s = i32::from(mem.read::<i16>(h(0x216))).cast_unsigned();
                    mem.write(u.wrapping_add(s.wrapping_mul(4)), saved);
                }
                let s: u16 = mem.read(h(0x216));
                mem.write(h(0x216), s.wrapping_add(1));
                if s.wrapping_add(1).cast_signed() >= 4 {
                    break;
                }
            }
            if run_motion || mem.read::<u8>(h(0x21d)) == 0 {
                motion(mem, host, u, ptr, call);
            }
            let i: u16 = mem.read(h(0x218));
            mem.write(h(0x218), i.wrapping_add(1));
        }
    }

    /// `opu_main_host`'s end: the battle's 0x100 bytes back, the list and
    /// the SPR block released, `g_VfxPass` 0.
    fn finish<M: Mem>(
        mem: &mut M,
        host: u32,
        b: u32,
        pass: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for k in 0..4u32 {
            ee_copy64(
                mem,
                host.wrapping_add(0x300 + 0x40 * k),
                b.wrapping_add(0x40 * k),
                true,
            );
        }
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let last: u32 = mem.read(regs + 0x3c);
        let hl = g(mem, 0x012a_12fc);
        mem.write(hl, 0u32);
        let free = g(mem, 0x007e_6a60);
        call(mem, free, &[0x400]);
        mem.write(pass, 0u32);
        last
    }
}
