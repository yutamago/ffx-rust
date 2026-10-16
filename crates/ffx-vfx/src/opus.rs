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
}
