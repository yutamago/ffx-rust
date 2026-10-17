//! The "opus" effect objects (`opus_*`, and the `opu_*`/`opp_*` script
//! commands that step them).
//!
//! An opus object has a draw list at `+0x1c` (`i16` part numbers, ended by
//! a negative one) and its 0x100-byte parts at `+0x20`; a part's group
//! byte is at `+0x1e` and its draw kind at `+0xbb`. The script commands
//! take the object and the command's address and return the next
//! command's.

/// The objects where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use crate::et::bridge::{EFF, OPUS_BATTLE};

    /// `g_OpusWeapons`: the four weapon objects.
    pub const WEAPONS: Global<u32> = Global::new(0x0119_fea0);
    /// `g_OpusMonsters`: the eight monster objects.
    pub const MONSTERS: Global<u32> = Global::new(0x0119_fed0);

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
}
