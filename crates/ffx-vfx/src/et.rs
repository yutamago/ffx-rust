//! The effect tables of the op layer (`op_et_*`, `op_get_chr*`).
//!
//! - `g_EtChr` (`0x012a22c0`): 240 records of 0x20 bytes, one per
//!   character the effects know: its handle at `+0`, its character number
//!   at `+4`, bind effect flags (`u16`) at `+0xc` (bit 0 the bind effect
//!   drawn, 0x10 the second bind effect drawn, 0x100 walk, 0x200 breath,
//!   0x400/0x1000/0x2000 water), the index of its draw flags at `+0xe`,
//!   and the stone status in bit 0 of `+0x17`. Searches by handle stop at
//!   the first match; a null handle finds nothing.
//! - Draw flags: a byte per index in `g_EtChrDrawFlags2` (`0x012a85b0`)
//!   when that one is nonzero, else in `g_EtChrDrawFlags` (`0x012a84c0`);
//!   bit 4 hides the bind effect, bit 8 the second one.
//! - `g_EtEff` (`0x012a4100`): 512 records of 0x20 bytes, an effect's ID
//!   at `+0` and its object at `+4`, whose `u16` at `+0xb0` takes the
//!   signals (0x4000 off, 0x8000 fade). A character's bind effects have
//!   the IDs `0x40000400`, `0x500`, `0x600` and `0x700` plus its record
//!   index.

