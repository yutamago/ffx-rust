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

    /// Sets or clears `bit` in a drawn character's draw flags (drawn: a bind
    /// flag in `drawn` set; 0 for any character); returns `bit` if it was
    /// clear before, else 0.
    fn draw_bit(mem: &mut impl Mem, h: u32, drawn: u16, bit: u8, set: bool) -> u32 {
        let Some(index) = find_chr(mem, h) else {
            return 0;
        };
        let rec = chr(mem, index);
        if drawn != 0 && mem.read::<u16>(rec + 0xc) & drawn == 0 {
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

    /// The four bind effects of character `h` (`find_chr`), each found by
    /// its ID (`0x40000400`, `0x500`, `0x600`, `0x700` plus the record
    /// index) and handed to `f`, which returns what the original leaves in
    /// `eax` for a found one. Returns that `eax`: the end of `g_EtChr` for
    /// an unknown handle, `f`'s for a found last effect, else the end of
    /// `g_EtEff`; 0 for null (the original leaves its caller's `eax`).
    fn bind_effects<M: Mem>(
        mem: &mut M,
        handle: u32,
        mut found: impl FnMut(&mut M, u32) -> u32,
    ) -> u32 {
        if handle == 0 {
            return 0;
        }
        let Some(index) = find_chr(mem, handle) else {
            return CHR.ptr(mem).at(CHRS).addr;
        };
        let mut eax = 0;
        for kind in [0x400, 0x500, 0x600, 0x700] {
            eax = match find_eff(mem, (0x4000_0000 + kind) + index) {
                Some(eff) => {
                    let rec = EFF.ptr(mem).at(eff).addr;
                    found(mem, rec)
                }
                None => EFF.ptr(mem).at(EFFS).addr,
            };
        }
        eax
    }

    /// `op_et_bindeff_send_common_signal(h, bits)` (`0x007fbdf0`, the tail
    /// of `MsEffectSendSignal`): `bits` OR-ed into the record flags (`+0xc`)
    /// of character `h`'s four bind effects. Returns `eax` as left
    /// ([`bind_effects`]; a found last effect leaves its record).
    pub fn op_et_bindeff_send_common_signal(mem: &mut impl Mem, h: u32, bits: u32) -> u32 {
        bind_effects(mem, h, |mem, r| {
            let p = Ptr::<u32>::new(r + 0xc);
            let v = p.get(mem) | bits;
            p.set(mem, v);
            r
        })
    }

    /// `op_et_bindeff_send_signal(h, bits)` (`0x007fbef0`, from
    /// `opu_et_etc`): the `u16` `bits` OR-ed into the signals (`+0xb0`) of
    /// the objects of character `h`'s four bind effects. Returns `eax` as
    /// left ([`bind_effects`]; a found last effect leaves its object).
    pub fn op_et_bindeff_send_signal(mem: &mut impl Mem, h: u32, bits: u32) -> u32 {
        bind_effects(mem, h, |mem, r| {
            let obj = mem.read::<u32>(r + 4);
            let p = Ptr::<u16>::new(obj.wrapping_add(0xb0));
            #[allow(clippy::cast_possible_truncation)] // a u16 argument
            let v = p.get(mem) | bits as u16;
            p.set(mem, v);
            obj
        })
    }

    /// `op_et_eff_off_signal(id)` (`0x007fc450`).
    pub fn op_et_eff_off_signal(mem: &mut impl Mem, id: u32) {
        signal(mem, id, 0x4000);
    }

    /// `op_et_weapon_afterimage_fade(id)` (`0x007fca40`).
    pub fn op_et_weapon_afterimage_fade(mem: &mut impl Mem, id: u32) {
        signal(mem, id, 0x8000);
    }

    /// The draw flag index of character `h`'s record, if it has one.
    fn draw_index(mem: &impl Mem, h: u32) -> Option<u32> {
        let i = find_chr(mem, h)?;
        Some(u32::from(mem.read::<u16>(chr(mem, i) + 0xe)))
    }

    fn range_check(i: u32) {
        // the original's __report_rangecheckfailure (a fast fail)
        assert!(i < 0xf0, "draw flag index {i:#x} out of range");
    }

    /// `op_et_draw_flags_restore(h)` (`0x007fc130`): the character's
    /// draw flags back from `g_EtChrDrawFlags2` (bit 7 off), which is
    /// cleared; returns what it held (0 for an unknown handle).
    pub fn op_et_draw_flags_restore(mem: &mut impl Mem, h: u32) -> u32 {
        let Some(i) = draw_index(mem, h) else {
            return 0;
        };
        let held = mem.read::<u8>(DRAW_FLAGS2.ptr(mem).addr + i);
        mem.write(DRAW_FLAGS.ptr(mem).addr + i, held & 0x7f);
        range_check(i);
        mem.write(DRAW_FLAGS2.ptr(mem).addr + i, 0u8);
        u32::from(held)
    }

    /// `op_et_draw_flags_save(h)` (`0x007fc190`): the character's draw
    /// flags kept in `g_EtChrDrawFlags2` (with bit 7) and cleared; returns
    /// what `g_EtChrDrawFlags2` held before (0 for an unknown handle).
    pub fn op_et_draw_flags_save(mem: &mut impl Mem, h: u32) -> u32 {
        let Some(i) = draw_index(mem, h) else {
            return 0;
        };
        let flags = mem.read::<u8>(DRAW_FLAGS.ptr(mem).addr + i);
        let held = mem.read::<u8>(DRAW_FLAGS2.ptr(mem).addr + i);
        mem.write(DRAW_FLAGS2.ptr(mem).addr + i, flags | 0x80);
        range_check(i);
        mem.write(DRAW_FLAGS.ptr(mem).addr + i, 0u8);
        u32::from(held)
    }

    /// `g_EtChrHidden`: 1 while `op_et_chr_show(0)` hides every character.
    pub const CHR_HIDDEN: Global<u32> = Global::new(0x00cc_b9d0);
    /// `g_OpEtChrShow`: the last `op_et_chr_show` argument (a byte).
    pub const CHR_SHOW: Global<u8> = Global::new(0x011a_0063);

    /// `op_et_chr_show(on)` (`0x007ff090`): with 0, sets bit 1 of the draw
    /// flags of the first 208 records that have a handle (and an index
    /// below 0xf0), else clears it.
    pub fn op_et_chr_show(mem: &mut impl Mem, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        CHR_SHOW.set(mem, on as u8);
        CHR_HIDDEN.set(mem, u32::from(on == 0));
        for r in 0..0xd0 {
            let at = chr(mem, r);
            if mem.read::<u32>(at) == 0 {
                continue;
            }
            let i = u32::from(mem.read::<u16>(at + 0xe));
            if i >= 0xf0 {
                continue;
            }
            let p = DRAW_FLAGS.ptr(mem).addr + i;
            let f: u8 = mem.read(p);
            mem.write(p, if on == 0 { f | 2 } else { f & 0xfd });
        }
    }

    /// `op_et_chr_index_by_no(no)` (`0x007feaa0`): the first of the 208
    /// `g_EtChr` records with character number `no` (`+4`), else -1.
    pub fn op_et_chr_index_by_no(mem: &impl Mem, no: u32) -> i32 {
        (0..0xd0)
            .find(|&i| mem.read::<u32>(chr(mem, i) + 4) == no)
            .map_or(-1, u32::cast_signed)
    }

    /// `op_et_chr_count_same_no_007feb20(h)` (`0x007feb20`): how many
    /// `g_EtChr` records (all 240, itself included) share the character
    /// number (`+4`) of handle `h`'s record; 0 for a null or unknown
    /// handle.
    pub fn op_et_chr_count_same_no(mem: &impl Mem, h: u32) -> u32 {
        let Some(i) = find_chr(mem, h) else {
            return 0;
        };
        let no: u32 = mem.read(chr(mem, i) + 4);
        let same = (0..CHRS).filter(|&k| mem.read::<u32>(chr(mem, k) + 4) == no);
        u32::try_from(same.count()).unwrap_or(u32::MAX)
    }

    /// `opu_eff_part_index(unit)` (`0x00800da0`): for the `g_EtEff` record
    /// the unit names (`+0x14`), its part (`+0x1a & 7`) when below the
    /// record's count (`+8`); else the index of the unit's byte `+0x18` in
    /// the record's list (`+0x10`), or the count with its low byte
    /// cleared (0 for none).
    pub fn opu_eff_part_index(mem: &impl Mem, unit: u32) -> u32 {
        let rec = EFF
            .ptr(mem)
            .at(u32::from(mem.read::<u16>(unit.wrapping_add(0x14))))
            .addr;
        let part = u32::from(mem.read::<u16>(unit.wrapping_add(0x1a)) & 7);
        let count = u32::from(mem.read::<u8>(rec + 8));
        if part < count {
            return part;
        }
        let want: u8 = mem.read(unit.wrapping_add(0x18));
        for i in 0..count {
            if mem.read::<u8>(rec + 0x10 + i) == want {
                return i;
            }
        }
        count & 0xffff_ff00
    }

    /// `opu_eff_free_by_id(id)` (`0x007fe920`): every `g_EtEff` record
    /// with this ID (none for -1) emptied.
    pub fn opu_eff_free_by_id(mem: &mut impl Mem, id: u32) {
        if id == u32::MAX {
            return;
        }
        for i in 0..EFFS {
            let r = EFF.ptr(mem).at(i).addr;
            if mem.read::<u32>(r) != id {
                continue;
            }
            mem.write(r, u32::MAX);
            mem.write(r + 4, 0u32);
            mem.write(r + 8, 0x101u16);
            mem.write(r + 0xc, 0u32);
            mem.write(r + 0x10, 0u8);
            mem.write(r + 0x18, 0u8);
            mem.write(r + 0x1c, 0xffu16);
        }
    }

    /// `g_EtChrHandles`: 240 character handles (`0x012a8100`) in the order
    /// the draw flags were laid out.
    pub const CHR_HANDLES: Global<u32> = Global::new(0x012a_8100);

    /// `opus_chr_index_map(out)` (`0x007fe450`): `out` (240 `u16`, all
    /// 0xffff first) maps each character's draw flag index to its place in
    /// `g_EtChrHandles`; returns how many moved.
    pub fn opus_chr_index_map(mem: &mut impl Mem, out: u32) -> u32 {
        for i in 0..0x78 {
            mem.write(out.wrapping_add(4 * i), u32::MAX);
        }
        let mut moved = 0u32;
        for r in 0..CHRS {
            let rec = chr(mem, r);
            let h: u32 = mem.read(rec);
            if h == 0 {
                continue;
            }
            let Some(u) =
                (0..0xf0).find(|&u| mem.read::<u32>(CHR_HANDLES.ptr(mem).addr + 4 * u) == h)
            else {
                continue;
            };
            let idx = u32::from(mem.read::<u16>(rec + 0xe));
            if idx != u {
                moved += 1;
            }
            #[allow(clippy::cast_possible_truncation)] // below 0xf0
            mem.write(out.wrapping_add(2 * idx), u as u16);
        }
        moved
    }

    fn or_flags(mem: &mut impl Mem, h: u32, set: u16) -> Option<u32> {
        let i = find_chr(mem, h)?;
        let p = Ptr::<u16>::new(chr(mem, i) + 0xc);
        let v = p.get(mem) | set;
        p.set(mem, v);
        Some(i)
    }

    /// `op_et_bindeff2_breath_off(h)` (`0x007fb470`): bind flag 0x200 set;
    /// for characters 1 and 0x65 (the `u16` at the handle) bit 0 of
    /// `g_EtBreathFlags` cleared.
    pub fn op_et_bindeff2_breath_off(mem: &mut impl Mem, h: u32) {
        if or_flags(mem, h, 0x200).is_some() && matches!(mem.read::<u16>(h), 1 | 0x65) {
            let v = BREATH_FLAGS.get(mem) & !1;
            BREATH_FLAGS.set(mem, v);
        }
    }

    /// `op_et_bindeff2_draw_off(h)` (`0x007fb530`).
    pub fn op_et_bindeff2_draw_off(mem: &mut impl Mem, h: u32) -> u32 {
        draw_bit(mem, h, 0x10, 8, true)
    }

    /// `op_et_bindeff2_draw_set(h, hide)` (`0x007fb610`): as
    /// `op_et_bindeff2_draw_off` (`hide` not 0) or `_draw_on`.
    pub fn op_et_bindeff2_draw_set(mem: &mut impl Mem, h: u32, hide: u32) -> u32 {
        draw_bit(mem, h, 0x10, 8, hide != 0)
    }

    /// `op_et_bindeff_draw_set(h, hide)` (`0x007fbb20`): as
    /// `op_et_bindeff_draw_off` (`hide` not 0) or `_draw_on`.
    pub fn op_et_bindeff_draw_set(mem: &mut impl Mem, h: u32, hide: u32) -> u32 {
        draw_bit(mem, h, 1, 4, hide != 0)
    }

    /// `op_et_chr_draw_off(h)` (`0x007fc000`): bit 0 of any character's
    /// draw flags set; returns 1 if it was clear.
    pub fn op_et_chr_draw_off(mem: &mut impl Mem, h: u32) -> u32 {
        draw_bit(mem, h, 0, 1, true)
    }

    /// `op_et_chr_draw_on(h)` (`0x007fc060`): bit 0 cleared; returns 1 if
    /// it was clear.
    pub fn op_et_chr_draw_on(mem: &mut impl Mem, h: u32) -> u32 {
        draw_bit(mem, h, 0, 1, false)
    }

    /// `op_et_chr_draw_set(h, hide)` (`0x007fc0c0`).
    pub fn op_et_chr_draw_set(mem: &mut impl Mem, h: u32, hide: u32) -> u32 {
        draw_bit(mem, h, 0, 1, hide != 0)
    }

    /// `op_et_bindeff2_walk_off(h)` (`0x007fb7f0`): bind flag 0x100 set.
    pub fn op_et_bindeff2_walk_off(mem: &mut impl Mem, h: u32) {
        or_flags(mem, h, 0x100);
    }

    /// `op_et_bindeff2_water_off(h)` (`0x007fb890`): bind flags 0x3400 set.
    pub fn op_et_bindeff2_water_off(mem: &mut impl Mem, h: u32) {
        or_flags(mem, h, 0x3400);
    }

    /// `g_EtBindDefault` (`0x011a0080`): bind flags; `0x007fb870` keeps
    /// the water bits at 0x800.
    pub const BIND_DEFAULT: Global<u32> = Global::new(0x011a_0080);

    /// `op_et_bind_default_water(...)` (`0x007fb870`): the low half of
    /// `g_EtBindDefault` with bits 0x3400 cleared and 0x800 set; returns the
    /// word it computed (left in `eax`).
    pub fn op_et_bind_default_water(mem: &mut impl Mem) -> u32 {
        let v = (BIND_DEFAULT.get(mem) & 0xcbff) | 0x800;
        #[allow(clippy::cast_possible_truncation)] // the low half is stored
        mem.write(BIND_DEFAULT.ptr(mem).addr, v as u16);
        v
    }

    /// `op_et_draw_flags_switch(h, restore)` (`0x007fc200`):
    /// [`op_et_draw_flags_restore`] (`restore` not 0) or
    /// [`op_et_draw_flags_save`].
    pub fn op_et_draw_flags_switch(mem: &mut impl Mem, h: u32, restore: u32) -> u32 {
        if restore == 0 {
            op_et_draw_flags_save(mem, h)
        } else {
            op_et_draw_flags_restore(mem, h)
        }
    }

    fn eff_obj_flags(mem: &mut impl Mem, id: u32, f: impl FnOnce(u16) -> u16) {
        if let Some(i) = find_eff(mem, id) {
            let obj = mem.read::<u32>(EFF.ptr(mem).at(i).addr + 4);
            let p = Ptr::<u16>::new(obj.wrapping_add(0xb0));
            let v = f(p.get(mem));
            p.set(mem, v);
        }
    }

    fn eff_flags(mem: &mut impl Mem, id: u32, f: impl FnOnce(u32) -> u32) {
        if let Some(i) = find_eff(mem, id) {
            let p = Ptr::<u32>::new(EFF.ptr(mem).at(i).addr + 0xc);
            let v = f(p.get(mem));
            p.set(mem, v);
        }
    }

    /// `op_et_eff_signal_and(id, mask)` (`0x007fc490`): the first effect
    /// with ID `id` keeps the bits of `mask` in its object's signals (`u16`
    /// at `+0xb0`).
    pub fn op_et_eff_signal_and(mem: &mut impl Mem, id: u32, mask: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        eff_obj_flags(mem, id, |v| v & mask as u16);
    }

    /// `op_et_eff_signal_or(id, bits)` (`0x007fc550`).
    pub fn op_et_eff_signal_or(mem: &mut impl Mem, id: u32, bits: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        eff_obj_flags(mem, id, |v| v | bits as u16);
    }

    /// `op_et_eff_flags_and(id, mask)` (`0x007fc4d0`): the first effect
    /// with ID `id` keeps the bits of `mask` in its record's `+0xc`.
    pub fn op_et_eff_flags_and(mem: &mut impl Mem, id: u32, mask: u32) {
        eff_flags(mem, id, |v| v & mask);
    }

    /// `op_et_eff_flags_or(id, bits)` (`0x007fc510`).
    pub fn op_et_eff_flags_or(mem: &mut impl Mem, id: u32, bits: u32) {
        eff_flags(mem, id, |v| v | bits);
    }

    /// `op_et_eff_init_all()` (`0x007fef70`): every `g_EtEff` record
    /// emptied as `opu_eff_free_by_id` does, the object (`+4`) kept.
    pub fn op_et_eff_init_all(mem: &mut impl Mem) {
        for i in 0..EFFS {
            let r = EFF.ptr(mem).at(i).addr;
            mem.write(r, u32::MAX);
            mem.write(r + 8, 0x101u16);
            mem.write(r + 0xc, 0u32);
            mem.write(r + 0x10, 0u8);
            mem.write(r + 0x18, 0u8);
            mem.write(r + 0x1c, 0xffu16);
        }
    }

    /// `MsBattleGetStatus()` (`0x007816f0`), called through `call`.
    const MS_BATTLE_GET_STATUS: Global<u32> = Global::new(0x0078_16f0);

    /// The first of the 208 `g_EtChrHandles` the opus functions look at:
    /// 0 in battle, else 0x20.
    fn handles_from<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) -> u32 {
        let f = MS_BATTLE_GET_STATUS.ptr(mem).addr;
        if call(mem, f, &[]) == 0 { 0x20 } else { 0 }
    }

    fn handle(mem: &impl Mem, k: u32) -> u32 {
        mem.read(CHR_HANDLES.ptr(mem).addr.wrapping_add(4 * k))
    }

    /// `opus_chr_handle_count()` (`0x007fcf60`): how many of the 208
    /// handles (from 0 in battle, else from 0x20; `MsBattleGetStatus`
    /// through `call`) are not null.
    pub fn opus_chr_handle_count<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let k0 = handles_from(mem, call);
        let n = (0..0xd0).filter(|&k| handle(mem, k0 + k) != 0).count();
        u32::try_from(n).unwrap_or(0)
    }

    /// `opus_chr_handle_nth(n)` (`0x007fcfb0`): the `n`th non-null one of
    /// those handles; the first when there are fewer, 0 for none.
    pub fn opus_chr_handle_nth<M: Mem>(
        mem: &mut M,
        n: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let k0 = handles_from(mem, call);
        let mut seen = 0u32;
        for k in 0..0xd0 {
            let h = handle(mem, k0 + k);
            if h != 0 {
                if seen == n {
                    return h;
                }
                seen += 1;
            }
        }
        (0..0xd0)
            .map(|k| handle(mem, k0 + k))
            .find(|&h| h != 0)
            .unwrap_or(0)
    }

    /// `opus_chr_draw_index_nth(n)` (`0x007fcf10`): the draw flag index
    /// (`+0xe`) of [`opus_chr_handle_nth`]'s character, 0 when it has no
    /// record.
    pub fn opus_chr_draw_index_nth<M: Mem>(
        mem: &mut M,
        n: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let h = opus_chr_handle_nth(mem, n, call);
        draw_index(mem, h).unwrap_or(0)
    }

    /// `opu_unit_free_by_oef2(opus, oef2)` (`0x007fdf70`), called through
    /// `call`.
    const OPU_UNIT_FREE_BY_OEF2: Global<u32> = Global::new(0x007f_df70);
    /// `op_oef2_set2(oef2)` (`0x007e3e30`), called through `call`.
    const OP_OEF2_SET2: Global<u32> = Global::new(0x007e_3e30);
    /// `opu_et_unit(oef2, a, b, c)` (`0x007fd7f0`), called through `call`.
    const OPU_ET_UNIT: Global<u32> = Global::new(0x007f_d7f0);
    /// `g_OpusEncount`: the encounter effect's oef2 object
    /// (`op_et_encount_effect_on`).
    pub const ENCOUNT: Global<u32> = Global::new(0x0119_fe98);

    /// `op_et_oef2_free(oef2)` (`0x007fc2a0`) and `opu_et_oef2_free(oef2)`
    /// (`0x007fda60`): `opu_unit_free_by_oef2(g_OpusBattle, oef2)` through
    /// `call`; returns what it returns.
    pub fn op_et_oef2_free<M: Mem>(
        mem: &mut M,
        oef2: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = OPU_UNIT_FREE_BY_OEF2.ptr(mem).addr;
        let b = OPUS_BATTLE.get(mem);
        call(mem, f, &[b, oef2])
    }

    /// `op_et_oef2_start(oef2)` (`0x007fc2c0`, and `0x007fc970`):
    /// `op_oef2_set2(oef2)` and `opu_et_unit(oef2, 0, 0, -1)` through
    /// `call`; returns the latter's result.
    pub fn op_et_oef2_start<M: Mem>(
        mem: &mut M,
        oef2: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = OP_OEF2_SET2.ptr(mem).addr;
        call(mem, f, &[oef2]);
        let f = OPU_ET_UNIT.ptr(mem).addr;
        call(mem, f, &[oef2, 0, 0, u32::MAX])
    }

    /// `op_et_encount_effect_free()` (`0x007fc5f0`): `g_OpusEncount`'s
    /// units freed (`op_et_oef2_free`) and the object cleared; returns 0
    /// when there is none, else the free's result.
    pub fn op_et_encount_effect_free<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let e = ENCOUNT.get(mem);
        if e == 0 {
            return 0;
        }
        let r = op_et_oef2_free(mem, e, call);
        ENCOUNT.set(mem, 0);
        r
    }

    /// `op_et_monster_add(oef2)` (`0x007fc8a0`): -1 for null, 0 when it is
    /// one of the 8 `g_OpusMonsters` already or none is free; else it takes
    /// the first free one, is started (`op_et_oef2_start`) and 1 returned.
    pub fn op_et_monster_add<M: Mem>(
        mem: &mut M,
        oef2: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if oef2 == 0 {
            return u32::MAX;
        }
        let at = |mem: &M, i: u32| crate::opus::bridge::MONSTERS.ptr(mem).addr + 4 * i;
        if (0..8).any(|i| mem.read::<u32>(at(mem, i)) == oef2) {
            return 0;
        }
        let Some(i) = (0..8).find(|&i| mem.read::<u32>(at(mem, i)) == 0) else {
            return 0;
        };
        mem.write(at(mem, i), oef2);
        op_et_oef2_start(mem, oef2, call);
        1
    }

    /// `opu_unit_free_by_id(opus, id)` (`0x007fde30`), called through
    /// `call`.
    const OPU_UNIT_FREE_BY_ID: Global<u32> = Global::new(0x007f_de30);

    /// `op_et_bindeff2_off(h)` (`0x007fb6d0`): the character's bind flags
    /// lose 0x730 and its `op_et_bindeff2` unit (ID `0x50001800 + index`)
    /// is freed (`opu_unit_free_by_id(g_OpusBattle, id)` through `call`).
    /// Returns what the original leaves in `eax`: the free's result, the
    /// end of `g_EtChr` for an unknown handle, 0 for null (the original's
    /// is undefined there).
    pub fn op_et_bindeff2_off<M: Mem>(
        mem: &mut M,
        h: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if h == 0 {
            return 0;
        }
        let Some(i) = and_flags(mem, h, 0xf8cf) else {
            return CHR.ptr(mem).at(CHRS).addr;
        };
        let f = OPU_UNIT_FREE_BY_ID.ptr(mem).addr;
        let b = OPUS_BATTLE.get(mem);
        call(mem, f, &[b, 0x5000_1800 + i])
    }

    /// `opu_et_unit_free_by_id(id)` (`0x007fda40`):
    /// `opu_unit_free_by_id(g_OpusBattle, id)` through `call`; returns its
    /// result.
    pub fn opu_et_unit_free_by_id<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = OPU_UNIT_FREE_BY_ID.ptr(mem).addr;
        let b = OPUS_BATTLE.get(mem);
        call(mem, f, &[b, id])
    }

    /// `opu_et_unit_ofs(oef2, a, b, c)` (`0x007fda30`):
    /// `opu_et_unit(oef2 + 0x10, a, b, c)` (a tail call, through `call`);
    /// returns its result.
    pub fn opu_et_unit_ofs<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = OPU_ET_UNIT.ptr(mem).addr;
        call(
            mem,
            f,
            &[args[0].wrapping_add(0x10), args[1], args[2], args[3]],
        )
    }

    /// `op_et_eff(oef2, a, kind, id)` (`0x007fc2e0`), called through
    /// `call`.
    const OP_ET_EFF: Global<u32> = Global::new(0x007f_c2e0);
    /// `g_OpEtBindOff` (`0x011a0065`): while set, `op_et_bindeff` does
    /// nothing.
    pub const BIND_OFF: Global<u8> = Global::new(0x011a_0065);

    /// `op_et_bindeff(h)` (`0x007fb290`): unless `g_OpEtBindOff` is set,
    /// a known character without bind flag 0 whose oef2 object (record
    /// `+8`) is one of `g_OpusMonsters` gets its four bind effects
    /// (`op_et_eff(oef2, 0, k, 0x40000k00 + index)` for k = 4..7, through
    /// `call`) and bind flags `& 0xfffd | 1`. Returns what the original
    /// leaves in `eax` where that is defined (`None` for null `h` or with
    /// the flag set): the end of `g_EtChr` for an unknown handle, the
    /// record when it is skipped, the monster index when that is negative,
    /// else the last `op_et_eff` result's high half with the new flags.
    pub fn op_et_bindeff<M: Mem>(
        mem: &mut M,
        h: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        if BIND_OFF.get(mem) != 0 || h == 0 {
            return None;
        }
        let Some(i) = find_chr(mem, h) else {
            return Some(CHR.ptr(mem).at(CHRS).addr);
        };
        let rec = chr(mem, i);
        let flags: u16 = mem.read(rec + 0xc);
        let oef: u32 = mem.read(rec + 8);
        if flags & 1 != 0 || oef == 0 {
            return Some(rec);
        }
        let m = crate::opus::bridge::opus_get_monster_index(mem, oef);
        if m < 0 {
            return Some(m.cast_unsigned());
        }
        let f = OP_ET_EFF.ptr(mem).addr;
        let mut last = 0;
        for k in 4..8u32 {
            last = call(mem, f, &[oef, 0, k, 0x4000_0000 + (k << 8) + i]);
        }
        let flags = (mem.read::<u16>(rec + 0xc) & 0xfffd) | 1;
        mem.write(rec + 0xc, flags);
        Some((last & 0xffff_0000) | u32::from(flags))
    }

    /// `op_et_bindeff_off(h)` (`0x007fbb90`): bind flags 0 and 1 cleared,
    /// the four bind effect units (`0x40000k00 + index`, k = 4..7) freed
    /// (`opu_unit_free_by_id(g_OpusBattle, id)` through `call`).
    pub fn op_et_bindeff_off<M: Mem>(
        mem: &mut M,
        h: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let Some(i) = and_flags(mem, h, 0xfffc) else {
            return;
        };
        let f = OPU_UNIT_FREE_BY_ID.ptr(mem).addr;
        for k in 4..8u32 {
            let b = OPUS_BATTLE.get(mem);
            call(mem, f, &[b, 0x4000_0000 + (k << 8) + i]);
        }
    }

    /// `g_OpusWeaponEffCount` (`0x0119fef8`): a byte per weapon slot,
    /// numbering its effect units.
    pub const WEAPON_EFF_COUNT: Global<u8> = Global::new(0x0119_fef8);

    /// `op_et_weapon_unit(h)` (`0x007fc9b0`): for a known character, the
    /// weapon object whose number is its record's `u16` `+0xe` (else the
    /// record's oef2 at `+8`, slot 0) runs unit 9 with ID `(0x300000 +
    /// count) << 8 | index` (`opu_et_unit(obj, 0, 9, id)` through `call`),
    /// the slot's count one more; returns the ID, 0 for an unknown handle.
    pub fn op_et_weapon_unit<M: Mem>(
        mem: &mut M,
        h: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let Some(i) = find_chr(mem, h) else {
            return 0;
        };
        let rec = chr(mem, i);
        let no = u32::from(mem.read::<u16>(rec + 0xe));
        let weapon = crate::opus::bridge::opus_get_weapon_by_no(mem, no);
        let (obj, slot) = if weapon == 0 {
            (mem.read::<u32>(rec + 8), 0)
        } else {
            (
                weapon,
                crate::opus::bridge::opus_get_weapon_index(mem, weapon).cast_unsigned(),
            )
        };
        let at = WEAPON_EFF_COUNT.ptr(mem).addr.wrapping_add(slot);
        let count: u8 = mem.read(at);
        let id = ((u32::from(count) + 0x30_0000) << 8).wrapping_add(i);
        mem.write(at, count.wrapping_add(1));
        let f = OPU_ET_UNIT.ptr(mem).addr;
        call(mem, f, &[obj, 0, 9, id]);
        id
    }

    /// `opus_chr_handles_clear()` (`0x007fedf0`): the 240 `g_EtChrHandles`
    /// zeroed.
    pub fn opus_chr_handles_clear(mem: &mut impl Mem) {
        let at = CHR_HANDLES.ptr(mem).addr;
        for i in 0..0xf0 {
            mem.write(at + 4 * i, 0u32);
        }
    }

    /// `0x02321580` bit 1 with debug mode: use the debug character's
    /// resident data (`0x023215c8`) instead of `Ch_EffectGetResidentPtr`.
    pub const DEBUG_CHR_FLAGS: Global<u8> = Global::new(0x0232_1580);
    /// The debug character's resident data pointer.
    pub const DEBUG_CHR_RESIDENT: Global<u32> = Global::new(0x0232_15c8);

    /// `opus_get_chr_status_index(chr, no, add)` (`0x007fe970`): the
    /// `g_EtChr` record of character `chr` (-1 for null); when there is
    /// none and `add` is set, a free record takes it: its resident data
    /// (`Ch_EffectGetResidentPtr(chr)`, or the debug character's), a
    /// handle slot (`g_EtChrHandles`: its own, else the first free from
    /// 0x20 up with resident data, the last free from 0xef down without),
    /// `+0` = `chr`, `+4` = `no`, `+8` = the resident data, `+0xc` = 0,
    /// `+0xe` = the slot, both draw-flag bytes of the slot cleared. -1 when
    /// no record or slot is free.
    pub fn opus_get_chr_status_index<M: Mem>(
        mem: &mut M,
        chr: u32,
        no: u32,
        add: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        if chr == 0 {
            return -1;
        }
        let rec = |mem: &M, i: u32| CHR.ptr(mem).addr.wrapping_add(0x20 * i);
        if let Some(i) = (0..CHRS).find(|&i| mem.read::<u32>(rec(mem, i)) == chr) {
            return i.cast_signed();
        }
        if add == 0 {
            return -1;
        }
        let res =
            if crate::op::bridge::DEBUG_MODE.get(mem) != 0 && DEBUG_CHR_FLAGS.get(mem) & 2 != 0 {
                DEBUG_CHR_RESIDENT.get(mem)
            } else {
                let f = Global::<u32>::new(0x0082_a9b0).ptr(mem).addr;
                call(mem, f, &[chr])
            };
        let Some(free) = (0..CHRS).find(|&i| mem.read::<u32>(rec(mem, i)) == 0) else {
            return -1;
        };
        let handle = |mem: &M, k: u32| CHR_HANDLES.ptr(mem).at(k).get(mem);
        let slot = (0..0xf0).find(|&k| handle(mem, k) == chr).or_else(|| {
            if res == 0 {
                (0x20..0xf0).rev().find(|&k| handle(mem, k) == 0)
            } else {
                (0x20..0xf0).find(|&k| handle(mem, k) == 0)
            }
        });
        let Some(k) = slot else { return -1 };
        CHR_HANDLES.ptr(mem).at(k).set(mem, chr);
        let r = rec(mem, free);
        mem.write(r.wrapping_add(4), no);
        mem.write(r, chr);
        mem.write(r.wrapping_add(0xc), 0u16);
        #[allow(clippy::cast_possible_truncation)] // below 0xf0
        mem.write(r.wrapping_add(0xe), k as u16);
        mem.write(r.wrapping_add(8), res);
        DRAW_FLAGS.ptr(mem).at(k).set(mem, 0);
        DRAW_FLAGS2.ptr(mem).at(k).set(mem, 0);
        free.cast_signed()
    }

    /// `opu_unit_free_by_oef2(list, oef)` (`0x007fdf70`): the oef2 slot `k`
    /// of `oef` in the list (`+0x60`, 16 pointers; nothing when absent or
    /// `oef` is null) is released: every `g_EtEff` record of group `k`
    /// (`+0x1c`) freed (`opu_eff_free_by_id(+0)`), every slot holding
    /// `oef` cleared, and every unit of group `k` (`+0x1e`) in the index
    /// lists `+0x10`, `+0x14`, `+0x18` shut down (`opus_prog_off`,
    /// `opus_draw_off`, `yonExtMapDeleteId`; the list entry is read again
    /// in place, as `opus_prog_off` moves the rest down). Callees through
    /// `call`.
    pub fn opu_unit_free_by_oef2<M: Mem>(
        mem: &mut M,
        list: u32,
        oef: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if oef == 0 {
            return;
        }
        let slots = list.wrapping_add(0x60);
        let Some(k) = (0..16u32).find(|&i| mem.read::<u32>(slots.wrapping_add(4 * i)) == oef)
        else {
            return;
        };
        #[allow(clippy::cast_possible_truncation)] // below 16
        let group = k as u8;
        let free = Global::<u32>::new(0x007f_e920).ptr(mem).addr;
        for i in 0..EFFS {
            let e = EFF.ptr(mem).at(i).addr;
            if mem.read::<u8>(e + 0x1c) == group {
                let id: u32 = mem.read(e);
                call(mem, free, &[id]);
            }
        }
        for i in 0..16u32 {
            let at = slots.wrapping_add(4 * i);
            if mem.read::<u32>(at) == oef {
                mem.write(at, 0u32);
            }
        }
        let calls =
            [0x0080_14c0u32, 0x0080_0d10, 0x0080_7830].map(|f| Global::<u32>::new(f).ptr(mem).addr);
        for at in [0x10u32, 0x14, 0x18] {
            let mut e: u32 = mem.read(list.wrapping_add(at));
            loop {
                let u: u16 = mem.read(e);
                if u & 0x8000 != 0 {
                    break;
                }
                let no = i32::from(u.cast_signed()).cast_unsigned();
                let base: u32 = mem.read(list.wrapping_add(0x20));
                if mem.read::<u8>(no.wrapping_shl(8).wrapping_add(base).wrapping_add(0x1e)) == group
                {
                    call(mem, calls[0], &[list, u32::from(u)]);
                    call(mem, calls[1], &[list, u32::from(u)]);
                    call(mem, calls[2], &[no]);
                    e = e.wrapping_sub(2);
                }
                e = e.wrapping_add(2);
            }
        }
    }

    /// `0x007fead0(chr)`: a character's effect slot: with any of the top
    /// four bits, the low byte; else the first of the 240 [`CHR`] records
    /// whose draw flag index (`u16 +0xe`) is the low byte, or failing that
    /// the first with a handle (`+0`), or -1.
    pub fn op_et_chr_slot(mem: &impl Mem, chr: u32) -> u32 {
        if chr & 0xf000_0000 != 0 {
            return chr & 0xff;
        }
        let base = CHR.ptr(mem).addr;
        let want = u16::try_from(chr & 0xff).unwrap_or(0);
        for i in 0..CHRS {
            if mem.read::<u16>(base + 0x20 * i + 0xe) == want {
                return i;
            }
        }
        for i in 0..CHRS {
            if mem.read::<u32>(base + 0x20 * i) != 0 {
                return i;
            }
        }
        u32::MAX
    }
}
