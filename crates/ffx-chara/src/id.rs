//! The map id word of a character ([`crate::layout::PID`]): what the
//! ground under it says, as bit fields (`Ch_IdGet*`).

/// A 2-bit field of the word at bit `shift` (`sar` then `and 3`).
#[must_use]
pub const fn field2(pid: u32, shift: u32) -> u32 {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // sar
    let v = ((pid as i32) >> shift) as u32;
    v & 3
}

/// The originals, on the characters and the id tables.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::field2;
    use crate::layout as l;

    /// `u32` map ids for the indirect ids `0x30..=0x3f`
    /// (`Ch_IdGetMap`; 64 entries by raw id).
    pub const MAP_TABLE: Global<u32> = Global::new(0x0130_19d8);
    /// `u8` effect tables, 4 per set (`Ch_IdGetEffectTable`).
    pub const EFFECT_TABLE: Global<u8> = Global::new(0x0130_1af8);
    /// `i8` which set of [`EFFECT_TABLE`] (`Sg_IdSetEffectTableIndex`).
    pub const EFFECT_TABLE_INDEX: Global<i8> = Global::new(0x0130_1b00);

    fn pid(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::PID)
    }

    /// A 2-bit field: `Ch_IdGetEncount` (`0x0083d820`, bit 7),
    /// `Ch_IdGetEffect` (`0x0083d7c0`, 9), `Ch_IdGetDictionary`
    /// (`0x0083d7a0`, 11), `Ch_IdGetWater` (`0x0083d920`, 13),
    /// `Ch_IdGetSound` (`0x0083d900`, 15).
    pub fn ch_id_get_field(mem: &impl Mem, chr: u32, shift: u32) -> u32 {
        field2(pid(mem, chr), shift)
    }

    /// `Ch_IdGetMapB(chr)` (`0x0083d860`): the raw map id.
    pub fn ch_id_get_map_b(mem: &impl Mem, chr: u32) -> u32 {
        pid(mem, chr) & 0x7f
    }

    /// `Ch_IdGetMap(chr)` (`0x0083d840`): the map id, ids `0x30..=0x3f`
    /// through [`MAP_TABLE`].
    pub fn ch_id_get_map(mem: &impl Mem, chr: u32) -> u32 {
        let m = ch_id_get_map_b(mem, chr);
        if m.wrapping_sub(0x30) <= 0xf {
            MAP_TABLE.ptr(mem).at(m).get(mem)
        } else {
            m
        }
    }

    /// `Ch_IdGetEffectTable(chr)` (`0x0083d7e0`): 8 or 9 for motion type
    /// 2 (by `FLAGS` bit 29), else the effect's entry in the current set.
    pub fn ch_id_get_effect_table(mem: &impl Mem, chr: u32) -> u32 {
        if mem.read::<u8>(chr + l::MOTION_TYPE) == 2 {
            let w: u32 = mem.read(chr + l::FLAGS);
            return (!(w >> 29) & 1) | 8;
        }
        let e = ch_id_get_field(mem, chr, 9);
        #[allow(clippy::cast_sign_loss)] // movsx, then scaled
        let set = i32::from(EFFECT_TABLE_INDEX.get(mem)) as u32;
        let at = EFFECT_TABLE
            .ptr(mem)
            .addr
            .wrapping_add(e)
            .wrapping_add(set.wrapping_mul(4));
        u32::from(mem.read::<u8>(at))
    }

    /// `Ch_IdSetDefPid(chr)` (`0x0083d940`).
    pub fn ch_id_set_def_pid(mem: &mut impl Mem, chr: u32) {
        mem.write(chr + l::PID, 0xfffe_0000u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields() {
        assert_eq!(field2(0xffff_ffff, 15), 3);
        assert_eq!(field2(0x0000_0180, 7), 3);
        assert_eq!(field2(0x8000_0000, 30), 2);
    }
}
