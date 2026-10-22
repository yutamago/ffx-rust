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

    /// `i8`: id shades on (`0x00844bb0` returns it).
    pub const ID_SHADE_ON: Global<i8> = Global::new(0x0130_34b8);

    /// `Ch_IdGetShade(chr)` (`0x0083d880`): the id word's shade (`0x0083e980`
    /// of it: bits 17..31 while id shades are on, else `0x4210`).
    pub fn ch_id_get_shade(mem: &impl Mem, chr: u32) -> u32 {
        let id: i32 = mem.read(chr + l::PID);
        if ID_SHADE_ON.get(mem) != 0 {
            ((id >> 17) & 0x7fff).cast_unsigned()
        } else {
            0x4210
        }
    }

    /// `Ch_IdGetShade0/1/2(chr)` (`0x0083d8a0`, `0x0083d8c0`,
    /// `0x0083d8e0`): 5-bit field `k` of [`ch_id_get_shade`].
    pub fn ch_id_get_shade_field(mem: &impl Mem, chr: u32, k: u32) -> u32 {
        (ch_id_get_shade(mem, chr) >> (5 * k)) & 0x1f
    }

    /// `graphicCharacterLightingSetShadeCoeff_debug(inst)` (`0x0063cec0`),
    /// `Sg_VectorGetLength2(a, b)` (`0x008366a0`, ST0),
    /// `Sg_VectorGetLength(v, n)` (`0x00836670`, ST0), `FFXVu0Normalize(dst,
    /// src)` (`0x006ed6f0`) and `Ch_SetShadeID(chr, v, frames)`
    /// (`0x0082b720`), all through the game.
    pub const SET_SHADE_COEFF_DEBUG: u32 = 0x0063_cec0;
    pub const VECTOR_GET_LENGTH2: u32 = 0x0083_66a0;
    pub const VECTOR_GET_LENGTH: u32 = 0x0083_6670;
    pub const VU0_NORMALIZE: u32 = 0x006e_d6f0;
    pub const SET_SHADE_ID: u32 = 0x0082_b720;
    /// Bytes of scratch [`ch_id_shade_calc`] needs: its vector
    /// (`[ebp-0x14]`; `w` is never written, as in the original).
    pub const ID_SHADE_CALC_SCRATCH: u32 = 0x10;

    /// `Ch_IdShadeCalc(chr)` (`0x00834110`; caller `Ch_CalcMiddle_Batch`).
    /// - Id shades off ([`ID_SHADE_ON`], `Sg_IdExistShade`): unless
    ///   `g_ChrDrawDetached`, `chr`'s instance (`+0x830`) gets the
    ///   renderer's debug shade coefficient once per other active character
    ///   bound to it (`+0x19c`; the array re-read after each call, as the
    ///   walk's end) and once more.
    /// - Id shades on, with flag bit 18 (`+0x194`): the distances from
    ///   `+0x3c` to `+0x8c`, `+0x9c`, `+0xac` (`Sg_VectorGetLength2`, as
    ///   `f32`) make a vector in `scratch`, normalized, each component
    ///   `1 - c`, normalized again, scaled by the id word's shades 0, 1, 2
    ///   over 31 (`f32(shade) / 31.0 * c`, in 53 bits); its length
    ///   (`Sg_VectorGetLength(v, 3)`) is the shade id, set over 3 frames
    ///   (`Ch_SetShadeID`).
    ///
    /// No caller reads `eax`.
    pub fn ch_id_shade_calc<M: Mem>(
        mem: &mut M,
        chr: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use crate::handle::bridge::{CHR_ARRAY, CHR_COUNT};
        if ID_SHADE_ON.get(mem) == 0 {
            if crate::sys::bridge::DRAW_DETACHED.get(mem) != 0 {
                return;
            }
            let end = |mem: &M| {
                let n = CHR_COUNT.get(mem).cast_unsigned();
                CHR_ARRAY.get(mem).wrapping_add(n.wrapping_mul(l::SIZE))
            };
            let coeff = mem.rebase(SET_SHADE_COEFF_DEBUG);
            let mut c = CHR_ARRAY.get(mem);
            while c != end(mem) {
                if mem.read::<u8>(c.wrapping_add(l::ACTIVE)) != 0
                    && c != chr
                    && mem.read::<u32>(c.wrapping_add(0x19c)) == chr
                {
                    let inst: u32 = mem.read(chr.wrapping_add(0x830));
                    call(mem, coeff, &[inst]);
                }
                c = c.wrapping_add(l::SIZE);
            }
            let inst: u32 = mem.read(chr.wrapping_add(0x830));
            call(mem, coeff, &[inst]);
            return;
        }
        if mem.read::<u32>(chr.wrapping_add(l::FLAGS)) & 0x4_0000 == 0 {
            return;
        }
        let v = scratch;
        let from = chr.wrapping_add(0x3c);
        for (k, off) in [0x8cu32, 0x9c, 0xac].into_iter().enumerate() {
            let len2 = mem.rebase(VECTOR_GET_LENGTH2);
            let d = fcall(mem, len2, &[from, chr.wrapping_add(off)]);
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            mem.write(v + 4 * k as u32, d as f32);
        }
        let normalize = mem.rebase(VU0_NORMALIZE);
        call(mem, normalize, &[v, v]);
        for k in 0..3 {
            let c: f32 = mem.read(v + 4 * k);
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            mem.write(v + 4 * k, (1.0 - f64::from(c)) as f32);
        }
        call(mem, normalize, &[v, v]);
        for k in 0..3 {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)] // fild of 0..=31
            let shade = ch_id_get_shade_field(mem, chr, k) as i32 as f32;
            let c: f32 = mem.read(v + 4 * k);
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            mem.write(v + 4 * k, (f64::from(shade) / 31.0 * f64::from(c)) as f32);
        }
        let len = mem.rebase(VECTOR_GET_LENGTH);
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let s = fcall(mem, len, &[v, 3]) as f32;
        call(mem, mem.rebase(SET_SHADE_ID), &[chr, s.to_bits(), 3]);
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