/// The tables where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    /// `g_EtChr`.
    pub const CHR: Global<[u32; 8]> = Global::new(0x012a_22c0);
    /// Records in `g_EtChr`.
    pub const CHRS: u32 = 240;
    /// `g_EtEff`.
    pub const EFF: Global<[u32; 8]> = Global::new(0x012a_4100);
    /// Records in `g_EtEff`.
    pub const EFFS: u32 = 512;
    /// `g_EtChrDrawFlags`.
    pub const DRAW_FLAGS: Global<u8> = Global::new(0x012a_84c0);
    /// `g_EtChrDrawFlags2`, used where nonzero.
    pub const DRAW_FLAGS2: Global<u8> = Global::new(0x012a_85b0);
    /// `g_EtBreathFlags`: bit 0 set when a bind breath effect of
    /// character 1 or 0x65 is turned on.
    pub const BREATH_FLAGS: Global<u32> = Global::new(0x00cc_b248);
    /// `g_OpusBattle`: the battle's effect object (`+0x1c` its draw list,
    /// `+0x20` its parts, `+0x3a` the genko counter).
    pub const OPUS_BATTLE: Global<u32> = Global::new(0x012a_40c0);

    fn chr(mem: &impl Mem, i: u32) -> u32 {
        CHR.ptr(mem).at(i).addr
    }

    /// The record of character handle `h` (`None` for null or unknown).
    fn find_chr(mem: &impl Mem, h: u32) -> Option<u32> {
        if h == 0 {
            return None;
        }
        (0..CHRS).find(|&i| mem.read::<u32>(chr(mem, i)) == h)
    }

    /// The first effect with ID `id`.
    fn find_eff(mem: &impl Mem, id: u32) -> Option<u32> {
        (0..EFFS).find(|&i| mem.read::<u32>(EFF.ptr(mem).at(i).addr) == id)
    }

    fn signal(mem: &mut impl Mem, id: u32, bits: u16) {
        if let Some(i) = find_eff(mem, id) {
            let obj = mem.read::<u32>(EFF.ptr(mem).at(i).addr + 4);
            let p = Ptr::<u16>::new(obj.wrapping_add(0xb0));
            let v = p.get(mem) | bits;
            p.set(mem, v);
        }
    }

    fn and_flags(mem: &mut impl Mem, h: u32, keep: u16) -> Option<u32> {
        let i = find_chr(mem, h)?;
        let p = Ptr::<u16>::new(chr(mem, i) + 0xc);
        let v = p.get(mem) & keep;
        p.set(mem, v);
        Some(i)
    }

    /// `op_get_chr_status_stone(h)` (`0x007fcc80`): the stone bit. An
    /// unknown handle reads the record before the table (index -1), as the
    /// original does.
    pub fn op_get_chr_status_stone(mem: &impl Mem, h: u32) -> u32 {
        let i = find_chr(mem, h).unwrap_or(u32::MAX);
        let at = CHR
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(0x20))
            .wrapping_add(0x17);
        u32::from(mem.read::<u8>(at) & 1)
    }

    /// `op_get_chrhd_by_chrno(no)` (`0x007fcd10`): the handle of the first
    /// of the first 208 records with that character number, else 0.
    pub fn op_get_chrhd_by_chrno(mem: &impl Mem, no: u32) -> u32 {
        (0..208)
            .find(|&i| mem.read::<u32>(chr(mem, i) + 4) == no)
            .map_or(0, |i| mem.read(chr(mem, i)))
    }

    /// `op_get_chrhd_all_list(out)` (`0x007fccc0`): the non-null handles
    /// of the first 208 records, in order; returns how many.
    pub fn op_get_chrhd_all_list(mem: &mut impl Mem, out: u32) -> u32 {
        let mut n = 0u32;
        for i in 0..208 {
            let h = mem.read::<u32>(chr(mem, i));
            if h != 0 {
                mem.write(out.wrapping_add(4 * n), h);
                n += 1;
            }
        }
        n
    }

    /// `op_et_battle_genko_counter_get()` (`0x007fb240`).
    pub fn op_et_battle_genko_counter_get(mem: &impl Mem) -> u32 {
        let b = OPUS_BATTLE.get(mem);
        u32::from(mem.read::<u8>(b.wrapping_add(0x3a)))
    }

    /// Sets or clears `bit` in a drawn character's draw flags; returns
    /// `bit` if it was clear before, else 0.
    fn draw_bit(mem: &mut impl Mem, h: u32, drawn: u16, bit: u8, set: bool) -> u32 {
        let Some(index) = find_chr(mem, h) else {
            return 0;
        };
        let rec = chr(mem, index);
        if mem.read::<u16>(rec + 0xc) & drawn == 0 {
            return 0;
        }
        let k = u32::from(mem.read::<u16>(rec + 0xe));
        let second = DRAW_FLAGS2.ptr(mem).at(k);
        let flags = if second.get(mem) != 0 {
            second
        } else {
            DRAW_FLAGS.ptr(mem).at(k)
        };
        let old = flags.get(mem);
        flags.set(mem, if set { old | bit } else { old & !bit });
        u32::from(!old & bit)
    }

    /// `op_et_bindeff_draw_on(h)` (`0x007fbab0`).
    pub fn op_et_bindeff_draw_on(mem: &mut impl Mem, h: u32) -> u32 {
        draw_bit(mem, h, 1, 4, false)
    }

    /// `op_et_bindeff_draw_off(h)` (`0x007fba40`).
    pub fn op_et_bindeff_draw_off(mem: &mut impl Mem, h: u32) -> u32 {
        draw_bit(mem, h, 1, 4, true)
    }

    /// `op_et_bindeff2_draw_on(h)` (`0x007fb5a0`).
    pub fn op_et_bindeff2_draw_on(mem: &mut impl Mem, h: u32) -> u32 {
        draw_bit(mem, h, 0x10, 8, false)
    }

    /// `op_et_bindeff2_walk_on(h)` (`0x007fb830`).
    pub fn op_et_bindeff2_walk_on(mem: &mut impl Mem, h: u32) {
        and_flags(mem, h, 0xfeff);
    }

    /// `op_et_bindeff2_water_on(h)` (`0x007fb8e0`).
    pub fn op_et_bindeff2_water_on(mem: &mut impl Mem, h: u32) {
        and_flags(mem, h, 0xcbff);
    }

    /// `op_et_bindeff2_breath_on(h)` (`0x007fb4c0`): also sets bit 0 of
    /// `g_EtBreathFlags` for characters 1 and 0x65 (the `u16` at the
    /// handle).
    pub fn op_et_bindeff2_breath_on(mem: &mut impl Mem, h: u32) {
        if and_flags(mem, h, 0xfdff).is_some() && matches!(mem.read::<u16>(h), 1 | 0x65) {
            let v = BREATH_FLAGS.get(mem) | 1;
            BREATH_FLAGS.set(mem, v);
        }
    }

    /// `op_et_bindeff_off_signal(h)` (`0x007fbcd0`): clears the bind bits
    /// (0 and 1) and signals the character's four bind effects off.
    pub fn op_et_bindeff_off_signal(mem: &mut impl Mem, h: u32) {
        let Some(i) = and_flags(mem, h, 0xfffc) else {
            return;
        };
        for kind in [0x400, 0x500, 0x600, 0x700] {
            signal(mem, (0x4000_0000 + kind) + i, 0x4000);
        }
    }

    /// `op_et_eff_off_signal(id)` (`0x007fc450`).
    pub fn op_et_eff_off_signal(mem: &mut impl Mem, id: u32) {
        signal(mem, id, 0x4000);
    }

    /// `op_et_weapon_afterimage_fade(id)` (`0x007fca40`).
    pub fn op_et_weapon_afterimage_fade(mem: &mut impl Mem, id: u32) {
        signal(mem, id, 0x8000);
    }
}
