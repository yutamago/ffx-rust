//! A monster's rewards when it dies (`docs/rewrite-plan.md`, M2):
//! `MsGetItem` (`0x007990d0`, from `MsDamageCheckDeath`) hands out its AP,
//! gil, item drops (or what a bribe bought) and a chance of gear
//! (`BattleRewards_AddGear`, `0x00798c10`), into the battle's spoils
//! (`g_BtlCountsBattle`, `0x02310ea0`). `docs/formats/battle.md`, "Rewards".
//!
//! The monster's reward record (its actor's `+0xf88`): `u16` gil `+0`, AP
//! `+2` (`+4` with overkill), drop chances `+8`, `+9`, gear chance `+0xb`,
//! drops `+0xc` (normal, rare; second slot normal, rare; `+0x18` with
//! overkill), their counts `+0x14` (`+0x20`), the gear's slot byte `+0x2d`,
//! its owner bytes `+0x2e`-`+0x30`, ability count byte `+0x31` and from
//! `+0x32` per character a weapon's then armour's eight abilities (`u16`,
//! the first always given), the bribe's item `+0x2a` and the bribe's
//! factor byte `+0x2c`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::exec::bridge::Call;
    use crate::rng::bridge::brnd;
    use crate::turn::bridge::{battle_rewards_add_item, ms_ap_up};

    /// Debug toggles (`u8`): every chance taken (with one above 0), rare
    /// drops always, AP ×100, gil ×100.
    pub const DEBUG_ALWAYS_HIT: Global<u8> = Global::new(0x0112_a90a);
    pub const DEBUG_RARE_STEAL: Global<u8> = Global::new(0x0112_a911);
    pub const DEBUG_AP_X100: Global<u8> = Global::new(0x0112_a912);
    pub const DEBUG_GIL_X100: Global<u8> = Global::new(0x0112_a913);
    /// The gil won this battle (`u32`, at most 999999999).
    pub const GIL_WON: Global<u32> = Global::new(0x0231_0f6c);
    /// The battle's spoils (`g_BtlCountsBattle`, an image address): `i8`
    /// gear count at
    /// `+0xd2`, eight gear records of [`GEAR_RECORD`] bytes from
    /// [`GEAR_RECORDS`].
    pub const SPOILS: u32 = 0x0231_0ea0;
    pub const GEAR_RECORDS: u32 = 0xfe;
    pub const GEAR_RECORD: u32 = 0x16;
    /// `TOGetFFXLangJmp()` (system), whose language 0xb trades item 0x2065
    /// for another; `MsWeaponName(name, owner, 0, &icon)` (battle, ported).
    pub const TO_GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// `brnd(index)` as the original's signed division sees it.
    fn roll(mem: &mut impl Mem, index: u32) -> i32 {
        brnd(mem, index).cast_signed()
    }

    /// `BattleRewards_NewGear_00798bd0(spoils)`: the next of the spoils'
    /// eight gear records (`i8` count at `+0xd2`), cleared and counted;
    /// -1 with eight taken.
    pub fn new_gear(mem: &mut impl Mem, spoils: u32) -> i32 {
        let n: i8 = mem.read(spoils.wrapping_add(0xd2));
        if n >= 8 {
            return -1;
        }
        let i = i32::from(n);
        let at = spoils
            .wrapping_add(GEAR_RECORDS)
            .wrapping_add(i.cast_unsigned().wrapping_mul(GEAR_RECORD));
        crate::util::bridge::ms_struct_clear(mem, at, GEAR_RECORD.cast_signed());
        mem.write(spoils.wrapping_add(0xd2), n.wrapping_add(1));
        i
    }

    /// `MsGetAbilityGroup_00798a90(ability)`: an auto-ability's group
    /// (`g_KernelAAbility` record byte `+0x69`; gear holds one of a
    /// group), -1 for none (0).
    pub fn ability_group(mem: &impl Mem, ability: u32) -> u32 {
        if ability == 0 {
            return u32::MAX;
        }
        let table = ffx_kernel::bridge::A_ABILITY.get(mem);
        #[allow(clippy::cast_possible_wrap)] // 12 bits
        let (r, _) = ffx_kernel::bridge::excel_data(mem, (ability & 0xfff) as i32, table);
        mem.read::<u8>(r.wrapping_add(0x69)).into()
    }

    /// Who gets a piece of gear: one of the party members who joined (of
    /// the seven), drawn among them and, when the killer `chr` is one of
    /// the seven, three more chances for the killer; else character 0
    /// for those.
    fn gear_owner(mem: &mut impl Mem, chr: i32) -> u8 {
        let members = (0..7)
            .filter(|&i| crate::save::bridge::ms_get_save_ply_joined(mem, i) != 0)
            .count();
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // at most 7
        let mut chances = members as i32;
        let killer = if chr < 7 {
            chances += 3;
            chr
        } else {
            0
        };
        // With no member and no killer the original divides by zero.
        let pick = roll(mem, 0xc).checked_rem(chances).unwrap_or(0);
        let mut seen = 0;
        for i in 0..7u8 {
            if crate::save::bridge::ms_get_save_ply_joined(mem, i.into()) != 0 {
                seen += 1;
                if seen > pick {
                    return i;
                }
            }
        }
        killer.to_le_bytes()[0]
    }

    /// `BattleRewards_AddGear(chr, rewards, spoils)` (`0x00798c10`): a
    /// new piece of gear in the spoils ([`new_gear`]; -1 with none left):
    /// count 1 (`+2`), an owner ([`gear_owner`], `+4`), weapon or armour at
    /// random (`+5`), `+6` 0xff, the monster's bytes `+0x2e`, `+0x30`,
    /// `+0x2f` (`+8`-`+0xa`), slots `(slot byte + 1..8 − 5) / 4` within
    /// 1-4 (`+0xb`); its abilities from the owner's list for that kind:
    /// the first (when not 0), then `(ability byte + 1..8 − 5) / 8` draws
    /// among the other seven, each taken when not 0 and of a group the
    /// gear lacks, until the slots are full; empty ones 0xff. Then its
    /// name (`MsWeaponNameNum`, `+0`) and the name's icon (`MsWeaponName`,
    /// `+0xc`). 0.
    pub fn battle_rewards_add_gear<M: Mem>(
        mem: &mut M,
        chr: u32,
        rewards: u32,
        spoils: u32,
    ) -> u32 {
        let i = new_gear(mem, spoils);
        if i < 0 {
            return u32::MAX;
        }
        let gear = spoils
            .wrapping_add(GEAR_RECORDS)
            .wrapping_add(i.cast_unsigned().wrapping_mul(GEAR_RECORD));
        let byte = |mem: &M, o: u32| -> u8 { mem.read(rewards.wrapping_add(o)) };
        mem.write(gear.wrapping_add(2), 1u16);
        let owner = gear_owner(mem, chr.cast_signed());
        mem.write(gear.wrapping_add(4), owner);
        let kind = brnd(mem, 0xc).to_le_bytes()[0] & 1;
        mem.write(gear.wrapping_add(5), kind);
        mem.write(gear.wrapping_add(6), 0xffu8);
        for (to, from) in [(8, 0x2e), (9, 0x30), (0xa, 0x2f)] {
            let v = byte(mem, from);
            mem.write(gear.wrapping_add(to), v);
        }
        let slot_roll = (brnd(mem, 0xc) & 7).cast_signed() + 1;
        let ability_roll = (brnd(mem, 0xc) & 7).cast_signed() + 1;
        let slots = ffx_kernel::bridge::ms_check_range(
            (i32::from(byte(mem, 0x2d)) - 5 + slot_roll) / 4,
            1,
            4,
        );
        let slots = slots.to_le_bytes()[0];
        mem.write(gear.wrapping_add(0xb), slots);
        let mut draws = (i32::from(byte(mem, 0x31)) - 5 + ability_roll) / 8;
        let owner: u8 = mem.read(gear.wrapping_add(4));
        let kind: u8 = mem.read(gear.wrapping_add(5));
        let list = rewards.wrapping_add(0x32 + (u32::from(kind) + 2 * u32::from(owner)) * 0x10);
        let ability = |mem: &M, k: u32| -> u16 { mem.read(list.wrapping_add(2 * k)) };
        let slot = |k: u32| gear.wrapping_add(0xe + 2 * k);
        let mut n = 0u32;
        let first = ability(mem, 0);
        if slots != 0 && first != 0 {
            mem.write(slot(0), first);
            n = 1;
        }
        while draws > 0 {
            if n >= u32::from(mem.read::<u8>(gear.wrapping_add(0xb))) {
                break;
            }
            let k = roll(mem, 0xd) % 7;
            let candidate = ability(mem, k.cast_unsigned() + 1);
            if candidate != 0 {
                let group = ability_group(mem, candidate.into());
                let taken = (0..n).any(|j| {
                    let held: u16 = mem.read(slot(j));
                    ability_group(mem, held.into()) == group
                });
                if !taken {
                    mem.write(slot(n), candidate);
                    n += 1;
                }
            }
            draws -= 1;
        }
        for k in n..4 {
            mem.write(slot(k), 0xffu16);
        }
        let name = crate::gear::bridge::ms_weapon_name_num(mem, gear).to_le_bytes();
        let name = u16::from_le_bytes([name[0], name[1]]);
        mem.write(gear, name);
        let owner: u8 = mem.read(gear.wrapping_add(4));
        crate::menu::bridge::ms_weapon_name(
            mem,
            name.into(),
            owner.into(),
            0,
            gear.wrapping_add(0xc),
        );
        0
    }

    /// The items a bribe buys: `(brnd % 11 + 20) · sqrt(gil paid) · factor
    /// / 16 / 25`, plus 0 or 1, within 1-99. The x87 at 53 bits: the
    /// square root and the sum are stored as `f32`, the product kept as a
    /// double; truncated by `_ftol2_sse`.
    fn bribe_count(mem: &mut impl Mem, chr: u32, actor: u32, rewards: u32) -> i32 {
        use crate::damage::bridge::ms_get_rnd_chr;
        use ffx_bridge::x87;
        let g = ms_get_rnd_chr(mem, chr, 0);
        let scale = brnd(mem, g).cast_signed() % 11 + 20;
        let paid: i32 = mem.read(actor.wrapping_add(0x708));
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let root = x87::sqrt(f64::from(paid)) as f32;
        let factor: u8 = mem.read(rewards.wrapping_add(0x2c));
        let product = x87::div(
            x87::mul(
                x87::mul(x87::mul(f64::from(root), f64::from(factor)), 0.0625),
                f64::from(scale),
            ),
            25.0,
        );
        let g = ms_get_rnd_chr(mem, chr, 0);
        let extra = brnd(mem, g) & 1;
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // fstp dword
        let sum = x87::add(f64::from(extra as f32), product) as f32;
        let n = x87::ftol(mem, f64::from(sum)).cast_signed();
        ffx_kernel::bridge::ms_check_range(n, 1, 99)
    }

    /// One drop roll for slot `k` (0, 1): its chance (`+8 + k`, out of
    /// 255; any above 0 with [`DEBUG_ALWAYS_HIT`]), then rare (1 in 8 or
    /// [`DEBUG_RARE_STEAL`]) or normal; the item `+drops + 2k + rare` (the
    /// killing hit's `flags` 2, 4, 8 replace it with 0x2046-0x2048, with
    /// 0x10 clear 0x20 with 0x2049) and its count.
    fn drop_roll<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        rewards: u32,
        overkill: bool,
        flags: u32,
        k: u32,
    ) {
        let (items, counts) = if overkill { (0x18, 0x20) } else { (0xc, 0x14) };
        let chance: u8 = mem.read(rewards.wrapping_add(8 + k));
        let hit = roll(mem, 10) % 0xff < i32::from(chance)
            || (DEBUG_ALWAYS_HIT.get(mem) != 0 && chance != 0);
        if !hit {
            return;
        }
        let rare = u32::from(brnd(mem, 0xb) & 0xff < 0x20 || DEBUG_RARE_STEAL.get(mem) != 0);
        let i = 2 * k + rare;
        let mut item: u16 = mem.read(rewards.wrapping_add(items + 2 * i));
        let mut count: u8 = mem.read(rewards.wrapping_add(counts + i));
        if flags & 2 != 0 {
            item = 0x2046;
        } else if flags & 4 != 0 {
            item = 0x2047;
        } else if flags & 8 != 0 {
            item = 0x2048;
        } else if flags & 0x10 == 0 && flags & 0x20 != 0 {
            item = 0x2049;
        }
        if game(mem, call, TO_GET_FFX_LANG_JMP, &[]) == 0xb && item == 0x2065 {
            item = 0x2032;
            count = if overkill { 8 } else { 4 };
        }
        let spoils = mem.rebase(SPOILS);
        battle_rewards_add_item(mem, item.into(), count.into(), spoils);
    }

    /// `MsGetItem(chr, actor, rewards, overkill, flags)` (`0x007990d0`):
    /// the dead monster `actor`'s rewards (record `rewards`, none when
    /// null) for the killer `chr`: its AP (`+2`, `+4` with `overkill`;
    /// ×100 with [`DEBUG_AP_X100`]) to the 18 characters (`MsApUp`, each
    /// handed the last one's result, 1 first). A bribed monster (`+0x701`
    /// 2) gives its bribe item (`+0x2a`, [`bribe_count`] of them; with
    /// language 0xb item 0x2065 becomes 0x2048, written back, and 4).
    /// Else its gil (`+0`, ×100 with [`DEBUG_GIL_X100`]) times the last
    /// `MsApUp` result to [`GIL_WON`] and two drop rolls ([`drop_roll`]).
    /// Then the gear's chance (`+0xb` of 255,
    /// [`battle_rewards_add_gear`]).
    pub fn ms_get_item<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        chr: u32,
        actor: u32,
        rewards: u32,
        overkill: u32,
        flags: u32,
    ) {
        if rewards == 0 {
            return;
        }
        let overkill = overkill != 0;
        let mut ap = u32::from(mem.read::<u16>(rewards.wrapping_add(if overkill { 4 } else { 2 })));
        if DEBUG_AP_X100.get(mem) != 0 {
            ap = ap.wrapping_mul(100);
        }
        let mut gil = u32::from(mem.read::<u16>(rewards));
        if DEBUG_GIL_X100.get(mem) != 0 {
            gil = gil.wrapping_mul(100);
        }
        let mut last = 1;
        for i in 0..0x12 {
            let a = crate::actor::bridge::ms_get_chr(mem, i);
            last = ms_ap_up(mem, i, a, ap, last);
        }
        if mem.read::<u8>(actor.wrapping_add(0x701)) == 2 {
            let mut n = bribe_count(mem, chr, actor, rewards);
            let item_at = rewards.wrapping_add(0x2a);
            if game(mem, &mut *call, TO_GET_FFX_LANG_JMP, &[]) == 0xb
                && mem.read::<u16>(item_at) == 0x2065
            {
                mem.write(item_at, 0x2048u16);
                n = 4;
            }
            let item: u16 = mem.read(item_at);
            let spoils = mem.rebase(SPOILS);
            battle_rewards_add_item(mem, item.into(), n, spoils);
        } else {
            let won = last.wrapping_mul(gil).wrapping_add(GIL_WON.get(mem));
            let won = ffx_kernel::bridge::ms_check_range(won.cast_signed(), 0, 999_999_999);
            GIL_WON.set(mem, won.cast_unsigned());
            for k in 0..2 {
                drop_roll(mem, &mut *call, rewards, overkill, flags, k);
            }
        }
        let chance: u8 = mem.read(rewards.wrapping_add(0xb));
        if roll(mem, 10) % 0xff < i32::from(chance)
            || (DEBUG_ALWAYS_HIT.get(mem) != 0 && chance != 0)
        {
            let spoils = mem.rebase(SPOILS);
            battle_rewards_add_gear(mem, chr, rewards, spoils);
        }
    }
}
