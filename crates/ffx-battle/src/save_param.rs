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

    /// The lowest value of each of a party actor's eight stats (`+0x5a8`:
    /// the base at `+0x5b0` plus the save's `+0x2f` on, at most 255); the
    /// seventh (`+0x5ae`) may be 0.
    const STAT_FLOORS: [i32; 8] = [1, 1, 1, 1, 1, 1, 0, 1];

    /// `*at |= v` (a byte).
    fn or_byte(mem: &mut impl Mem, at: u32, v: u8) {
        let old: u8 = mem.read(at);
        mem.write(at, old | v);
    }

    /// `*at |= v` (a `u16`).
    fn or_word(mem: &mut impl Mem, at: u32, v: u16) {
        let old: u16 = mem.read(at);
        mem.write(at, old | v);
    }

    /// The `n` bytes at `to` plus those at `from`, each within `0..=hi`.
    fn add_bytes(mem: &mut impl Mem, to: u32, from: u32, n: u32, hi: i32) {
        for k in 0..n {
            let sum = i32::from(byte(mem, from, k)) + i32::from(byte(mem, to, k));
            let v = ms_check_range(sum, 0, hi).to_le_bytes()[0];
            mem.write(to.wrapping_add(k), v);
        }
    }

    /// One auto-ability record's (`g_KernelAAbility`) effects added to a
    /// party actor: flag bytes `+0x11`-`+0x15` into `+0x5d9`-`+0x5dd`;
    /// `+0x16` (25 bytes) and `+0x2f` (13) added to `+0x5de` and `+0x5f7`
    /// (each within 0-0xfe); `+0x5e` into `+0x604`; the words `+0x58`,
    /// `+0x5a`, `+0x5c` into `+0x630`-`+0x634`, or with bit 0 of `+0x10`
    /// into `+0x636`-`+0x63a` (and `+0x63c` set); `+0x3c` (25 bytes) added
    /// to `+0x641` (within 0-0xff); the words `+0x60`-`+0x66` into
    /// `+0x65a`, `+0x6bc`, `+0x6be`, `+0x6c0`.
    fn add_auto_ability(mem: &mut impl Mem, actor: u32, r: u32) {
        let at = |o: u32| actor.wrapping_add(o);
        for k in 0..5 {
            let v = byte(mem, r, 0x11 + k);
            or_byte(mem, at(0x5d9 + k), v);
        }
        add_bytes(mem, at(0x5de), r.wrapping_add(0x16), 0x19, 0xfe);
        add_bytes(mem, at(0x5f7), r.wrapping_add(0x2f), 0xd, 0xfe);
        let v: u16 = mem.read(r.wrapping_add(0x5e));
        or_word(mem, at(0x604), v);
        let elements = if byte(mem, r, 0x10) & 1 != 0 {
            mem.write(at(0x63c), 1u8);
            0x636
        } else {
            0x630
        };
        for k in 0..3 {
            let v: u16 = mem.read(r.wrapping_add(0x58 + 2 * k));
            or_word(mem, at(elements + 2 * k), v);
        }
        add_bytes(mem, at(0x641), r.wrapping_add(0x3c), 0x19, 0xff);
        for (to, from) in [(0x65a, 0x60), (0x6bc, 0x62), (0x6be, 0x64), (0x6c0, 0x66)] {
            let v: u16 = mem.read(r.wrapping_add(from));
            or_word(mem, at(to), v);
        }
    }

    /// `MsSetRamChrParam(chr)` (`0x0079c5f0`): party actor `chr` from its
    /// save record (`g_PlySave`, 0x94 bytes): maximum HP and MP (`+0x24`,
    /// `+0x28` into `+0x594`/`+0x598` and the bases `+0x59c`/`+0x5a0`), the
    /// stats (`+0x5a8`, [`STAT_FLOORS`]), the weapon and armour (`+0x2d`,
    /// `+0x2e` into `+0x592`/`+0x593`). The equipment fields cleared
    /// (formula 1 `+0x5c1`, power 0x10 `+0x5c7`), then for each piece the
    /// save holds (`MsGetSaveWeapon`): the weapon's formula and power
    /// (`+8`, `+9`), each piece's critical bonus (`+0xa`, a byte added to
    /// `+0x5d8`) and each of its four auto-abilities but 0 and 0xff
    /// ([`add_auto_ability`]). Then the overdrive statuses
    /// (`MsLimitStatusProcess`, kept) and HP, MP within the maxima; the MP
    /// is what the original leaves in `eax`.
    pub fn ms_set_ram_chr_param(mem: &mut impl Mem, chr: u32) -> i32 {
        let actor = ms_get_chr(mem, chr);
        let at = |o: u32| actor.wrapping_add(o);
        let save = PLY_SAVE
            .ptr(mem)
            .addr
            .wrapping_add(chr.wrapping_mul(PLY_SAVE_SIZE));
        let hp: u32 = mem.read(save.wrapping_add(0x24));
        let mp: u32 = mem.read(save.wrapping_add(0x28));
        for (o, v) in [(0x594, hp), (0x598, mp), (0x59c, hp), (0x5a0, mp)] {
            mem.write(at(o), v);
        }
        for (k, floor) in (0u32..).zip(STAT_FLOORS) {
            let sum = i32::from(byte(mem, actor, 0x5b0 + k)) + i32::from(byte(mem, save, 0x2f + k));
            let v = ms_check_range(sum, floor, 0xff).to_le_bytes()[0];
            mem.write(at(0x5a8 + k), v);
        }
        let equipment = [byte(mem, save, 0x2d), byte(mem, save, 0x2e)];
        mem.write(at(0x592), equipment[0]);
        mem.write(at(0x593), equipment[1]);
        let pieces = equipment.map(|id| ms_get_save_weapon(mem, id.into(), 0));
        mem.write(at(0x5c1), 1u8);
        mem.write(at(0x5c7), 0x10u8);
        // the ability fields: +0x5d8..+0x605, +0x62a..+0x63c,
        // +0x641..+0x65b, +0x6bc..+0x6c1
        for (from, to) in [
            (0x5d8, 0x606),
            (0x62a, 0x63d),
            (0x641, 0x65c),
            (0x6bc, 0x6c2),
        ] {
            mem.write_bytes(
                at(from),
                &vec![0; usize::try_from(to - from).unwrap_or_default()],
            );
        }
        for (k, &piece) in pieces.iter().enumerate() {
            if piece == 0 {
                continue;
            }
            if k == 0 {
                let formula = byte(mem, piece, 8);
                let power = byte(mem, piece, 9);
                mem.write(at(0x5c1), formula);
                mem.write(at(0x5c7), power);
            }
            let critical = byte(mem, actor, 0x5d8).wrapping_add(byte(mem, piece, 0xa));
            mem.write(at(0x5d8), critical);
            for slot in 0..4 {
                let ability: u16 = mem.read(piece.wrapping_add(0xe + 2 * slot));
                if ability == 0 || ability == 0xff {
                    continue;
                }
                let table = ffx_kernel::bridge::A_ABILITY.get(mem);
                let r = ffx_kernel::bridge::ms_get_excel_data(
                    mem,
                    i32::from(ability & 0xfff),
                    table,
                    0,
                );
                add_auto_ability(mem, actor, r);
            }
        }
        crate::flow::bridge::ms_limit_status_process(mem, actor, -1);
        let mut last = 0;
        for (value, max) in [(0x5d0, 0x594), (0x5d4, 0x598)] {
            let v: i32 = mem.read(at(value));
            let hi: i32 = mem.read(at(max));
            last = ms_check_range(v, 0, hi);
            mem.write(at(value), last);
        }
        last
    }

    /// The debug env menu's "party HP 1" toggle (`0x007c6e20`,
    /// `0x007ce450`, env item 0x24; `0x00784c80` sets every party actor's
    /// HP to 1 or back to the maximum): [`ms_set_ram_chr`] gives a new
    /// party actor HP 1 while it is set.
    pub const DEBUG_PARTY_HP1: Global<u8> = Global::new(0x0112_a91d);
    /// Its twin for the monsters (env item 0x25, `0x00784bc0`).
    pub const DEBUG_MONSTER_HP1: Global<u8> = Global::new(0x0112_a91e);
    /// The bytes [`ms_set_ram_chr`] clears from `+0x540` on.
    pub const RAM_CHR_SIZE: i32 = 0x1ec;
    /// One actor's 0x46 words of item memory (`g_BtlItemMem`).
    pub const ITEM_MEM_SIZE: u32 = 0x118;
    /// One actor's command list in `g_BtlItemMem2`: 0x14 `u16` IDs at the
    /// head of 0x478 bytes.
    pub const ITEM_MEM2_SIZE: u32 = 0x478;

    fn put<T: ffx_bridge::Value>(mem: &mut impl Mem, actor: u32, o: u32, v: T) {
        mem.write(actor.wrapping_add(o), v);
    }

    /// The low byte of `MsCheckRange(v, lo, 0xff)`.
    fn clamp_byte(v: u8, lo: i32) -> u8 {
        ms_check_range(i32::from(v), lo, 0xff).to_le_bytes()[0]
    }

    /// `MsSetRamChr`'s party half: the actor from the character's kernel
    /// record (`ply_rom`, `rom`) and save record (`save`).
    fn ram_party(mem: &mut impl Mem, chr: u32, actor: u32) -> u8 {
        let rom = ffx_kernel::bridge::ms_get_excel_data(
            mem,
            chr.cast_signed(),
            crate::save::bridge::KERNEL_PLY_ROM.get(mem),
            0,
        );
        let save = ply(mem, chr);
        let names = crate::save::bridge::SAVE_NAMES_AT
            .ptr(mem)
            .addr
            .wrapping_add(chr.wrapping_mul(0x14));
        for k in 0..0x14 {
            let c = byte(mem, names, k);
            put(mem, actor, 0x540 + k, c);
        }
        let v = byte(mem, rom, 0x10);
        put(mem, actor, 0x590, v);
        let v = byte(mem, save, 0x2c);
        put(mem, actor, 0x591, v);
        let v = clamp_byte(byte(mem, save, 0x37), 0);
        put(mem, actor, 0x5ba, v);
        put(mem, actor, 0x5b8, 0u16);
        put(mem, actor, 0x5c5, 0xffffu16);
        for k in 0..3 {
            let v = byte(mem, save, 0x38 + k);
            put(mem, actor, 0x5bb + k, v);
        }
        put(mem, actor, 0x5c8, 0u8);
        let v = byte(mem, rom, 0x2a);
        put(mem, actor, 0x5c9, v);
        put(mem, actor, 0x6d6, 0u16);
        let v = byte(mem, save, 0x3d);
        put(mem, actor, 0x6d8, v);
        let v = byte(mem, rom, 0x2b);
        put(mem, actor, 0x6d9, v);
        // the save's words +0x14, +0x1c (HP), +0x20 (MP), +0x24
        for (to, from) in [(0x5cc, 0x14), (0x5d0, 0x1c), (0x5d4, 0x20), (0x5a4, 0x24)] {
            let v: u32 = mem.read(save.wrapping_add(from));
            put(mem, actor, to, v);
        }
        tail_fields(mem, actor, 0x32, 0);
        let v: u32 = mem.read(actor.wrapping_add(0x5a4));
        put(mem, actor, 0xf60, v);
        put(mem, actor, 0x70c, 1.0f32);
        put(mem, actor, 0x5c2, chr.to_le_bytes()[0]);
        put(mem, actor, 0x5c3, 0u8);
        put(mem, actor, 0x712, 0xffu16);
        ms_set_ram_chr_param(mem, chr);
        let items = crate::misc::bridge::ITEM_MEM
            .get(mem)
            .wrapping_add(chr.wrapping_mul(ITEM_MEM_SIZE));
        put(mem, actor, 0xf88, items);
        put(mem, actor, 0x606, 0u16);
        put(mem, actor, 0x616, 0u16);
        put(mem, actor, 0x6c6, 0x3000u16);
        let summon = crate::damage::bridge::ms_get_ram_chr_summon(mem, chr) != 0;
        put(mem, actor, 0x6c8, if summon { 0u16 } else { 0x3021 });
        let flags: u16 = mem.read(actor.wrapping_add(0x5b8));
        put(mem, actor, 0x5b8, flags | 0x200);
        put(mem, actor, 0x6d3, 0xffffu16);
        for k in 0..11 {
            put(mem, actor, 0x690 + 4 * k, 0u32);
        }
        crate::flow::bridge::ms_set_chr_image_damage_attack(mem, actor);
        crate::ability::bridge::ms_set_ram_chr_ability(mem, chr, actor);
        crate::minor::bridge::ms_build_gear_list(mem, chr);
        if crate::state::bridge::UNDERWATER.get(mem) != 0
            || crate::save::bridge::STEAL_RULE.get(mem) == 2
        {
            crate::turn::bridge::ms_set_command_use(mem, chr, 0x3002, 1);
        }
        DEBUG_PARTY_HP1.get(mem)
    }

    /// The fields both halves set alike after HP and MP: their copies
    /// `+0x6e4`/`+0x6e8`, `+0x6fc` -1, `+0x6d1` 0xff, `+0x6e1` (`rate`:
    /// 0x32 party, 0 monsters) and `+0x719` (`monster`, 1 for monsters).
    fn tail_fields(mem: &mut impl Mem, actor: u32, rate: u8, monster: u8) {
        for (to, from) in [(0x6e4, 0x5d0), (0x6e8, 0x5d4)] {
            let v: u32 = mem.read(actor.wrapping_add(from));
            put(mem, actor, to, v);
        }
        put(mem, actor, 0x6fc, u32::MAX);
        put(mem, actor, 0x6d1, 0xffu8);
        put(mem, actor, 0x6e1, rate);
        put(mem, actor, 0x719, monster);
    }

    /// `MsSetRamChr`'s monster half with monster data (`+0x48` not null):
    /// the actor from its monster record `mon` (`+0xf80`) and the data's
    /// item memory (`+0x14`, the offset of 0x46 words).
    fn ram_monster_data(mem: &mut impl Mem, chr: u32, actor: u32, mon: u32, data: u32) {
        let v: u16 = mem.read(mon.wrapping_add(0x74));
        put(mem, actor, 8, u32::from(v));
        for k in 0..0x28 {
            let c = byte(mem, actor, 0x50 + k);
            put(mem, actor, 0x540 + k, c);
        }
        put(mem, actor, 0x592, 0xffffu16);
        for (to, from) in [(0x594, 0x14), (0x598, 0x18), (0x59c, 0x14), (0x5a0, 0x18)] {
            let v: u32 = mem.read(mon.wrapping_add(from));
            put(mem, actor, to, v);
        }
        // the stats: each at least 1, but +0x26 (from 0)
        for k in 0..8 {
            let lo = i32::from(k != 6);
            let v = clamp_byte(byte(mem, mon, 0x20 + k), lo);
            put(mem, actor, 0x5a8 + k, v);
        }
        let v = clamp_byte(byte(mem, mon, 0x2a), 0);
        put(mem, actor, 0x5ba, v);
        let v: u16 = mem.read(mon.wrapping_add(0x28));
        put(mem, actor, 0x5b8, v);
        put(mem, actor, 0x5c1, 1u8);
        put(mem, actor, 0x5c6, 0x10ffu16);
        put(mem, actor, 0x5c5, 0xffu8);
        put(mem, actor, 0x5bb, 0u16);
        put(mem, actor, 0x5bd, 100u8);
        put(mem, actor, 0x5c8, 0u8);
        let v = byte(mem, mon, 0x77);
        put(mem, actor, 0x5c9, v);
        let v: u16 = mem.read(mon.wrapping_add(0x78));
        put(mem, actor, 0x6d6, v);
        put(mem, actor, 0x6d8, 0u16);
        for (to, from) in [(0x5d0, 0x14), (0x5d4, 0x18), (0x5a4, 0x1c), (0xf60, 0x1c)] {
            let v: u32 = mem.read(mon.wrapping_add(from));
            put(mem, actor, to, v);
        }
        tail_fields(mem, actor, 0, 1);
        put(mem, actor, 0x5d9, 0u8);
        for k in 0..4 {
            let v = byte(mem, mon, 0x2b + k);
            put(mem, actor, 0x5da + k, v);
        }
        put(mem, actor, 0x62a, 0u16);
        put(mem, actor, 0x62e, 0u16);
        put(mem, actor, 0x63c, 0u8);
        for k in 0..3 {
            let v: u16 = mem.read(mon.wrapping_add(0x48 + 2 * k));
            put(mem, actor, 0x630 + 2 * k, v);
        }
        put(mem, actor, 0x636, 0u32);
        put(mem, actor, 0x63a, 0u16);
        // rep movsd: word by word, forward
        let from = data.wrapping_add(mem.read::<u32>(data.wrapping_add(0x14)));
        let items = crate::misc::bridge::ITEM_MEM
            .get(mem)
            .wrapping_add(chr.wrapping_mul(ITEM_MEM_SIZE));
        for k in 0..0x46 {
            let v: u32 = mem.read(from.wrapping_add(4 * k));
            mem.write(items.wrapping_add(4 * k), v);
        }
        put(mem, actor, 0xf88, items);
        for k in 0..0x19 {
            let sum = i32::from(byte(mem, mon, 0x2f + k)) + i32::from(byte(mem, actor, 0x641 + k));
            let v = ms_check_range(sum, 0, 0xff).to_le_bytes()[0];
            put(mem, actor, 0x641 + k, v);
        }
        let v: u16 = mem.read(mon.wrapping_add(0x4e));
        put(mem, actor, 0x65a, v);
        let v: u16 = mem.read(mon.wrapping_add(0x70));
        put(mem, actor, 0x6c6, v);
        put(mem, actor, 0x6c8, 0xffu16);
        put(mem, actor, 0x6d3, 0xffffu16);
        let v = byte(mem, mon, 0x76);
        put(mem, actor, 0x5c2, v);
        put(mem, actor, 0x5c3, 0u8);
    }

    /// `MsSetRamChr`'s monster half: the fields from the monster data when
    /// there is any, then its command list (`g_BtlItemMem2`): 0x3001 first,
    /// the record's 16 commands (`+0x50`) but the zero ones, 0xff to 0x14.
    fn ram_monster(mem: &mut impl Mem, chr: u32, actor: u32) -> u8 {
        let mon: u32 = mem.read(actor.wrapping_add(0xf80));
        let _ = crate::getmem::bridge::ms_get_save_config_english();
        let data: u32 = mem.read(actor.wrapping_add(0x48));
        if data != 0 {
            ram_monster_data(mem, chr, actor, mon, data);
        }
        put(mem, actor, 0x70c, 1.0f32);
        put(mem, actor, 0x712, 0xffu16);
        crate::flow::bridge::ms_set_chr_image_damage_attack(mem, actor);
        let head = chr.wrapping_mul(ITEM_MEM2_SIZE);
        let at = crate::misc::bridge::ITEM_MEM2.get(mem).wrapping_add(head);
        mem.write(at, 0x3001u16);
        let mut n = 1u32;
        let mut to = head.wrapping_add(2);
        for k in 0..16 {
            let id: u16 = mem.read(mon.wrapping_add(0x50 + 2 * k));
            if n < 0x14 && id != 0 {
                n += 1;
                let base = crate::misc::bridge::ITEM_MEM2.get(mem);
                mem.write(base.wrapping_add(to), id);
                to = to.wrapping_add(2);
            }
        }
        for k in n..0x14 {
            let base = crate::misc::bridge::ITEM_MEM2.get(mem);
            mem.write(base.wrapping_add(head).wrapping_add(2 * k), 0xffu16);
        }
        DEBUG_MONSTER_HP1.get(mem)
    }

    /// `MsSetRamChr(_, chr)` (`0x0079b4f0`, was `Mon_InitActor`; the first
    /// argument unused): sets up actor `chr` for a battle. `+0x540`
    /// (0x1ec bytes) cleared, then a party actor from its kernel and save
    /// records ([`ram_party`]: name, HP, MP, `MsSetRamChrParam`, the
    /// abilities, the gear list, "Escape" and "Steal" off under water or
    /// when nothing can be stolen) or a monster from its record
    /// ([`ram_monster`]). HP 1 under the debug toggles
    /// ([`DEBUG_PARTY_HP1`], [`DEBUG_MONSTER_HP1`]); auto-life, the weak
    /// level and the auto statuses; with no HP left (or already) the
    /// actor is down (`+0x606` bit 0, `+0xdcc` 2).
    pub fn ms_set_ram_chr(mem: &mut impl Mem, chr: u32) {
        let actor = ms_get_chr(mem, chr);
        crate::util::bridge::ms_struct_clear(mem, actor.wrapping_add(0x540), RAM_CHR_SIZE);
        let hp1 = if crate::actor::bridge::ms_get_ram_chr_monster(chr) == 0 {
            ram_party(mem, chr, actor)
        } else {
            ram_monster(mem, chr, actor)
        };
        if hp1 != 0 {
            put(mem, actor, 0x5d0, 1u32);
        }
        crate::actor::bridge::ms_set_eternal_relife(mem, actor);
        crate::input::bridge::ms_set_chr_weak(mem, chr, u32::MAX);
        crate::actor::bridge::ms_set_auto_status(mem, actor);
        let hp: i32 = mem.read(actor.wrapping_add(0x5d0));
        let status: u16 = mem.read(actor.wrapping_add(0x606));
        if hp < 1 || status & 1 != 0 {
            put(mem, actor, 0x606, status | 1);
            put(mem, actor, 0xdcc, 2u8);
        }
    }

    /// `MsSetRamMonData` (`0x0079cb10`): [`ms_set_ram_chr`] for each of the
    /// eight monster slots in use (`MsGetMon(i) + 0x10`).
    pub fn ms_set_ram_mon_data(mem: &mut impl Mem) {
        for i in 0..8 {
            let mon = crate::actor::bridge::ms_get_mon(mem, i);
            if byte(mem, mon, 0x10) != 0 {
                ms_set_ram_chr(mem, i + 0x14);
            }
        }
    }

    /// The records `MsSetRamChrData` gives defaults: the 0x1f actors and
    /// their 0x3e weapon records (`MsGetChr` 0..0x5d).
    pub const CHR_RECORDS: u32 = 0x5d;

    /// `MsSetRamChrData`'s defaults for every record (actors and weapons).
    fn record_defaults(mem: &mut impl Mem) {
        let water = crate::state::bridge::UNDERWATER.get(mem) != 0;
        for i in 0..CHR_RECORDS {
            let p = ms_get_chr(mem, i);
            let n = i.to_le_bytes()[0];
            put(mem, p, 0xc, n);
            put(mem, p, 0x13, 0xffu8);
            put(mem, p, 0x14, n);
            for o in [0x11, 0x24, 0x37] {
                put(mem, p, o, 1u8);
            }
            // under water the party floats (3)
            let mode: u8 = if water && i < 0x12 { 3 } else { 1 };
            put(mem, p, 0x20, mode);
            put(mem, p, 0x21, 0x101u16);
            put(mem, p, 0x25, u8::from(!water));
            put(mem, p, 0x26, u8::from(!water));
            put(mem, p, 0x27, 0xffffu16);
            put(mem, p, 0x2a, 0x100u16);
            put(mem, p, 0x2c, 3u8);
        }
    }

    /// `MsSetRamChrData`'s defaults for actor `i`, then its two weapon
    /// records' IDs (`0x5000 + i`, kind 0xe; `0x5100 + i`, kind 0xf).
    fn actor_defaults(mem: &mut impl Mem, i: u32) {
        let a = ms_get_chr(mem, i);
        // MsSetFields_0079a170
        put(mem, a, 0x776, 0xffu8);
        put(mem, a, 0xa4e, 0xffu8);
        put(mem, a, 0x40, 1.0f32);
        put(mem, a, 0x44, 1.0f32);
        put(mem, a, 0xdd2, 0x101u16);
        put(mem, a, 0xdd4, 1u8);
        put(mem, a, 0xdde, 0xffu16);
        put(mem, a, 0x15, 1u8);
        put(mem, a, 0x1b, 1u16);
        put(mem, a, 0x1d, 1u8);
        put(mem, a, 0x16, 0x0101_0101u32);
        put(mem, a, 0x2d, 1u8);
        put(mem, a, 0x2e, 0x0101_0101u32);
        put(mem, a, 0x23, 1u8);
        put(mem, a, 0x422, i.to_le_bytes()[0]);
        put(mem, a, 0xdf9, 1u8);
        put(mem, a, 0xde4, 0xffffu16);
        put(mem, a, 0x3f1, 0x10u8);
        put(mem, a, 0xe0e, 0x108u16);
        put(mem, a, 0xe2c, 1u32);
        put(mem, a, 0xe15, 1u8);
        crate::flow::bridge::ms_magic_effect_set(mem, i, a);
        put(mem, a, 0x43b, 1u8);
        put(mem, a, 0x3fc, 0x100u16);
        put(mem, a, 0x400, 0x100u16);
        put(mem, a, 0x406, 8u8);
        put(mem, a, 0x41d, 1u8);
        for o in [0x43f, 0x442, 0x3f2] {
            put(mem, a, o, 0xffu8);
        }
        put(mem, a, 0x44f, 0u8);
        put(mem, a, 0x408, 0u16);
        put(mem, a, 0x40f, 1u8);
        put(mem, a, 0x438, 0xffu8);
        put(mem, a, 0xf74, 0xffffu16);
        for o in [0x3a0, 0x3a4, 0x3a8] {
            put(mem, a, o, 1.0f32);
        }
        put(mem, a, 0x3f0, 0xffu8);
        put(mem, a, 0x3ac, 1.0f32);
        put(mem, a, 0x4b0, 0x0006_0006u32);
        put(mem, a, 0x4b8, 0x2004u16);
        put(mem, a, 0x4bc, 40.0f32);
        put(mem, a, 0x4ba, 0x20u8);
        put(mem, a, 0x32, 1u8);
        put(mem, a, 0x4c0, 0.0f32);
        put(mem, a, 0x50c, 1u8);
        put(mem, a, 0x4c4, 0.0f32);
        put(mem, a, 0x6d4, 0xffu8);
        put(mem, a, 0x4d4, 3.0f32);
        put(mem, a, 0x4d8, 3.0f32);
        put(mem, a, 0x4d0, 0.4f32);
        put(mem, a, 0x4dc, 50.0f32);
        put(mem, a, 0x4f0, 0.2f32);
        put(mem, a, 0x534, 0.0f32);
        let dry = crate::state::bridge::UNDERWATER.get(mem) == 0;
        put(mem, a, 0x34, u8::from(dry));
        let id = i & 0xffff;
        for (second, base, kind) in [(0, 0x5000, 0xeu8), (1, 0x5100, 0xf)] {
            let w = crate::actor::bridge::ms_get_weapon(mem, i, second);
            put(mem, w, 0xe, u16::try_from(id + base).unwrap_or_default());
            put(mem, w, 0xd, kind);
        }
    }

    /// `MsSetRamChrData`'s party: the battle party from the save's
    /// (`g_SaveInParty`, 3 slots; each member present (`+0x10`), able
    /// (`+0xdc8`), in its slot (`+0x4fe`), the middle one `+0x3f8`), slots
    /// 3-6 empty and the 17 reserve members copied; then the characters
    /// who joined (`MsGetSavePlyJoin`) or are present marked present.
    fn party_defaults(mem: &mut impl Mem) {
        let party = crate::ability::bridge::PARTY.ptr(mem).addr;
        let save_in = crate::save::bridge::IN_PARTY.ptr(mem).addr;
        for k in 0..3u8 {
            let c = byte(mem, save_in, k.into());
            if c != 0xff {
                let a = ms_get_chr(mem, c.into());
                put(mem, a, 0x10, 1u8);
                put(mem, a, 0xdc8, 1u8);
                put(mem, a, 0x4fe, k);
                if k == 1 {
                    put(mem, a, 0x3f8, 1u8);
                }
            }
            mem.write(party.wrapping_add(k.into()), c);
        }
        mem.write(party.wrapping_add(3), u32::MAX);
        let out: [u8; 17] = mem.read(crate::save::bridge::OUT_PARTY.ptr(mem).addr);
        crate::ability::bridge::PARTY_EXTRA.set(mem, out);
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            if crate::save::bridge::ms_get_save_ply_join(mem, i) == 1 || byte(mem, a, 0x10) != 0 {
                present(mem, a);
            }
        }
    }

    /// An actor in the battle: `+0x10`, `+0xdd6`-`+0xdda` set.
    fn present(mem: &mut impl Mem, a: u32) {
        put(mem, a, 0x10, 1u8);
        put(mem, a, 0xdd6, 0x101u16);
        put(mem, a, 0xdd9, 0x101u16);
        put(mem, a, 0xdd8, 1u8);
    }

    /// `MsSetRamChrData` (`0x0079c110`, was `Battle_SetUpActors`): every
    /// actor and weapon record to its defaults (`g_BtlNoCommand` cleared
    /// too), the party from the save, the 18 characters' actors
    /// ([`ms_set_ram_chr`], their weapons' models from the gear they
    /// wear), each actor's kind (party 3, aeon 4 in slot `i - 8`, the
    /// third aeon (0x10) `+0x3f8`; monster 5) and the formation's eight
    /// monsters (`g_BtlVoiceEvent + 0xc`, `u16` IDs, 0xffff an empty
    /// slot): each one met (`MsSetMonsterCheck`), numbered in order,
    /// present and able.
    pub fn ms_set_ram_chr_data<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::turn::bridge::ms_get_chr_id;
        crate::util::bridge::ms_struct_clear(
            mem,
            crate::stat::bridge::NO_COMMAND.ptr(mem).addr,
            0x60,
        );
        record_defaults(mem);
        for i in 0..crate::actor::ACTORS {
            actor_defaults(mem, i);
        }
        party_defaults(mem);
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            put(mem, a, 0xdd5, 1u8);
            put(mem, a, 0xe, u16::try_from(i).unwrap_or_default());
            ms_get_chr_id(mem, &mut *call, a);
            ms_set_ram_chr(mem, i);
            for k in 0..2u32 {
                let w = crate::actor::bridge::ms_get_weapon(mem, i, k.cast_signed());
                let id = byte(mem, a, 0x592 + k);
                let model = crate::exchange::bridge::ms_get_save_weapon_model(mem, id.into());
                put(mem, w, 4, model);
            }
        }
        for i in 0..crate::actor::ACTORS {
            let a = ms_get_chr(mem, i);
            if crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 {
                put(mem, a, 0x3f5, 0x303u16);
                put(mem, a, 0x43d, 3u8);
                put(mem, a, 0x4fd, 5u8);
                continue;
            }
            put(mem, a, 0x3f5, 0u16);
            let summon = crate::damage::bridge::ms_get_ram_chr_summon(mem, i) != 0;
            put(mem, a, 0x43d, 8u8);
            if summon {
                put(mem, a, 0x4fd, 4u8);
                put(mem, a, 0x4fe, i.wrapping_sub(8).to_le_bytes()[0]);
                put(mem, a, 0x1e, 1u8);
                put(mem, a, 0x3f9, 1u8);
                put(mem, a, 0xe15, 0u8);
                if i == 0x10 {
                    put(mem, a, 0x3f8, 1u8);
                }
            } else {
                put(mem, a, 0x4fd, 3u8);
                put(mem, a, 0x437, 1u8);
            }
        }
        let mut n = 0u8;
        for i in 0..8 {
            let m = crate::actor::bridge::ms_get_mon(mem, i);
            let formation = crate::state::bridge::VOICE_EVENT.get(mem);
            let id: u16 = mem.read(formation.wrapping_add(0xc + 2 * i));
            put(mem, m, 0xe, id);
            ms_get_chr_id(mem, &mut *call, m);
            let id: u16 = mem.read(m.wrapping_add(0xe));
            if id == 0xffff {
                continue;
            }
            let meet = crate::save::bridge::ms_get_save_monster_meet(mem);
            crate::save::bridge::ms_set_monster_check(mem, id.into(), meet);
            put(mem, m, 0x4fe, n);
            n = n.wrapping_add(1);
            put(mem, m, 0xdf3, 1u8);
            put(mem, m, 0x10, 1u8);
            put(mem, m, 0x39, 1u8);
            present(mem, m);
            put(mem, m, 0xdc8, 1u8);
            put(mem, m, 0x447, 3u8);
        }
    }
}
