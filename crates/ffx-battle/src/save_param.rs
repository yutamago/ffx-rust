//! The characters' saved parameters (`docs/rewrite-plan.md`, M2):
//! `MsSetSaveParam` (`0x007860f0`) works out a character's maximum HP and
//! MP and its stats from the save, the sphere grid (`g_BtlAbilityMap`) and
//! its weapons' auto-abilities, an aeon's from Yuna's and the battles
//! fought; `MsSetSaveData` writes the battle's actors back into the save
//! at a battle's end; `MsWeaponRead` swaps a weapon mid-battle.
//!
//! The weapon models' transparency (`Ch_SetTransparent`) is chara's and
//! `Sg_FreeA` the system's: those go to the game's code through `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, records, stats
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};
    use ffx_kernel::bridge::{excel_data, ms_check_range};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::save::bridge::{PLY_SAVE, ms_get_save_weapon};
    use crate::target::bridge::Dm;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `Ch_SetTransparent(chr, alpha, frames)` (chara).
    pub const CH_SET_TRANSPARENT: u32 = 0x0082_b900;
    /// `Sg_FreeA(p, kind)` (system).
    pub const SG_FREE_A: u32 = 0x0083_f090;

    /// The aeons' stat floors by battles fought (`g_KernelSumAssure`, the
    /// kernel's `sum_assure.bin`, directory 0xd file 0xe of `MsBattleInit`):
    /// records of `u16` HP and MP, then bytes.
    pub const SUM_ASSURE: Global<u32> = Global::new(0x0112_a948);
    /// The record of one character in the save.
    pub const PLY_SAVE_SIZE: u32 = 0x94;

    fn ply(mem: &impl Mem, i: u32) -> u32 {
        PLY_SAVE
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(PLY_SAVE_SIZE))
    }

    /// `0x007987f0(i)` (`MsGetAbilityMap`, name ours): character `i`'s
    /// sphere grid totals ([`crate::minor::bridge::ABILITY_MAP`], 0x1c
    /// bytes each), null from 8 on.
    pub fn ms_get_ability_map(mem: &impl Mem, i: u32) -> u32 {
        if i > 7 {
            return 0;
        }
        crate::minor::bridge::ABILITY_MAP
            .ptr(mem)
            .addr
            .wrapping_add(i * 0x1c)
    }

    /// [`ms_get_chr_ability_map`] without the pointer: the stats (eight
    /// bytes' worth, then HP and MP) and the map.
    pub fn chr_ability_map(mem: &impl Mem, i: u32) -> ([i32; 10], u32) {
        let p = ply(mem, i);
        let map = ms_get_ability_map(mem, i);
        let mut v = [0i32; 10];
        v[8] = mem.read(p.wrapping_add(4));
        v[9] = mem.read(p.wrapping_add(8));
        for (k, s) in (0u32..).zip(v.iter_mut().take(8)) {
            *s = i32::from(byte(mem, p, 0xc + k));
        }
        if map != 0 {
            v[8] = v[8].wrapping_add(mem.read::<i32>(map).wrapping_mul(0x32));
            v[9] = v[9].wrapping_add(mem.read::<i32>(map.wrapping_add(4)).wrapping_mul(5));
            for (k, s) in (0u32..).zip(v.iter_mut().take(8)) {
                *s = s.wrapping_add(i32::from(byte(mem, map, 8 + k)));
            }
        }
        (v, map)
    }

    /// `MsGetChrAbilityMap(i, out)` (`0x00785b60`): character `i`'s base
    /// stats and HP and MP from the save (`+0xc`-`+0x13`, `+4`, `+8`) with
    /// its sphere grid's added (HP nodes × 50, MP × 5), as ten words at
    /// `out` (stats, HP, MP); the map returned.
    pub fn ms_get_chr_ability_map(mem: &mut impl Mem, i: u32, out: u32) -> u32 {
        let (v, map) = chr_ability_map(mem, i);
        for (k, x) in (0u32..).zip(v) {
            let at = if k < 8 { 4 * k } else { 0x20 + 4 * (k - 8) };
            mem.write(out.wrapping_add(at), x);
        }
        map
    }

    /// An aeon's stats before its own save bonuses (`MsSetSaveParam` from
    /// 8 on): from Yuna's (character 1) HP, MP and stats by the aeon's
    /// `ply_rom` record, raised to the floors for the battles fought
    /// ([`SUM_ASSURE`], one step every 30 up to 20).
    fn aeon_stats(mem: &mut impl Mem, i: u32) -> [i32; 10] {
        let rom = crate::menu::bridge::PLY_ROM.get(mem);
        let (rec, _) = excel_data(mem, i.cast_signed(), rom);
        let (y, _) = chr_ability_map(mem, 1);
        let hp = ms_check_range(y[8], 0, 9999);
        let mp = ms_check_range(y[9], 0, 999);
        let st: [i32; 8] = std::array::from_fn(|k| {
            let lo = i32::from(k != 6);
            ms_check_range(y[k], lo, 0xff)
        });
        let r = |mem: &mut _, o: u32| i32::from(byte(mem, rec, o));
        let t = (hp / 100)
            .wrapping_add(st[7])
            .wrapping_add(mp / 10)
            .wrapping_add(st[6])
            .wrapping_add(st[4])
            .wrapping_add(st[3])
            .wrapping_add(st[2])
            .wrapping_add(st[1])
            .wrapping_add(st[0]);
        // (a zero divisor faults in the original; never in the game's data)
        let div = |t: i32, d: i32| if d == 0 { 0 } else { t.wrapping_div(d) };
        let mut hpn = r(mem, 0x19)
            .wrapping_mul(hp)
            .wrapping_div(100)
            .wrapping_add(r(mem, 0x18).wrapping_mul(t));
        let mut mpn = r(mem, 0x1b)
            .wrapping_mul(mp)
            .wrapping_div(100)
            .wrapping_add(r(mem, 0x1a).wrapping_mul(t).wrapping_div(10));
        // the stats but luck (the sixth, Yuna's)
        let mut a = [0i32; 8];
        for (k, o) in [
            (0, 0x1c),
            (1, 0x1e),
            (2, 0x20),
            (3, 0x22),
            (4, 0x24),
            (6, 0x26),
            (7, 0x28),
        ] {
            a[k] =
                div(t, r(mem, o)).wrapping_add(r(mem, o + 1).wrapping_mul(st[k]).wrapping_div(10));
        }
        a[5] = st[5];
        let n = crate::save::bridge::BATTLE_COUNT.get(mem) / 0x1e;
        if n.cast_signed() >= 1 {
            let n = n.min(0x14).cast_signed();
            let id = i
                .cast_signed()
                .wrapping_add(n.wrapping_mul(5).wrapping_sub(9).wrapping_mul(2));
            let grow = SUM_ASSURE.get(mem);
            let (q, _) = excel_data(mem, id, grow);
            hpn = hpn.max(i32::from(mem.read::<u16>(q)));
            mpn = mpn.max(i32::from(mem.read::<u16>(q.wrapping_add(2))));
            for (k, o) in [(0, 4), (1, 5), (2, 6), (3, 7), (7, 8), (6, 9), (4, 0xa)] {
                a[k] = a[k].max(i32::from(byte(mem, q, o)));
            }
        }
        let p = ply(mem, i);
        let mut v = [0i32; 10];
        v[8] = hpn.wrapping_add(mem.read::<i32>(p.wrapping_add(4)));
        v[9] = mpn.wrapping_add(mem.read::<i32>(p.wrapping_add(8)));
        for (k, s) in (0u32..).zip(v.iter_mut().take(8)) {
            *s = a[k as usize].wrapping_add(i32::from(byte(mem, p, 0xc + k)));
        }
        v
    }

    /// `MsSetSaveParam(i)` (`0x007860f0`): character `i`'s maximum HP
    /// (`+0x24`, up to 9999 or 99999 with Break HP Limit, `+0x4c` bit 9)
    /// and MP (`+0x28`, 999 or 9999, bit 10), stats (`+0x2f`-`+0x36`, 1-255,
    /// Luck from 0) and its current HP and MP clamped to them: below 8 the
    /// sphere grid's ([`ms_get_chr_ability_map`], its learnt commands added
    /// into `+0x3e`-`+0x48`), from 8 an aeon's (`aeon_stats`); Kimahri (5)
    /// learns the commands his Lancet ones give (`0x3041`-`0x3053` →
    /// `+0x37`). The weapons' (`+0x2d`, `+0x2e`) auto-abilities set the
    /// ability bits `+0x4a`-`+0x4f` and raise the fourteen values by their
    /// percentages; the last four of those are the damage rates
    /// ([`crate::minor::bridge::DAMAGE_RATES`]). Joined with no encounters
    /// (`+0x4e` bit 1) clears [`crate::minor::bridge::ENCOUNT_ON`]. EAX as
    /// the original leaves it.
    pub fn ms_set_save_param(mem: &mut impl Mem, i: u32) -> u32 {
        let p = ply(mem, i);
        let mut l = [0i32; 14];
        let mut boost = [0i32; 14];
        if i.cast_signed() < 8 {
            let (v, map) = chr_ability_map(mem, i);
            l[..10].copy_from_slice(&v);
            if map != 0 {
                for k in 0..6 {
                    let at = p.wrapping_add(0x3e + 2 * k);
                    let w = mem.read::<u16>(at) | mem.read::<u16>(map.wrapping_add(0x10 + 2 * k));
                    mem.write(at, w);
                }
            }
        } else {
            l[..10].copy_from_slice(&aeon_stats(mem, i));
        }
        if i == 5 {
            for c in 0x3041..=0x3053 {
                if crate::save::bridge::ms_get_save_command(mem, 5, c) != 0 {
                    crate::ability::bridge::ms_set_save_command(mem, 5, c + 0x37, 1);
                }
            }
        }
        mem.write(p.wrapping_add(0x4a), 0u32);
        mem.write(p.wrapping_add(0x4e), 0u16);
        for k in 0..2 {
            let w = ms_get_save_weapon(mem, u32::from(byte(mem, p, 0x2d + k)), 0);
            if w == 0 {
                continue;
            }
            for s in 0..4 {
                let ab = u32::from(mem.read::<u16>(w.wrapping_add(0xe + 2 * s)));
                if ab == 0 || ab == 0xff {
                    continue;
                }
                let table = ffx_kernel::bridge::A_ABILITY.get(mem);
                let (r, _) = excel_data(mem, (ab & 0xfff).cast_signed(), table);
                for o in [0, 2, 4] {
                    let at = p.wrapping_add(0x4a + o);
                    let v = mem.read::<u16>(at) | mem.read::<u16>(r.wrapping_add(0x62 + o));
                    mem.write(at, v);
                }
                let bits = mem.read::<u16>(r.wrapping_add(0x56));
                let pct = i32::from(byte(mem, r, 0x55));
                for (b, x) in boost.iter_mut().enumerate() {
                    if bits & (1 << b) != 0 {
                        *x = x.wrapping_add(pct);
                    }
                }
            }
        }
        for (x, b) in l.iter_mut().zip(boost) {
            if b != 0 {
                *x = x.wrapping_mul(b).wrapping_div(100).wrapping_add(*x);
            }
        }
        let rates = crate::minor::bridge::DAMAGE_RATES
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(4));
        for k in 0..4 {
            mem.write(
                rates.wrapping_add(k),
                boost[10 + k as usize].to_le_bytes()[0],
            );
        }
        let flags = mem.read::<u16>(p.wrapping_add(0x4c));
        let hp_max = if flags & 0x200 != 0 { 99999 } else { 9999 };
        let mp_max = if flags & 0x400 != 0 { 9999 } else { 999 };
        let hp = ms_check_range(l[8], 0, hp_max);
        mem.write(p.wrapping_add(0x24), hp);
        let mp = ms_check_range(l[9], 0, mp_max);
        mem.write(p.wrapping_add(0x28), mp);
        for k in 0..8u32 {
            let lo = i32::from(k != 6);
            let v = ms_check_range(l[k as usize], lo, 0xff);
            mem.write(p.wrapping_add(0x2f + k), v.to_le_bytes()[0]);
        }
        let cur = ms_check_range(mem.read(p.wrapping_add(0x1c)), 0, hp);
        mem.write(p.wrapping_add(0x1c), cur);
        let cur = ms_check_range(mem.read(p.wrapping_add(0x20)), 0, mp);
        mem.write(p.wrapping_add(0x20), cur);
        if crate::save::bridge::ms_get_save_ply_join(mem, i) == 0 {
            return 0;
        }
        let keep: u8 = if byte(mem, p, 0x4e) & 2 != 0 { 0 } else { 0xff };
        let on = crate::minor::bridge::ENCOUNT_ON;
        on.set(mem, on.get(mem) & keep);
        u32::from(keep)
    }

    /// `MsSetSaveParamAll()` (`0x00786900`): encounters on, every
    /// character's parameters ([`ms_set_save_param`], 0-0x11); the last's
    /// EAX.
    pub fn ms_set_save_param_all(mem: &mut impl Mem) -> u32 {
        crate::minor::bridge::ENCOUNT_ON.set(mem, 1);
        let mut r = 0;
        for i in 0..0x12 {
            r = ms_set_save_param(mem, i);
        }
        r
    }

    /// `MsSetSaveData()` (`0x00785fc0`): at a battle's end the party back
    /// (`MsReturnParty`), the battle and reserve parties saved, and each
    /// of the 18 characters' weapons (`+0x592`, `+0x593`), overdrive
    /// (`+0x5bb`-`+0x5bd`) and HP and MP (`+0x5d0`, `+0x5d4`; 1 HP for the
    /// KO'd, full after the stand-in count `+0x6d8` runs out, its rest at
    /// `+0x3d`) into the save; the aeons' parameters
    /// ([`ms_set_save_param`] 8-0x11). The last's EAX.
    pub fn ms_set_save_data(mem: &mut impl Mem) -> u32 {
        crate::flow::bridge::ms_return_party(mem);
        crate::save::bridge::ms_set_save_party(mem);
        let out = crate::flow::bridge::PARTY_EXTRA.get(mem);
        crate::save::bridge::OUT_PARTY
            .ptr(mem)
            .cast::<[u8; 17]>()
            .set(mem, out);
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            let s = ply(mem, i).wrapping_add(0x1c);
            mem.write(s.wrapping_add(0x11), byte(mem, a, 0x592));
            mem.write(s.wrapping_add(0x12), byte(mem, a, 0x593));
            let hp: i32 = mem.read(a.wrapping_add(0x5d0));
            let mut left = i32::from(byte(mem, a, 0x6d8));
            mem.write(s, if hp <= 0 && left == 0 { 1 } else { hp });
            mem.write(s.wrapping_add(4), mem.read::<u32>(a.wrapping_add(0x5d4)));
            if left != 0 {
                left -= 1;
                if left <= 0 {
                    mem.write(s, mem.read::<u32>(s.wrapping_add(8)));
                    left = 0;
                    mem.write(s.wrapping_add(4), mem.read::<u32>(s.wrapping_add(0xc)));
                }
            }
            mem.write(s.wrapping_add(0x21), left.to_le_bytes()[0]);
            let v = ms_check_range(mem.read(s), 0, mem.read(s.wrapping_add(8)));
            mem.write(s, v);
            let v = ms_check_range(
                mem.read(s.wrapping_add(4)),
                0,
                mem.read(s.wrapping_add(0xc)),
            );
            mem.write(s.wrapping_add(4), v);
            for k in 0..3 {
                mem.write(s.wrapping_add(0x1c + k), byte(mem, a, 0x5bb + k));
            }
        }
        let mut r = 0;
        for i in 8..0x12 {
            r = ms_set_save_param(mem, i);
        }
        r
    }

    /// `MsEquipSaveWeapon(i, k, id)` (`0x007ab970`): character `i`'s
    /// weapon `k` (`+0x2d`, `+0x2e`) becomes `id & 0xfff`: the old one's
    /// owner (`+6`) cleared, the new one's set (none for 0xff). The
    /// original writes through a missing equipment record unchecked.
    pub fn ms_equip_save_weapon(mem: &mut impl Mem, i: u32, k: u32, id: u32) {
        let i = i & 0xff;
        let p = ply(mem, i);
        let id = id & 0xfff;
        let slot = p.wrapping_add(if k == 0 { 0x2d } else { 0x2e });
        let old = u32::from(mem.read::<u8>(slot));
        if old != 0xff {
            let w = ms_get_save_weapon(mem, old, 0);
            mem.write(w.wrapping_add(6), 0xffu8);
        }
        if id != 0xff {
            let w = ms_get_save_weapon(mem, id, 0);
            mem.write(w.wrapping_add(6), i.to_le_bytes()[0]);
        }
        mem.write(slot, id.to_le_bytes()[0]);
    }

    /// `0x007ad570(i, k, frames)` (`MsWeaponFadeIn`, name ours): weapon
    /// `k`'s model of actor `i` turns clear, then opaque over `frames`
    /// (`Ch_SetTransparent`, 0.0 then 1.0).
    pub fn ms_weapon_fade_in<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, k: u32, frames: u32) {
        let w = crate::actor::bridge::ms_get_weapon(mem, i, k.cast_signed());
        let chr: u32 = mem.read(w);
        if chr == 0 {
            return;
        }
        game(mem, &mut *call, CH_SET_TRANSPARENT, &[chr, 0, 0]);
        game(
            mem,
            call,
            CH_SET_TRANSPARENT,
            &[chr, 1.0f32.to_bits(), frames],
        );
    }

    /// `MsWeaponRead(i, k, id)` (`0x007ad820`): in effect state 9 (a weapon
    /// change), actor `i`'s weapon `k` becomes `id & 0xfff` (the save's
    /// [`ms_equip_save_weapon`], `+0x592 + k`, the parameters again), state
    /// 10 and no effect. The same model fades in; another is disposed of
    /// (`+0xdfd` set until it loads): for the first weapon its effects,
    /// sounds and motion data (`+0x48`) go too, and the new model's reads
    /// are queued.
    pub fn ms_weapon_read<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        k: u32,
        id: u32,
    ) {
        use crate::exchange::bridge::ms_get_save_weapon_model;
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let id = id & 0xfff;
        if byte(mem, a, 0xdfb) != 9 {
            return;
        }
        let slot = a.wrapping_add(0x592).wrapping_add(k);
        let old = ms_get_save_weapon_model(mem, u32::from(mem.read::<u8>(slot)));
        let new = ms_get_save_weapon_model(mem, id);
        ms_equip_save_weapon(mem, i, k, id);
        mem.write(slot, id.to_le_bytes()[0]);
        let w = crate::actor::bridge::ms_get_weapon(mem, i, k.cast_signed());
        mem.write(a.wrapping_add(0xdfb), 0xau8);
        mem.write(a.wrapping_add(0xf8c), 0u32);
        ms_set_save_param(mem, i);
        if old == new {
            mem.write(a.wrapping_add(0xdfd), 0u8);
            ms_weapon_fade_in(mem, &mut *s.call, i, k, 8);
            return;
        }
        mem.write(a.wrapping_add(0xdfd), 1u8);
        crate::damage_end::bridge::ms_weapon_dispose(mem, s, i, k, 7, w);
        if k == 0 {
            crate::exchange::bridge::ms_effect_weapon_free(mem, s, i);
            crate::exec::bridge::ms_sep_chr_clear(mem, &mut *s.call, a, 1);
            let m: u32 = mem.read(a.wrapping_add(0x48));
            if m != 0 {
                game(mem, &mut *s.call, SG_FREE_A, &[m, 5]);
                mem.write(a.wrapping_add(0x48), 0u32);
            }
        }
        mem.write(w.wrapping_add(4), new);
        crate::exchange::bridge::ms_btl_read_cue_regist_weapon(mem, s, i, 1u32 << (k & 0x1f));
        if k == 0 {
            crate::exchange::bridge::ms_btl_read_cue_regist(mem, s, 3, i, i, 0, 9);
        }
    }
}
