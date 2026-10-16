//! Damage, hits and statuses (`docs/rewrite-plan.md`, M2): the
//! `DmgCalc_*` steps `DmgCalc` (`0x0078e630`) runs for one target of one
//! command, the damage formulas (`MsCalcDamageCommand`) and the escape
//! roll. Names are Fahrenheit's (`fh` lines in `symbols/ffx.sym`).
//!
//! An actor's fields the damage code reads (actor layout: [`crate::actor`]):
//!
//! | Offset | Type | Field |
//! |---|---|---|
//! | `+0x0c` | `u8` | its index (`MsGetChr`), picks the random generator ([`rnd_chr`]) |
//! | `+0x590` | `u8` | bit 2: a summon (`MsGetRamChrSummon`) |
//! | `+0x594`, `+0x598` | `u32` | max HP, max MP |
//! | `+0x5a8`-`+0x5af` | `u8` | strength, defense, magic, magic defense, agility, luck, evasion, accuracy |
//! | `+0x5b8` | `u16` | monster traits: 1 armored, 2 no gravity, 0x20/0x40/0x80 no physical/magic/any damage, 0x400 no bribe |
//! | `+0x5b9` | `u8` | bit 0: no delay |
//! | `+0x5ca` | `u8` | a magic booster charge (`DmgCalc_MagicBooster`) |
//! | `+0x5d0`, `+0x5d4` | `u32` | HP, MP |
//! | `+0x5d8` | `u8` | the weapon's critical bonus |
//! | `+0x5da`-`+0x5dd` | `u8` × 4 | elements absorbed, immune, halved, weak |
//! | `+0x606`, `+0x616` | `u16` | status bits (`0x10` power break, `0x20` magic break, `0x200` berserk; `0x40` shield, `0x80` boost) |
//! | `+0x640` | `u8` | bit 3: damage over 9999 kept, bit 4: always critical |
//! | `+0x65d` | `u8` | CTB |
//! | `+0x65e`-`+0x663` | `u8` | cheer, aim, focus, reflex, luck and jinx levels |
//! | `+0x6bc` | `u16` | auto-abilities: 8/0x10 counter, 0x20 magic counter, 0x40 magic booster, 0x200 alchemy, 0x2000 piercing |
//! | `+0x6da`, `+0x6dc` | `u8` | defended, counters |
//! | `+0x6e4`-`+0x6ec` | `u32` × 3 | HP, MP, CTB after this action's damage so far |
//! | `+0xd26`, `+0xd2c`, `+0xd30` | `u8`, `f32`, `f32` | Wakka's reels: set, time, start time (`MsSetReelResult`) |
//!
//! A command (`Command`, from the kernel tables through `MsGetCommand`):
//!
//! | Offset | Type | Field |
//! |---|---|---|
//! | `+0x1c` | `u32` | flags: `0x38` hit formula, `0x40` darkness, `0x100` drain, `0x2000`/`0x4000` delay, `0x40000` weapon's stats, `0x800000` only the living, `0x80000000` bribe |
//! | `+0x1e`, `+0x1a` | `u8` | bit 0: pierces armor; `0xc`: no counter |
//! | `+0x20` | `u16` | `& 3` physical (1) or magical (2); 4 can be critical, 8 with the weapon's bonus, 0x10 heals, 0x20 removes statuses, 0x40 caps at 9999, 0x80 at 99999 |
//! | `+0x23` | `u8` | what it hits: 1 HP, 2 MP, 4 CTB |
//! | `+0x27`, `+0x28`, `+0x29`, `+0x2a` | `u8` | critical bonus, damage formula, accuracy, power |
//! | `+0x2d` | `u8` | elements |
//!
//! The per-target result (`DamageInfo`, 0x2c bytes) carries the target's
//! statuses as they will be: `+7`-`+0x13` the 13 counters, `+0x14` and
//! `+0x16` the status bits (from actor `+0x606`, `+0x608`, `+0x616` by
//! `SetUpDamageInfo`), the damage at `+0x20` (HP, MP, CTB).

/// Elements: fire, ice, thunder, water, holy and three more, one bit each.
pub const ELEMENTS: u32 = 8;

/// `MsGetRndChr(chr, kind)`: the random generator (`brnd`) for actor
/// `chr`'s rolls of a kind (0 damage, 1 hits, 2 statuses), 0x10 apart:
/// monsters (`monster`) from 0x1c, a summon (`summon`) 0x1b, the rest from
/// 0x14.
#[must_use]
pub fn rnd_chr(chr: u32, kind: i32, monster: bool, summon: impl FnOnce() -> bool) -> u32 {
    let off = match kind {
        1 => 0x10,
        2 => 0x20,
        _ => 0,
    };
    if monster {
        chr.wrapping_add(8).wrapping_add(off)
    } else if summon() {
        0x1b + off
    } else {
        chr.wrapping_add(0x14).wrapping_add(off)
    }
}

/// What a damage formula reads of the user and the target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FormulaStats {
    /// Strength, magic (`+0x5a8`, `+0x5aa`).
    pub strength: u8,
    pub magic: u8,
    /// Cheer and focus levels (`+0x65e`, `+0x660`).
    pub cheer: u8,
    pub focus: u8,
    /// Max HP and MP (`+0x594`, `+0x598`).
    pub max_hp: u32,
    pub max_mp: u32,
    /// HP and MP (`+0x5d0`, `+0x5d4`).
    pub hp: u32,
    pub mp: u32,
    /// CTB (`+0x65d`).
    pub ctb: u8,
    /// HP, MP and CTB after the damage so far (`+0x6e4`-`+0x6ec`).
    pub pending: [u32; 3],
}

/// The inputs of `MsCalcDamageCommand` besides the two actors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Formula {
    /// The damage formula (command `+0x28`), 1-0x17.
    pub formula: i32,
    /// The command's power (`+0x2a`).
    pub power: i32,
    /// The target's defense and magic defense, as used (zeroed by the
    /// caller's flags for armor-piercing commands).
    pub defense: i32,
    pub magic_defense: i32,
    /// Which of the target's values formulas 5 and 8 use: 1 HP, 2 MP, 4 CTB.
    pub stat: i32,
    /// 240-271 (the roll), or 256 without variance.
    pub variance: i32,
    /// The result for formulas that compute none.
    pub fallback: i32,
    /// Gil for formula 0x15 (`0x0112be90`).
    pub gil: i32,
    /// For formula 0x16: the target's kill count, when it has one.
    pub kills: Option<i32>,
}

/// `(a + b)³ / 32 + 30`, the strength or magic base.
const fn cube(stat: i32) -> i32 {
    stat.wrapping_mul(stat).wrapping_mul(stat) / 32 + 30
}

/// `730 - (51·def - def²/11) / 10`, the defense factor.
const fn defense_factor(def: i32) -> i32 {
    0x2da - (def.wrapping_mul(0x33) - def.wrapping_mul(def) / 11) / 10
}

/// What a damage formula gives before healing turns it negative; `true`
/// with it for the formulas that return at once (0xe, 0x14: no out values,
/// no healing sign).
#[must_use]
#[allow(clippy::similar_names)]
pub fn formula_damage(user: &FormulaStats, target: &FormulaStats, f: &Formula) -> (i32, bool) {
    let p = f.power;
    let var = f.variance;
    let after_var = |x: i32| x.wrapping_mul(var) / 256;
    let str_c = i32::from(user.strength) + i32::from(user.cheer);
    let mag_f = i32::from(user.magic) + i32::from(user.focus);
    // formulas 5 and 8
    let (current, max) = match f.stat {
        1 => (target.pending[0], target.max_hp),
        2 => (target.pending[1], target.max_mp),
        4 => (target.pending[2], u32::from(target.ctb)),
        _ => (0, 0),
    };
    let (current, max) = (current.cast_signed(), max.cast_signed());
    let celestial = |num: u32, den: u32, add: bool| {
        let q = num.wrapping_mul(100).checked_div(den).unwrap_or(0);
        let k = if add {
            q.wrapping_add(10)
        } else {
            0x82u32.wrapping_sub(q)
        };
        let k = k.wrapping_mul(cube(str_c).cast_unsigned());
        let k = if add { k / 0x6e } else { k / 0x3c };
        (k.wrapping_mul(p.cast_unsigned()) >> 4).wrapping_mul(var.cast_unsigned()) >> 8
    };
    let d = match f.formula {
        1 => {
            let x = defense_factor(f.defense).wrapping_mul(cube(str_c)) / 0x2da;
            let x = x.wrapping_mul(0xf - i32::from(target.cheer)) / 0xf;
            after_var(x.wrapping_mul(p) / 16)
        }
        2 => after_var(cube(str_c).wrapping_mul(p) / 16),
        3 => {
            let x = (mag_f * mag_f / 6).wrapping_add(p).wrapping_mul(p) / 4;
            let x = defense_factor(f.magic_defense).wrapping_mul(x) / 0x2da;
            after_var(x.wrapping_mul(0xf - i32::from(target.focus)) / 0xf)
        }
        4 => after_var((mag_f * mag_f / 6).wrapping_add(p).wrapping_mul(p) / 4),
        5 => current.wrapping_mul(p) / 16,
        6 => p.wrapping_mul(50),
        7 => {
            (mag_f.wrapping_add(p) / 2)
                .wrapping_mul(var)
                .wrapping_mul(p)
                / 256
        }
        8 => max.wrapping_mul(p) / 16,
        9 => var.wrapping_mul(p).wrapping_mul(50) / 256,
        10 => (target.max_mp.wrapping_mul(p.cast_unsigned()) >> 4).cast_signed(),
        11 => i32::from(target.ctb).wrapping_mul(p) / 16,
        12 => target.pending[1].cast_signed().wrapping_mul(p) / 16,
        13 => target.pending[2].cast_signed().wrapping_mul(p) / 16,
        0xe => return (cube(i32::from(user.strength)).wrapping_mul(p) / 16, true),
        0xf => after_var(cube(mag_f).wrapping_mul(p) / 16),
        0x10 => (user.max_hp.wrapping_mul(p.cast_unsigned()) / 10).cast_signed(),
        0x11 => celestial(user.hp, user.max_hp, true).cast_signed(),
        0x12 => celestial(user.mp, user.max_mp, true).cast_signed(),
        0x13 => celestial(user.hp, user.max_hp, false).cast_signed(),
        0x14 => return (cube(i32::from(user.magic)).wrapping_mul(p) / 16, true),
        0x15 => f.gil / 10,
        0x16 => f.kills.map_or(f.fallback, |k| k.wrapping_mul(p)),
        0x17 => p.wrapping_mul(9999),
        _ => f.fallback,
    };
    (d, false)
}

/// `DmgCalc_Elem(target, command, elements, damage)` given the target's
/// elements absorbed, immune to, halved and weak to (`+0x5da`-`+0x5dd`):
/// ×1.5 for each weakness hit (no further checks after one); else unchanged
/// when one element is neutral; else halved when one is halved; else 0 when
/// one is immune, else negated (absorbed) when one is absorbed.
#[must_use]
pub fn element_damage(
    elements: u32,
    absorb: u8,
    immune: u8,
    half: u8,
    weak: u8,
    damage: i32,
) -> i32 {
    if elements == 0 {
        return damage;
    }
    let bits = (0..ELEMENTS)
        .map(|i| 1u8 << i)
        .filter(|&b| u32::from(b) & elements != 0);
    let mut d = damage;
    let mut weak_hit = false;
    for b in bits.clone() {
        if weak & b != 0 {
            d = d.wrapping_mul(3) / 2;
            weak_hit = true;
        }
    }
    if weak_hit {
        return d;
    }
    if bits.clone().any(|b| (immune | half | absorb) & b == 0) {
        return d;
    }
    if bits
        .clone()
        .any(|b| half & b != 0 && (immune | absorb) & b == 0)
    {
        return d / 2;
    }
    if bits.clone().any(|b| immune & b != 0 && absorb & b == 0) {
        return 0;
    }
    if bits.clone().any(|b| absorb & b != 0) {
        return d.wrapping_neg();
    }
    d
}

/// What `DmgCalc_CheckHit` reads of the user and the target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HitStats {
    /// Luck, evasion, accuracy (`+0x5ad`-`+0x5af`).
    pub luck: u8,
    pub evasion: u8,
    pub accuracy: u8,
    /// Aim, reflex, luck and jinx levels (`+0x65f`, `+0x661`-`+0x663`).
    pub aim: u8,
    pub reflex: u8,
    pub luck_level: u8,
    pub jinx: u8,
    /// Darkness (status counter `+0x60a`).
    pub dark: u8,
}

/// The hit chance (out of 101) of hit formula `kind` (command `+0x1c`
/// bits 3-5, 1-7) with command accuracy `accuracy`; `table` is the game's
/// nine-entry accuracy table (`0x00c421f0`, signed); `darkness` when the
/// command is affected by it.
#[must_use]
pub fn hit_chance(
    kind: u32,
    accuracy: u8,
    user: &HitStats,
    target: &HitStats,
    darkness: bool,
    table: &[i8; 9],
) -> i32 {
    let acc = i32::from(user.accuracy);
    let base = match kind {
        1 | 2 => i32::from(accuracy),
        3 | 4 => acc,
        5 => acc * 5 / 2,
        6 => acc * 3 / 2,
        7 => acc >> 1,
        _ => 0,
    };
    let dark = darkness && user.dark != 0;
    let mut chance = if matches!(kind, 2 | 4) {
        base - i32::from(target.evasion)
    } else {
        let i = (base * 2 / 5 - i32::from(target.evasion) + 10).clamp(0, 8);
        i32::from(table[i.cast_unsigned() as usize])
    };
    if dark {
        chance /= 10;
    }
    let levels = (i32::from(user.aim) - i32::from(target.reflex)) * 10 - i32::from(target.luck)
        + i32::from(user.luck_level);
    i32::from(user.luck) + i32::from(target.jinx) + chance + levels
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::frame::Frame;
    use ffx_bridge::{Global, Mem, Ptr};

    use super::{
        Formula, FormulaStats, HitStats, element_damage, formula_damage, hit_chance, rnd_chr,
    };
    use crate::actor::bridge::{ms_get_chr, ms_get_ram_chr_monster};
    use crate::rng::bridge::brnd;

    /// Debug: no damage variance (`DmgCalc` passes `1 - it` as `vary`).
    pub const DEBUG_NO_VARIANCE: Global<i8> = Global::new(0x0112_a908);
    /// Debug: no critical hits (`DmgCalc` skips `DmgCalc_Crit`).
    pub const DEBUG_NO_CRIT: Global<u8> = Global::new(0x0112_a909);
    /// Debug: everything hits (`DmgCalc_CheckHit`, statuses, bribes).
    pub const DEBUG_ALWAYS_HIT: Global<u8> = Global::new(0x0112_a90a);
    /// Debug: no MP or other costs (`MsCalcMagic`).
    pub const DEBUG_NO_COST: Global<u8> = Global::new(0x0112_a90c);
    /// Debug: every hit is critical (`DmgCalc_Crit`).
    pub const DEBUG_ALWAYS_CRIT: Global<u8> = Global::new(0x0112_a90d);
    /// Debug environment flag: everything misses, no status lands, no escape.
    pub const DEBUG_ALL_MISS: Global<u8> = Global::new(0x0112_a91f);
    /// Gil offered (Bribe) or thrown (formula 0x15).
    pub const GIL_AMOUNT: Global<i32> = Global::new(0x0112_be90);
    /// Escape rule set by battle script: 1 never, 2 always.
    pub const ESCAPE_RULE: Global<i8> = Global::new(0x0112_c9ff);
    /// Counterattacks allowed (set by battle script).
    pub const COUNTER_ON: Global<u8> = Global::new(0x0112_ca12);
    /// Per actor, four bytes: physical and magic damage dealt bonus (%),
    /// physical and magic damage taken reduction (%).
    pub const DAMAGE_RATES: Global<[u8; 4]> = Global::new(0x0231_1240);
    /// The accuracy table of the hit formulas, 9 × `i8`.
    pub const ACCURACY_TABLE: Global<[i8; 9]> = Global::new(0x00c4_21f0);
    /// Kill counts, `u32` every 0x94 bytes by player character (formula 0x16).
    pub const KILLS: Global<u32> = Global::new(0x0113_20b0);
    /// Bytes per [`KILLS`] entry.
    pub const KILLS_STRIDE: u32 = 0x94;

    fn u8_at(mem: &impl Mem, p: u32, o: u32) -> u8 {
        Ptr::<u8>::new(p).field::<u8>(o).get(mem)
    }

    fn u16_at(mem: &impl Mem, p: u32, o: u32) -> u16 {
        Ptr::<u8>::new(p).field::<u16>(o).get(mem)
    }

    fn u32_at(mem: &impl Mem, p: u32, o: u32) -> u32 {
        Ptr::<u8>::new(p).field::<u32>(o).get(mem)
    }

    fn add_u32(mem: &mut impl Mem, p: u32, o: u32, n: u32) {
        let v = u32_at(mem, p, o).wrapping_add(n);
        mem.write(p.wrapping_add(o), v);
    }

    fn or_u32(mem: &mut impl Mem, p: u32, bits: u32) {
        let v = mem.read::<u32>(p) | bits;
        mem.write(p, v);
    }

    /// `& 3` of a command's `+0x20`: 1 physical, 2 magical.
    fn class(mem: &impl Mem, command: u32) -> u16 {
        u16_at(mem, command, 0x20) & 3
    }

    /// `MsGetRamChrSummon(chr)` (`0x0079af60`): bit 2 of `+0x590`.
    pub fn ms_get_ram_chr_summon(mem: &impl Mem, chr: u32) -> u32 {
        u32::from(u8_at(mem, ms_get_chr(mem, chr), 0x590) >> 2) & 1
    }

    /// `MsGetRndChr(chr, kind)` (`0x0078d210`): [`rnd_chr`].
    pub fn ms_get_rnd_chr(mem: &impl Mem, chr: u32, kind: i32) -> u32 {
        rnd_chr(chr, kind, ms_get_ram_chr_monster(chr) != 0, || {
            ms_get_ram_chr_summon(mem, chr) != 0
        })
    }

    /// One roll of actor `chr`'s generator of `kind`.
    fn roll(mem: &mut impl Mem, chr: u32, kind: i32) -> u32 {
        let g = ms_get_rnd_chr(mem, chr, kind);
        brnd(mem, g)
    }

    /// The actor's own index (`+0xc`).
    fn index(mem: &impl Mem, actor: u32) -> u32 {
        u32::from(u8_at(mem, actor, 0xc))
    }

    fn formula_stats(mem: &impl Mem, a: u32) -> FormulaStats {
        FormulaStats {
            strength: u8_at(mem, a, 0x5a8),
            magic: u8_at(mem, a, 0x5aa),
            cheer: u8_at(mem, a, 0x65e),
            focus: u8_at(mem, a, 0x660),
            max_hp: u32_at(mem, a, 0x594),
            max_mp: u32_at(mem, a, 0x598),
            hp: u32_at(mem, a, 0x5d0),
            mp: u32_at(mem, a, 0x5d4),
            ctb: u8_at(mem, a, 0x65d),
            pending: [
                u32_at(mem, a, 0x6e4),
                u32_at(mem, a, 0x6e8),
                u32_at(mem, a, 0x6ec),
            ],
        }
    }

    /// `MsCalcDamageCommand(user, target, command, formula, power, status,
    /// stat, vary, &defense, &magic_defense, fallback)` (`0x00789bf0`): the
    /// damage of a formula ([`formula_damage`]). An armor-piercing command
    /// (`+0x23` bit 0) ignores defense with `status` bit 0x40 and magic
    /// defense with 0x80; `vary` rolls the user's damage generator for the
    /// variance; healing commands (`+0x20` bit 0x10) negate it unless
    /// `status` has bit 2. The defenses used go to the two pointers when
    /// they are not null. (Formulas 0x11-0x13 divide by the user's max HP
    /// or MP, where the original faults on 0; this gives 0.)
    #[allow(clippy::too_many_arguments)]
    pub fn ms_calc_damage_command(
        mem: &mut impl Mem,
        user: u32,
        target: u32,
        command: u32,
        formula: i32,
        power: i32,
        status: u32,
        stat: i32,
        vary: i32,
        out_defense: u32,
        out_magic_defense: u32,
        fallback: i32,
    ) -> i32 {
        let g = ms_get_rnd_chr(mem, index(mem, user), 0);
        let mut defense = u32::from(u8_at(mem, target, 0x5a9));
        let mut mdef = u32::from(u8_at(mem, target, 0x5ab));
        if command != 0 && u8_at(mem, command, 0x23) & 1 != 0 {
            if status & 0x40 != 0 {
                defense = 0;
            }
            if status & 0x80 != 0 {
                mdef = 0;
            }
        }
        let variance = if vary == 0 {
            0x100
        } else {
            (brnd(mem, g) & 0x1f).cast_signed() + 0xf0
        };
        let id = u8_at(mem, target, 0xc);
        let kills = (id < 0x12).then(|| {
            KILLS
                .ptr(mem)
                .field::<u32>(u32::from(id) * KILLS_STRIDE)
                .get(mem)
                .cast_signed()
        });
        let f = Formula {
            formula,
            power,
            defense: defense.cast_signed(),
            magic_defense: mdef.cast_signed(),
            stat,
            variance,
            fallback,
            gil: GIL_AMOUNT.get(mem),
            kills,
        };
        let (mut d, early) =
            formula_damage(&formula_stats(mem, user), &formula_stats(mem, target), &f);
        if early {
            return d;
        }
        if command != 0 && u8_at(mem, command, 0x20) & 0x10 != 0 && status & 2 == 0 {
            d = d.wrapping_neg();
        }
        if out_defense != 0 {
            mem.write(out_defense, defense);
        }
        if out_magic_defense != 0 {
            mem.write(out_magic_defense, mdef);
        }
        d
    }

    /// `MsCalcEscape(chr)` (`0x0078a780`): 1 when `chr` escapes: never under
    /// rule 1 or [`DEBUG_ALL_MISS`]; always under rule 2; else when a byte of
    /// its damage roll is below 191.
    pub fn ms_calc_escape(mem: &mut impl Mem, chr: u32) -> u32 {
        let rule = ESCAPE_RULE.get(mem);
        let r = roll(mem, chr, 0) & 0xff;
        u32::from(rule != 1 && DEBUG_ALL_MISS.get(mem) == 0 && (r < 0xbf || rule == 2))
    }

    /// `DmgCalc_StatBoostAbility(user_id, target_id, command, damage)`
    /// (`0x007891e0`): + the user's dealt bonus %, − the target's taken
    /// reduction % of the physical or magic class ([`DAMAGE_RATES`]).
    pub fn dmg_calc_stat_boost_ability(
        mem: &impl Mem,
        user_id: u32,
        target_id: u32,
        command: u32,
        damage: i32,
    ) -> i32 {
        let k = match class(mem, command) {
            1 => 0,
            2 => 1,
            _ => return damage,
        };
        let rate = |id: u32, i: u32| {
            let at = DAMAGE_RATES.ptr(mem).addr.wrapping_add(id.wrapping_mul(4)) + i;
            i32::from(mem.read::<u8>(at))
        };
        let mut d = damage;
        let up = rate(user_id, k);
        if up != 0 {
            d = d.wrapping_add(up.wrapping_mul(d) / 100);
        }
        let down = rate(target_id, k + 2);
        if down != 0 {
            d = d.wrapping_add(down.wrapping_mul(d) / -100);
        }
        d
    }

    /// `DmgCalc_PowerMagicBreak(user, command, damage)` (`0x00789290`):
    /// halved under power break (physical) or magic break (magical).
    pub fn dmg_calc_power_magic_break(mem: &impl Mem, user: u32, command: u32, damage: i32) -> i32 {
        let s = u8_at(mem, user, 0x606);
        match class(mem, command) {
            1 if s & 0x10 != 0 => damage / 2,
            2 if s & 0x20 != 0 => damage / 2,
            _ => damage,
        }
    }

    /// `DmgCalc_Crit(user, target, command, &flags, damage)` (`0x00789690`):
    /// for a command that can be critical, doubles the damage (flag 0x100)
    /// when the user's damage roll % 101 is below user luck + luck level −
    /// target luck + target jinx + the bonus (the weapon's or the
    /// command's), or always with `+0x640` bit 4 or [`DEBUG_ALWAYS_CRIT`].
    pub fn dmg_calc_crit(
        mem: &mut impl Mem,
        user: u32,
        target: u32,
        command: u32,
        flags: u32,
        damage: i32,
    ) -> i32 {
        let k = u8_at(mem, command, 0x20);
        if k & 4 == 0 {
            return damage;
        }
        let r = (roll(mem, index(mem, user), 0) % 101).cast_signed();
        let bonus = if k & 8 == 0 {
            u8_at(mem, command, 0x27)
        } else {
            u8_at(mem, user, 0x5d8)
        };
        let chance = i32::from(u8_at(mem, user, 0x662)) - i32::from(u8_at(mem, target, 0x5ad))
            + i32::from(u8_at(mem, target, 0x663))
            + i32::from(u8_at(mem, user, 0x5ad))
            + i32::from(bonus);
        if r >= chance && u8_at(mem, user, 0x640) & 0x10 == 0 && DEBUG_ALWAYS_CRIT.get(mem) == 0 {
            return damage;
        }
        or_u32(mem, flags, 0x100);
        damage.wrapping_mul(2)
    }

    /// `DmgCalc_ApplyZombieDrainInteraction(user, command, info, damage)`
    /// (`0x0078a200`): a drain is reversed when exactly one of user and
    /// target (`info`) is a zombie (status bit 1).
    pub fn dmg_calc_apply_zombie_drain_interaction(
        mem: &impl Mem,
        user: u32,
        command: u32,
        info: u32,
        damage: i32,
    ) -> i32 {
        if u32_at(mem, command, 0x1c) & 0x100 != 0
            && ((u16_at(mem, user, 0x606) >> 1) & 1) + ((u16_at(mem, info, 0x14) >> 1) & 1) == 1
        {
            return damage.wrapping_neg();
        }
        damage
    }

    /// `DmgCalc_Elem(target, command, elements, damage)` (`0x0078a360`):
    /// [`element_damage`].
    pub fn dmg_calc_elem(mem: &impl Mem, target: u32, elements: u32, damage: i32) -> i32 {
        element_damage(
            elements,
            u8_at(mem, target, 0x5da),
            u8_at(mem, target, 0x5db),
            u8_at(mem, target, 0x5dc),
            u8_at(mem, target, 0x5dd),
            damage,
        )
    }

    /// `DmgCalc_Defend(target, command, &flags, info, damage)`
    /// (`0x0078a7d0`): physical damage halved while defending (`info`
    /// status 2 bit 0x800, flag 8) or under sentinel (0x2000, flag 0x10).
    pub fn dmg_calc_defend(
        mem: &mut impl Mem,
        target: u32,
        command: u32,
        flags: u32,
        info: u32,
        damage: i32,
    ) -> i32 {
        if u8_at(mem, command, 0x20) & 3 != 1 {
            return damage;
        }
        let s = u16_at(mem, info, 0x16);
        if s & 0x2800 == 0 {
            return damage;
        }
        mem.write(target.wrapping_add(0x6da), 1u8);
        or_u32(mem, flags, if s & 0x800 == 0 { 0x10 } else { 8 });
        damage / 2
    }

    /// `DmgCalc_Armored(user, target, command, info, &armored, damage)`
    /// (`0x0078a830`): a third against armor, unless the command or the
    /// user pierces it or the target is armor-broken (`info` `+0x14` bit
    /// 0x40).
    pub fn dmg_calc_armored(
        mem: &mut impl Mem,
        user: u32,
        target: u32,
        command: u32,
        info: u32,
        armored: u32,
        damage: i32,
    ) -> i32 {
        if u8_at(mem, command, 0x1e) & 1 == 0
            && u8_at(mem, target, 0x5b8) & 1 != 0
            && u16_at(mem, user, 0x6bc) & 0x2000 == 0
            && u8_at(mem, info, 0x14) & 0x40 == 0
        {
            mem.write(armored, 1u32);
            return damage / 3;
        }
        damage
    }

    fn hit_stats(mem: &impl Mem, a: u32) -> HitStats {
        HitStats {
            luck: u8_at(mem, a, 0x5ad),
            evasion: u8_at(mem, a, 0x5ae),
            accuracy: u8_at(mem, a, 0x5af),
            aim: u8_at(mem, a, 0x65f),
            reflex: u8_at(mem, a, 0x661),
            luck_level: u8_at(mem, a, 0x662),
            jinx: u8_at(mem, a, 0x663),
            dark: u8_at(mem, a, 0x60a),
        }
    }

    #[allow(clippy::verbose_bit_mask)]
    /// `DmgCalc_CheckHit(user, target, command, info, counter)`
    /// (`0x0078a890`): 0 hit, 1 miss, 2 nothing to hit (a command for the
    /// living on the dead). Hit formula 0 always hits, so do sleeping or
    /// otherwise helpless targets (`info` `+7`, `+0x14` bit 2); otherwise a
    /// hit roll % 101 below [`hit_chance`], unless `counter` is 2.
    pub fn dmg_calc_check_hit(
        mem: &mut impl Mem,
        user: u32,
        target: u32,
        command: u32,
        info: u32,
        counter: i32,
    ) -> u32 {
        let flags = u32_at(mem, command, 0x1c);
        if flags & 0x80_0000 != 0 && u16_at(mem, target, 0x606) & 3 == 0 {
            return 2;
        }
        let kind = (flags >> 3) & 7;
        if kind == 0 || u8_at(mem, info, 7) != 0 || u8_at(mem, info, 0x14) & 4 != 0 {
            return 0;
        }
        if DEBUG_ALL_MISS.get(mem) != 0 {
            return 1;
        }
        let r = (roll(mem, index(mem, user), 1) % 101).cast_signed();
        let chance = hit_chance(
            kind,
            u8_at(mem, command, 0x29),
            &hit_stats(mem, user),
            &hit_stats(mem, target),
            flags & 0x40 != 0,
            &ACCURACY_TABLE.get(mem),
        );
        u32::from((r >= chance || counter == 2) && DEBUG_ALWAYS_HIT.get(mem) == 0)
    }

    /// `DmgCalc_DmgImmunity(target, command, &what, mask, &nulled, damage)`
    /// (`0x0078aac0`): 0 against a target immune to the command's class or
    /// to all damage, clearing `mask` in `what` and counting it.
    pub fn dmg_calc_dmg_immunity(
        mem: &mut impl Mem,
        target: u32,
        command: u32,
        what: u32,
        mask: u32,
        nulled: u32,
        damage: i32,
    ) -> i32 {
        let t = u8_at(mem, target, 0x5b8);
        let class_immune = match class(mem, command) {
            1 => t & 0x20 != 0,
            2 => t & 0x40 != 0,
            _ => false,
        };
        if t & 0x80 == 0 && !class_immune {
            return damage;
        }
        let v = mem.read::<u32>(what) & !mask;
        mem.write(what, v);
        add_u32(mem, nulled, 0, 1);
        0
    }

    /// `DmgCalc_GravImmunity(target, formula, &what, mask, &nulled,
    /// damage)` (`0x0078ad80`): 0 for the gravity formulas (5, 8) against a
    /// target immune to them.
    pub fn dmg_calc_grav_immunity(
        mem: &mut impl Mem,
        target: u32,
        formula: i32,
        what: u32,
        mask: u32,
        nulled: u32,
        damage: i32,
    ) -> i32 {
        if (formula == 8 || formula == 5) && u8_at(mem, target, 0x5b8) & 2 != 0 {
            let v = mem.read::<u32>(what) & !mask;
            mem.write(what, v);
            add_u32(mem, nulled, 0, 1);
            return 0;
        }
        damage
    }

    /// `DmgCalc_Protect(command, &flags, &hit, info, damage)` (`0x0078ad40`)
    /// and `DmgCalc_Shell` (`0x0078adc0`): halved, flag 0x40 or 0x20, `hit`
    /// 2, when the command is physical (magical) and the target has
    /// protect (`info` `+0xb`) or shell (`+0xa`).
    fn protect_shell(
        mem: &mut impl Mem,
        command: u32,
        flags: u32,
        hit: u32,
        info: u32,
        damage: i32,
        (k, at, flag): (u8, u32, u32),
    ) -> i32 {
        if u8_at(mem, command, 0x20) & 3 == k && u8_at(mem, info, at) != 0 {
            mem.write(hit, 2u32);
            or_u32(mem, flags, flag);
            return damage / 2;
        }
        damage
    }

    /// `DmgCalc_Protect` (`0x0078ad40`).
    pub fn dmg_calc_protect(
        mem: &mut impl Mem,
        command: u32,
        flags: u32,
        hit: u32,
        info: u32,
        damage: i32,
    ) -> i32 {
        protect_shell(mem, command, flags, hit, info, damage, (1, 0xb, 0x40))
    }

    /// `DmgCalc_Shell` (`0x0078adc0`).
    pub fn dmg_calc_shell(
        mem: &mut impl Mem,
        command: u32,
        flags: u32,
        hit: u32,
        info: u32,
        damage: i32,
    ) -> i32 {
        protect_shell(mem, command, flags, hit, info, damage, (2, 0xa, 0x20))
    }

    /// `DmgCalc_ShatterFail(target, info, status, &what, counts, damage)`
    /// (`0x0078ba30`): a petrified target (`info` `+0x14` bit 2) hit by a
    /// petrifying command that is not a shatter takes nothing.
    pub fn dmg_calc_shatter_fail(
        mem: &mut impl Mem,
        info: u32,
        status: u32,
        what: u32,
        counts: u32,
        damage: u32,
    ) {
        if u8_at(mem, info, 0x14) & 4 != 0 && status & 4 != 0 && u8_at(mem, info, 0x17) & 1 == 0 {
            mem.write(what, 0u32);
            mem.write(damage, [0u32; 3]);
            add_u32(mem, counts, 0x18, 1);
        }
    }

    /// `DmgCalc_Threaten(target, info, &what, mask, flags, counts, damage)`
    /// (`0x0078bd50`): a threatened target (`info` `+0x14` bit 0x800) takes
    /// no delay; returns `flags`.
    #[allow(clippy::too_many_arguments)]
    pub fn dmg_calc_threaten(
        mem: &mut impl Mem,
        info: u32,
        what: u32,
        mask: u32,
        flags: u32,
        counts: u32,
        damage: u32,
    ) -> u32 {
        if flags & 4 != 0 && u16_at(mem, info, 0x14) & 0x800 != 0 {
            mem.write(damage.wrapping_add(8), 0u32);
            let v = mem.read::<u32>(what) & !mask;
            mem.write(what, v);
            add_u32(mem, counts, 4, 1);
        }
        flags
    }

    /// `DmgCalc_ReelBonus(user, damage)` (`0x0078bd90`): with Wakka's reels
    /// spun (`+0xd26`), `damage × (time + 2·start) / (2·start)` from the
    /// floats `MsSetReelResult` stores (`+0xd2c` time, `+0xd30` start). The
    /// original computes on the x87 at 53 bits with `damage` stored as an
    /// `f32` first, and converts with `_ftol2_sse`: the same as `f64` here.
    pub fn dmg_calc_reel_bonus(mem: &impl Mem, user: u32, damage: i32) -> u32 {
        if u8_at(mem, user, 0xd26) == 0 {
            return damage.cast_unsigned();
        }
        let start = f64::from(mem.read::<f32>(user.wrapping_add(0xd30)));
        let time = f64::from(mem.read::<f32>(user.wrapping_add(0xd2c)));
        let two = start + start;
        #[allow(clippy::cast_precision_loss)] // fild, then fstp dword
        let d = f64::from(damage as f32);
        ffx_bridge::x87::ftol(mem, (time + two) * d / two)
    }

    /// `DmgCalc_CheckNuls(command, elements, info)` (`0x0078bfb0`): -1 when
    /// every element of fire, ice, thunder and water the command has meets
    /// its Nul status (`info` counters `+0xe`, `+0x10`, `+0xf`, `+0xd`),
    /// which then lose a charge (unless 0xfe or 0xff); else 0.
    pub fn dmg_calc_check_nuls(mem: &mut impl Mem, elements: u32, info: u32) -> i32 {
        let mut need = 0;
        let mut have = 0;
        let mut hit = [0u8; 4];
        for (i, (bit, at)) in [(1, 0xe), (4, 0xf), (2, 0x10), (8, 0xd)]
            .into_iter()
            .enumerate()
        {
            if elements & bit != 0 {
                need += 1;
                let c = u8_at(mem, info, at);
                if c != 0 {
                    have += 1;
                    hit[i] = c;
                }
            }
        }
        if have == 0 || need > have {
            return 0;
        }
        for (c, at) in hit.into_iter().zip([0xe, 0xf, 0x10, 0xd]) {
            if c != 0 && c < 0xfe {
                mem.write(info.wrapping_add(at), c - 1);
            }
        }
        -1
    }

    /// `DmgCalc_Berserk(user, damage)` (`0x0078c0b0`): ×1.5 for a berserk
    /// user's (`+0x606` bit 0x200) attack command (`+0xf5c` the command,
    /// `+0x6c6` its attack).
    pub fn dmg_calc_berserk(mem: &impl Mem, user: u32, damage: i32) -> i32 {
        if u16_at(mem, user, 0x606) & 0x200 != 0
            && u16_at(mem, user, 0xf5c) == u16_at(mem, user, 0x6c6)
        {
            return damage.wrapping_mul(3) / 2;
        }
        damage
    }

    /// `DmgCalc_QueueRemoveStatusOnHit(target_id, target, command, removed,
    /// counts, info)` (`0x0078c0f0`): a physical hit wakes the target
    /// (`info` `+7`, setting `+0x50f`) and ends confusion (`info` `+0x14`
    /// bit 0x100) unless it is forced (`+0x62b` bit 0).
    pub fn dmg_calc_queue_remove_status_on_hit(
        mem: &mut impl Mem,
        target: u32,
        command: u32,
        counts: u32,
        info: u32,
    ) {
        if u8_at(mem, command, 0x20) & 3 != 1 {
            return;
        }
        let s = u16_at(mem, info, 0x14);
        if s & 0x100 != 0 && u8_at(mem, target, 0x62b) & 1 == 0 {
            mem.write(info.wrapping_add(0x14), s & 0xfeff);
            add_u32(mem, counts, 4, 1);
        }
        if u8_at(mem, info, 7).wrapping_sub(1) < 0xfe {
            mem.write(target.wrapping_add(0x50f), 1u8);
            mem.write(info.wrapping_add(7), 0u8);
            add_u32(mem, counts, 8, 1);
        }
    }

    /// `DmgCalc_DelayImmunity(target, damage, counts, _, flags, delayed)`
    /// (`0x0078c180`): no delay for a target immune to it (`+0x5b9` bit
    /// 0) unless `delayed`; returns `flags`.
    pub fn dmg_calc_delay_immunity(
        mem: &mut impl Mem,
        target: u32,
        damage: u32,
        counts: u32,
        flags: u32,
        delayed: i32,
    ) -> u32 {
        if flags & 4 != 0 {
            if u8_at(mem, target, 0x5b9) & 1 != 0 && delayed == 0 {
                add_u32(mem, counts, 0x14, 1);
                mem.write(damage.wrapping_add(8), 0u32);
            } else if u32_at(mem, damage, 8) != 0 {
                add_u32(mem, counts, 0x10, 1);
            }
        }
        flags
    }

    /// `MsCheckStatCount(i)` (`0x0079e330`): byte 3 of entry `i` of the
    /// counter statuses' table (`0x00c42464`).
    pub fn ms_check_stat_count(mem: &impl Mem, i: i32) -> u32 {
        let at = STAT_COUNT_TABLE
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4))
            + 3;
        u32::from(mem.read::<u8>(at))
    }

    /// The counter statuses' table, 13 × 4 bytes.
    pub const STAT_COUNT_TABLE: Global<u32> = Global::new(0x00c4_2464);

    /// `MsCheckWrongStatus(command)` (`0x0079e340`): -1 when a command that
    /// does not heal inflicts something harmful: it hits HP, MP or CTB
    /// (`+0x23` & 7), or any status whose table byte 3 has bit 7 (statuses
    /// `+0x2e`, 12; counters `+0x3a` with a duration at `+0x47`, 13; status
    /// bits 2 `+0x54`, 16).
    pub fn ms_check_wrong_status(mem: &impl Mem, command: u32) -> i32 {
        use crate::state::bridge::{STATUS_TABLE, STATUS_TABLE2};
        if u8_at(mem, command, 0x20) & 0x10 != 0 {
            return 0;
        }
        if u8_at(mem, command, 0x23) & 7 != 0 {
            return -1;
        }
        let bad = |table: u32, i: u32| mem.read::<u8>(table + 4 * i + 3) & 0x80 != 0;
        let t1 = STATUS_TABLE.ptr(mem).addr;
        if (0..12).any(|i| u8_at(mem, command, 0x2e + i) != 0 && bad(t1, i)) {
            return -1;
        }
        let t3 = STAT_COUNT_TABLE.ptr(mem).addr;
        if (0..13).any(|i| {
            u8_at(mem, command, 0x3a + i) != 0 && u8_at(mem, command, 0x47 + i) != 0 && bad(t3, i)
        }) {
            return -1;
        }
        let t2 = STATUS_TABLE2.ptr(mem).addr;
        let s2 = u16_at(mem, command, 0x54);
        if (0..16).any(|i| (s2 >> i) & 1 != 0 && bad(t2, i)) {
            return -1;
        }
        0
    }

    /// `DmgCalc_CheckCounter(apply, user_id, user, target_id, target,
    /// command)` (`0x0078c1d0`): whether the target counters: 2 with
    /// evade & counter, 1 with counterattack (physical) or magic counter
    /// (magical); never for itself, an uncounterable command (`+0x1a` &
    /// 0xc), a user that may not be countered (`+0xdf1`) or with counters
    /// off ([`COUNTER_ON`]). With `apply`, a harmful command
    /// ([`ms_check_wrong_status`]) sets the target's `+0x6dc`.
    pub fn dmg_calc_check_counter(
        mem: &mut impl Mem,
        apply: i32,
        user_id: i32,
        user: u32,
        target_id: i32,
        target: u32,
        command: u32,
    ) -> i32 {
        if u8_at(mem, command, 0x1a) & 0xc != 0
            || u8_at(mem, user, 0xdf1) != 0
            || user_id == target_id
            || COUNTER_ON.get(mem) == 0
        {
            return 0;
        }
        let a = u16_at(mem, target, 0x6bc);
        let r = match class(mem, command) {
            1 if a & 0x10 != 0 => 2,
            1 if a & 8 != 0 => 1,
            2 if a & 0x20 != 0 => 1,
            _ => 0,
        };
        if apply != 0 && r != 0 && ms_check_wrong_status(mem, command) != 0 {
            mem.write(target.wrapping_add(0x6dc), 1u8);
        }
        r
    }

    /// `DmgCalc_NoDamageDeath(target, info, status, flags, damage)`
    /// (`0x0078c480`): a target that ends up KO'd (`info` `+0x14` bit 0) by
    /// a command that does not kill (`status` bit 0) takes no damage; the
    /// HP, MP and CTB flags go.
    pub fn dmg_calc_no_damage_death(
        mem: &mut impl Mem,
        info: u32,
        status: u32,
        flags: u32,
        damage: u32,
    ) -> u32 {
        if status & 1 == 0 && u8_at(mem, info, 0x14) & 1 != 0 {
            mem.write(damage, [0u32; 3]);
            return flags & !7;
        }
        flags
    }

    /// `DmgCalc_ShieldBoost(target, &flags, damage)` (`0x0078c530`): a
    /// quarter under shield (flag 0x8000, `+0x6da`), ×1.5 under boost.
    pub fn dmg_calc_shield_boost(mem: &mut impl Mem, target: u32, flags: u32, damage: i32) -> i32 {
        let s = u8_at(mem, target, 0x616);
        let mut d = damage;
        if s & 0x40 != 0 {
            mem.write(target.wrapping_add(0x6da), 1u8);
            d /= 4;
            or_u32(mem, flags, 0x8000);
        }
        if s & 0x80 != 0 {
            d = d.wrapping_mul(3) / 2;
        }
        d
    }

    /// `DmgCalc_MagicBooster(user, command, command_id, &damage)`
    /// (`0x0078c580`): ×1.5 (when `damage` is not null) for command 0x311f
    /// on a booster charge (`+0x5ca`, used up; returns 0), or for a black
    /// or white magic (`+0x17` 1 or 2) with the magic booster ability
    /// (returns -1).
    pub fn dmg_calc_magic_booster(
        mem: &mut impl Mem,
        user: u32,
        command: u32,
        command_id: i32,
        damage: u32,
    ) -> i32 {
        let boost = |mem: &mut _| {
            if damage != 0 {
                let d: i32 = Mem::read(mem, damage);
                Mem::write(mem, damage, d.wrapping_mul(3) / 2);
            }
        };
        if command_id == 0x311f {
            if u8_at(mem, user, 0x5ca) != 0 {
                mem.write(user.wrapping_add(0x5ca), 0u8);
                boost(mem);
            }
            return 0;
        }
        if u8_at(mem, user, 0x6bc) & 0x40 != 0 && matches!(u8_at(mem, command, 0x17), 1 | 2) {
            boost(mem);
            return -1;
        }
        0
    }

    /// `DmgCalc_Alchemy(user, command, command_id, formula, damage)`
    /// (`0x0078c5f0`): doubled for an item (`0x2xxx`) that heals with
    /// formula 6 or 8 when the user has alchemy.
    pub fn dmg_calc_alchemy(
        mem: &impl Mem,
        user: u32,
        command: u32,
        command_id: u32,
        formula: i32,
        damage: i32,
    ) -> i32 {
        if u16_at(mem, user, 0x6bc) & 0x200 != 0
            && command_id & 0xffff_f000 == 0x2000
            && u8_at(mem, command, 0x20) & 0x10 != 0
            && (formula == 6 || formula == 8)
        {
            return damage.wrapping_mul(2);
        }
        damage
    }

    /// `DmgCalc_HitKind(missed, counts)` (`0x0078c640`): what `DmgCalc`
    /// stores at `info` `+1`: 1 when a status was nulled (`counts[1]`) and
    /// `missed` is 0, 2 when none was but `counts[0]`, else 0.
    pub fn dmg_calc_hit_kind(mem: &impl Mem, missed: i32, counts: u32) -> u32 {
        let [c0, c1]: [u32; 2] = mem.read(counts);
        if c1 == 0 && c0 != 0 {
            2
        } else {
            u32::from(missed == 0 && c1 != 0)
        }
    }

    /// `DmgCalc_IsDistill(status2)` (`0x0078c670`): -1 when one of the
    /// distill statuses (bits 1-5) is among `status2`'s low byte.
    #[must_use]
    pub fn dmg_calc_is_distill(status2: u32) -> i32 {
        -i32::from(status2 & 0x3e != 0)
    }

    /// `DmgCalc_MarkDamaged(user_id, user, command, damage, always)`
    /// (`0x0078bf50`): 1, setting the user's `+0x715`, when `always` or any
    /// of the three damage values hurts (heals, for a healing command).
    pub fn dmg_calc_mark_damaged(
        mem: &mut impl Mem,
        user: u32,
        command: u32,
        damage: u32,
        always: i32,
    ) -> u32 {
        let d: [i32; 3] = mem.read(damage);
        let heals = u8_at(mem, command, 0x20) & 0x10 != 0;
        let any = if heals {
            d.iter().any(|&x| x < 0)
        } else {
            d.iter().any(|&x| x >= 1)
        };
        if always != 0 || any {
            mem.write(user.wrapping_add(0x715), 1u8);
            return 1;
        }
        0
    }

    /// `MsAdjustCTB(_, ctb, slow, haste)` (`0x0078c150`): halved under
    /// haste, doubled under slow, clamped to 0-255.
    #[must_use]
    pub fn ms_adjust_ctb(ctb: i32, haste: i32, slow: i32) -> i32 {
        let mut c = ctb;
        if haste != 0 {
            c /= 2;
        }
        if slow != 0 {
            c = c.wrapping_mul(2);
        }
        ffx_kernel::bridge::ms_check_range(c, 0, 0xff)
    }

    /// `MsGetNextCTB(chr, rank, haste, slow)` (`0x0078d1d0`): the CTB of
    /// an action of `rank` (at least 1) for the actor's agility.
    pub fn ms_get_next_ctb(mem: &mut impl Mem, chr: u32, rank: i32, haste: i32, slow: i32) -> i32 {
        let rank = rank.max(1);
        let base = ffx_kernel::bridge::ms_get_rom_ctb_count(mem, i32::from(u8_at(mem, chr, 0x5ac)));
        ms_adjust_ctb(i32::from(base).wrapping_mul(rank), haste, slow)
    }

    /// `DmgCalc_Delay(target, command, flags, damage)` (`0x0078e0f0`):
    /// delay attacks add 1.5 (weak, `+0x1c` bit 0x2000) or 3 (strong, bit
    /// 0x4000) times the target's base CTB to its CTB damage; flag 4.
    pub fn dmg_calc_delay(
        mem: &mut impl Mem,
        target: u32,
        command: u32,
        flags: u32,
        damage: u32,
    ) -> u32 {
        let f = u32_at(mem, command, 0x1c);
        let k = if f & 0x4000 != 0 {
            2
        } else if f & 0x2000 != 0 {
            1
        } else {
            return flags;
        };
        let base =
            ffx_kernel::bridge::ms_get_rom_ctb_count(mem, i32::from(u8_at(mem, target, 0x5ac)));
        let add = i32::from(base) * 3 * k / 2;
        add_u32(mem, damage, 8, add.cast_unsigned());
        flags | 4
    }

    /// `DmgCalc_ApplyStatUpBuffs(user, target, command)` (`0x0078d730`):
    /// raises the target's cheer, aim, focus, reflex, luck and jinx levels
    /// the command's `+0x56` bits name by `+0x59` (at least 1), to 5 at
    /// most.
    pub fn dmg_calc_apply_stat_up_buffs(mem: &mut impl Mem, target: u32, command: u32) {
        let bits = u16_at(mem, command, 0x56);
        let n = i32::from(u8_at(mem, command, 0x59)).max(1);
        for i in 0..6 {
            if bits & (1 << i) != 0 {
                let at = 0x65e + i;
                let v =
                    ffx_kernel::bridge::ms_check_range(i32::from(u8_at(mem, target, at)) + n, 0, 5);
                mem.write(target.wrapping_add(at), v.to_le_bytes()[0]);
            }
        }
    }

    /// `DebugCanTakeDamage(chr)` (`0x0078d3a0`): 0 for a monster under the
    /// debug flag `0x0112a8f8`, or another actor under `0x0112a8f9`.
    pub fn debug_can_take_damage(mem: &impl Mem, chr: u32) -> u32 {
        let monster = ms_get_ram_chr_monster(chr);
        let off = if monster == 1 {
            DEBUG_NO_DAMAGE_MONSTER.get(mem)
        } else {
            DEBUG_NO_DAMAGE_PARTY.get(mem)
        };
        u32::from(off == 0)
    }

    /// Debug: monsters take no damage (`DebugCanTakeDamage`).
    pub const DEBUG_NO_DAMAGE_MONSTER: Global<u8> = Global::new(0x0112_a8f8);
    /// Debug: the party takes no damage (`DebugCanTakeDamage`).
    pub const DEBUG_NO_DAMAGE_PARTY: Global<u8> = Global::new(0x0112_a8f9);

    /// `SetUpDamageInfo(info, parent, target)` (`0x0078dae0`): the status
    /// part of a result (`+6`-`+0x17`) from `parent`'s, or else from the
    /// target's statuses (`+0x606`, `+0x608`, `+0x616`, `+0x640`).
    pub fn set_up_damage_info(mem: &mut impl Mem, info: u32, parent: u32, target: u32) {
        let (status, counters, status2, flags): (u16, [u8; 13], u16, u8) = if parent != 0 {
            (
                u16_at(mem, parent, 0x14),
                mem.read(parent.wrapping_add(7)),
                u16_at(mem, parent, 0x16),
                u8_at(mem, parent, 6),
            )
        } else {
            (
                u16_at(mem, target, 0x606),
                mem.read(target.wrapping_add(0x608)),
                u16_at(mem, target, 0x616),
                u8_at(mem, target, 0x640),
            )
        };
        mem.write(info.wrapping_add(0x14), status);
        mem.write(info.wrapping_add(7), counters);
        mem.write(info.wrapping_add(0x16), status2);
        mem.write(info.wrapping_add(6), flags);
    }

    /// What `DmgCalc_InflictStatus` and `_Extra` count per target, eight
    /// `i32` each: `[0]` statuses changed, `[1]` of those flagged 0x40 in
    /// the status table, `[2]` 0x20, `[4]` changes made, `[5]` immune,
    /// `[6]` failed, `[7]` flagged 0x80.
    fn count(mem: &mut impl Mem, counts: u32, i: u32, n: u32) {
        add_u32(mem, counts, 4 * i, n);
    }

    /// The counts by the status table's byte (`MsCheckStat`, `MsCheckStat2`,
    /// `MsCheckStatCount`).
    fn tally(
        mem: &mut impl Mem,
        kind: u8,
        inflicted: u32,
        removed: u32,
        did: u32,
        undid: u32,
        with_20: bool,
    ) {
        let at = u32::from(kind & 0x40 != 0);
        count(mem, inflicted, at, did);
        count(mem, removed, at, undid);
        if with_20 && kind & 0x20 != 0 {
            count(mem, inflicted, 2, did);
            count(mem, removed, 2, undid);
        }
        if kind & 0x80 != 0 {
            count(mem, inflicted, 7, did);
            count(mem, removed, 7, undid);
        }
    }

    fn low16(v: u32) -> u16 {
        let [b0, b1, ..] = v.to_le_bytes();
        u16::from_le_bytes([b0, b1])
    }

    fn set_u16(mem: &mut impl Mem, p: u32, o: u32, v: u16) {
        mem.write(p.wrapping_add(o), v);
    }

    /// `DmgCalc_InflictStatus(user_id, user, target_id, target, command,
    /// inflicted, removed, info, flags, damage, &delayed)` (`0x0078ae00`):
    /// the command's 25 status chances (`+0x2e`, the weapon's at user
    /// `+0x5de` when the command uses it and they are higher) against the
    /// target's resistances (`+0x641`): statuses 0-11 are bits of `info`
    /// `+0x14`, 12-24 the counters `info` `+7`, with their durations at
    /// `+0x47` (`+0x5f7`). A status lands when the user's status roll % 101
    /// is below chance − resistance (always at chance 255 or 254, never at
    /// resistance 255), or for status 11 (threaten) when roll % 100 is
    /// below the resistance itself; removing commands (`+0x20` bit 0x20)
    /// always reach. Returns `flags` with what immunities show (0x200-0x800,
    /// 0x2000, 0x4000) and 4 when threaten delayed the target.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn dmg_calc_inflict_status(
        mem: &mut impl Mem,
        user_id: u32,
        user: u32,
        target_id: u32,
        target: u32,
        command: u32,
        inflicted: u32,
        removed: u32,
        info: u32,
        flags: u32,
        damage: u32,
        delayed: u32,
    ) -> u32 {
        use crate::state::bridge::{UNDERWATER, ms_check_stat};
        let mut flags = flags;
        let weapon = (u32_at(mem, command, 0x1c) >> 0x12) & 1 != 0;
        let removing = u8_at(mem, command, 0x20) & 0x20 != 0;
        for i in 0..25u32 {
            let bit = 1u32 << i;
            let mut chance = u8_at(mem, command, 0x2e + i);
            if weapon {
                chance = chance.max(u8_at(mem, user, 0x5de + i));
            }
            if chance == 0 {
                continue;
            }
            let counter = i.checked_sub(12);
            let mut resist = u8_at(mem, target, 0x641 + i);
            let cbit = bit >> 12;
            let mut hit = false;
            if removing {
                hit = true;
            } else if counter.is_none() && bit & 0x800 != 0 {
                let r = roll(mem, user_id, 2).cast_signed() % 100;
                if r < i32::from(resist) {
                    hit = true;
                } else if resist == 0 {
                    resist = 0xff;
                }
            } else {
                let r = roll(mem, user_id, 2).cast_signed();
                if i == 0 && u8_at(mem, info, 0x14) & 2 != 0 {
                    resist = 0xfe;
                }
                hit = chance == 0xff
                    || (resist != 0xff
                        && (chance == 0xfe
                            || r % 101 < i32::from(chance) - i32::from(resist)
                            || DEBUG_ALWAYS_HIT.get(mem) != 0));
            }
            let mut kind = 0u8;
            let mut did = 0;
            let mut undid = 0;
            // 0x1800 here: haste or slow failed, no CTB damage
            let mut ctb_off = 0;
            let mut fail = false;
            if hit && DEBUG_ALL_MISS.get(mem) == 0 {
                if let Some(k) = counter {
                    let mut dur = u8_at(mem, command, 0x47 + k);
                    if weapon {
                        dur = dur.max(u8_at(mem, user, 0x5f7 + k));
                    }
                    kind = ms_check_stat_count(mem, k.cast_signed()).to_le_bytes()[0];
                    let status = u16_at(mem, info, 0x14);
                    if status & 4 != 0 {
                        fail = true;
                        ctb_off = cbit;
                    } else if !removing {
                        let table = STAT_COUNT_TABLE.ptr(mem).addr + 4 * k + 3;
                        if user_id == target_id && dur < 0xfd && mem.read::<u8>(table) & 4 != 0 {
                            dur += 1;
                        }
                        let free = u8_at(mem, info, 0x12) != 0xff
                            && u8_at(mem, info, 0x13) != 0xff
                            && status & 0x800 == 0;
                        if u8_at(mem, target, 0x5fc + i) != 0
                            || (cbit & (0x800 | 0x1000) != 0 && cbit & 1 == 0 && !free)
                        {
                            fail = true;
                            ctb_off = cbit;
                        } else {
                            mem.write(info.wrapping_add(7 + k), dur);
                            if cbit & 1 != 0 {
                                let s2 = u16_at(mem, info, 0x16) & 0xf7ff;
                                set_u16(mem, info, 0x16, s2);
                            } else if cbit & 0x800 != 0 {
                                mem.write(info.wrapping_add(0x13), 0u8);
                                set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) & 0xf7ff);
                            } else if cbit & 0x1000 != 0 {
                                mem.write(info.wrapping_add(0x12), 0u8);
                                set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) & 0xf7ff);
                            }
                            count(mem, inflicted, 4, 1);
                            did = 1;
                        }
                    } else {
                        let cur = u8_at(mem, info, 7 + k);
                        if cur == 0 || cur == 0xff {
                            fail = true;
                            ctb_off = cbit;
                        } else {
                            undid = 1;
                            let v = ffx_kernel::bridge::ms_check_range(
                                i32::from(cur) - i32::from(dur),
                                0,
                                0xff,
                            );
                            mem.write(info.wrapping_add(7 + k), v.to_le_bytes()[0]);
                            count(mem, inflicted, 4, 1);
                        }
                    }
                } else {
                    kind = ms_check_stat(mem, i.cast_signed()).to_le_bytes()[0];
                    let status = u16_at(mem, info, 0x14);
                    #[allow(clippy::cast_possible_truncation)] // bits 0-11
                    let b16 = bit as u16;
                    if !removing {
                        if status & 4 != 0 || bit & u32::from(u16_at(mem, target, 0x606)) != 0 {
                            fail = true;
                        } else {
                            let forced = u16_at(mem, target, 0x62a) & 0xf00 != 0;
                            match bit {
                                1 => {
                                    match u16_at(mem, user, 0xf5c) {
                                        0x30e2 => mem.write(target.wrapping_add(0x701), 3u8),
                                        0x3120 => mem.write(target.wrapping_add(0x701), 4u8),
                                        _ => {}
                                    }
                                    set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) | b16);
                                    count(mem, inflicted, 4, 1);
                                    did = 1;
                                }
                                0x100 | 0x200 | 0x400 if forced => fail = true,
                                0x100 => {
                                    set_u16(mem, info, 0x14, (b16 | status) & 0xf1ff);
                                    count(mem, inflicted, 4, 1);
                                    did = 1;
                                }
                                0x200 => {
                                    set_u16(mem, info, 0x14, (b16 | status) & 0xf2ff);
                                    count(mem, inflicted, 4, 1);
                                    did = 1;
                                }
                                0x400 => {
                                    set_u16(mem, info, 0x14, b16 | status);
                                    mem.write(target.wrapping_add(0x5c4), user_id.to_le_bytes()[0]);
                                    set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) & 0xf4ff);
                                    count(mem, inflicted, 4, 1);
                                    did = 1;
                                }
                                0x800 => {
                                    let free = u8_at(mem, info, 0x12) != 0xff
                                        && u8_at(mem, info, 0x13) != 0xff;
                                    if forced || !free {
                                        fail = true;
                                    } else {
                                        did = 1;
                                        mem.write(delayed, 1u32);
                                        if resist != 0 {
                                            #[allow(clippy::cast_possible_truncation)]
                                            // the low byte
                                            let c = ((u32::from(resist) * 7) / 10) as u8;
                                            mem.write(target.wrapping_add(0x641 + i), c.max(1));
                                        }
                                        set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) | b16);
                                        flags |= 4;
                                        mem.write(
                                            target.wrapping_add(0x5c5),
                                            user_id.to_le_bytes()[0],
                                        );
                                        let ctb = ms_get_next_ctb(
                                            mem,
                                            user,
                                            i32::from(u8_at(mem, user, 0xde8)),
                                            i32::from(u8_at(mem, user, 0x613)),
                                            i32::from(u8_at(mem, user, 0x614)),
                                        );
                                        let v = ctb
                                            .cast_unsigned()
                                            .wrapping_add(u32::from(u8_at(mem, user, 0x65c)))
                                            .wrapping_sub(u32::from(u8_at(mem, target, 0x65c)));
                                        mem.write(damage.wrapping_add(8), v);
                                        set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) & 0xf8ff);
                                        set_u16(mem, info, 0x12, 0);
                                        count(mem, inflicted, 4, 1);
                                    }
                                }
                                4 => {
                                    // petrified: nothing else counts
                                    did = 1;
                                    set_u16(mem, info, 0x14, b16);
                                    mem.write(info.wrapping_add(7), [0u8; 13]);
                                    set_u16(mem, info, 0x16, u16_at(mem, info, 0x16) & 0x813f);
                                    let monster = crate::actor::bridge::ms_get_ram_chr_monster(
                                        u32::from(u8_at(mem, target, 0xc)),
                                    );
                                    if monster != 0 || UNDERWATER.get(mem) != 0 {
                                        set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) | 1);
                                        set_u16(mem, info, 0x16, u16_at(mem, info, 0x16) | 0x100);
                                    }
                                    count(mem, inflicted, 4, 1);
                                }
                                _ => {
                                    did = 1;
                                    set_u16(mem, info, 0x14, b16 | status);
                                    count(mem, inflicted, 4, 1);
                                }
                            }
                        }
                    } else if status & 4 != 0 && bit & 4 == 0 {
                        fail = true;
                    } else if i == 0 && status & 2 != 0 {
                        // life on a zombie
                        if u8_at(mem, target, 0x5b8) & 4 != 0 {
                            count(mem, inflicted, 5, 1);
                        } else {
                            set_u16(mem, info, 0x14, b16 | status);
                            count(mem, inflicted, 4, 1);
                            did = 1;
                        }
                    } else if bit & u32::from(status) == 0
                        || bit & u32::from(u16_at(mem, target, 0x62a)) != 0
                    {
                        fail = true;
                    } else {
                        undid = 1;
                        set_u16(mem, info, 0x14, !b16 & status);
                        if bit & 1 != 0 {
                            count(mem, removed, 3, 1);
                        }
                        count(mem, inflicted, 4, 1);
                    }
                }
            } else {
                if counter.is_some() {
                    ctb_off = cbit;
                }
                if resist == 0xff {
                    if counter.is_none() {
                        if bit & 4 != 0 {
                            flags |= 0x4000;
                        }
                        if bit & 2 != 0 {
                            flags |= 0x2000;
                        }
                    } else {
                        for (c, f) in [(1, 0x200), (2, 0x400), (4, 0x800)] {
                            if cbit & c != 0 {
                                flags |= f;
                            }
                        }
                    }
                    count(mem, inflicted, 5, 1);
                } else {
                    fail = true;
                }
            }
            if fail {
                count(mem, inflicted, 6, 1);
            }
            tally(mem, kind, inflicted, removed, did, undid, true);
            if ctb_off & 0x1800 != 0 {
                mem.write(damage.wrapping_add(8), 0u32);
            }
        }
        flags
    }

    /// `DmgCalc_InflictStatus_Extra(user_id, user, target_id, target,
    /// command, inflicted, removed, info, ret, damage, status2)`
    /// (`0x0078b4e0`): the second status word's bits `status2` (the
    /// command's `+0x54`, or the weapon's): on a petrified target first a
    /// roll % 101 below the command's `+0x2c` adds KO and bit 0x100; then
    /// each bit is removed (removing commands) or set unless the target is
    /// immune (`+0x65a`), has it (`+0x616`, but KO) or two distills meet;
    /// the distills (bits 1-5) replace each other. Returns `ret`.
    #[allow(clippy::too_many_arguments)]
    pub fn dmg_calc_inflict_status_extra(
        mem: &mut impl Mem,
        user_id: u32,
        user: u32,
        target: u32,
        command: u32,
        inflicted: u32,
        removed: u32,
        info: u32,
        ret: u32,
        status2: u32,
    ) -> u32 {
        use crate::state::bridge::ms_check_stat2;
        let mut bits = status2;
        if u8_at(mem, info, 0x14) & 4 != 0 {
            let r = roll(mem, user_id, 2).cast_signed() % 101;
            if r < i32::from(u8_at(mem, command, 0x2c)) {
                bits |= 0x100;
                set_u16(mem, info, 0x14, u16_at(mem, info, 0x14) | 1);
            }
        }
        let removing = u8_at(mem, command, 0x20) & 0x20 != 0;
        for i in 0..16u32 {
            let bit = 1u32 << i;
            if bits & bit == 0 {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)] // 16 bits
            let b16 = bit as u16;
            let kind = ms_check_stat2(mem, i.cast_signed()).to_le_bytes()[0];
            let (mut did, mut undid) = (0, 0);
            let status = u16_at(mem, info, 0x14);
            let s2 = u16_at(mem, info, 0x16);
            let open = DEBUG_ALL_MISS.get(mem) == 0
                && (status & 4 == 0 || bits & 0x100 != 0 || bits & 0x3f != 0);
            if !open {
                count(mem, inflicted, 6, 1);
            } else if removing {
                if bit & u32::from(s2) == 0 || bit & u32::from(u16_at(mem, target, 0x62e)) != 0 {
                    count(mem, inflicted, 6, 1);
                } else {
                    if bit & 0x200 != 0 {
                        mem.write(target.wrapping_add(0x5ca), 0u8);
                    }
                    set_u16(mem, info, 0x16, s2 & !b16);
                    count(mem, inflicted, 4, 1);
                    undid = 1;
                }
            } else if bit & u32::from(u16_at(mem, target, 0x65a)) != 0 {
                count(mem, inflicted, 5, 1);
            } else if (bit & u32::from(u16_at(mem, target, 0x616)) != 0 && bit & 1 == 0)
                || (dmg_calc_is_distill(u32::from(u16_at(mem, target, 0x62e))) != 0
                    && dmg_calc_is_distill(bit) != 0)
            {
                count(mem, inflicted, 6, 1);
            } else {
                did = 1;
                let keep = match bit {
                    2 => 0xffc3,
                    4 => 0xffc5,
                    8 => 0xffc9,
                    0x10 => 0xffd1,
                    0x20 => 0xffe1,
                    _ => 0xffff,
                };
                if bit == 0x4000 {
                    let v = u8_at(mem, target, 0x5c9);
                    mem.write(target.wrapping_add(0x5c8), v);
                } else if bit == 0x100 {
                    set_u16(mem, info, 0x16, s2 | b16);
                    set_u16(mem, info, 0x14, status & 0xf7ff);
                } else if bit == 0x200 && dmg_calc_magic_booster(mem, user, command, 0, 0) != 0 {
                    mem.write(target.wrapping_add(0x5ca), 1u8);
                }
                let s2 = u16_at(mem, info, 0x16);
                set_u16(mem, info, 0x16, (s2 & keep) | b16);
                count(mem, inflicted, 4, 1);
            }
            tally(mem, kind, inflicted, removed, did, undid, false);
        }
        ret
    }

    /// What `DmgCalc_ResultType` decides on.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct ResultInputs {
        pub user_id: u32,
        pub target_id: u32,
        /// The command's class (`+0x20` & 3).
        pub class: i32,
        pub armored: i32,
        pub defense: i32,
        pub magic_defense: i32,
        /// HP, MP and CTB damage.
        pub damage: [i32; 3],
        pub inflicted: [i32; 8],
        pub removed: [i32; 8],
        /// The result's status bits (`info` `+0x14`), low byte.
        pub status: u8,
        /// The result's first counter (`info` `+7`).
        pub asleep: i32,
        /// What the command hits (`+0x23`) as `DmgCalc` keeps it.
        pub what: u32,
        /// The target's `+0x447`: 1 guards against physical, 2 magical.
        pub guard: u8,
    }

    /// `DmgCalc_ResultType` (`0x0078c270`): the reaction stored at `info`
    /// `+0`. With damage: 5, or 4 (physical) or 3 (magical) when the target
    /// guards against armored or high-defense (100+) hits; 6 or 5 for drains
    /// (`what` 0x100). Without: 4/5 for a dodged physical hit, 8 KO'd by a
    /// table-0x40 status, 0xb/0xc by 0x20 ones, 2 revived or healed, 7 a
    /// status removed, 9/0xa status changes, else 0.
    #[must_use]
    pub fn result_type(r: &ResultInputs) -> u32 {
        let [d0, d1, d2] = r.damage;
        let max = d0.max(d1);
        let min = d0.min(d1);
        if max > 0 {
            let mut guarded = false;
            let mut kind = 5;
            let high =
                (r.magic_defense >= 100 && r.class == 2) || (r.defense >= 100 && r.class == 1);
            if r.armored != 0 || high {
                if r.class == 2 && r.guard & 2 != 0 && r.status & 0x80 == 0 {
                    guarded = true;
                    kind = 3;
                }
                if r.class == 1 && r.guard & 1 != 0 && r.status & 0x40 == 0 {
                    guarded = true;
                    kind = 4;
                }
            }
            if r.what & 0x100 != 0 {
                return 6 - u32::from(guarded);
            }
            return kind;
        }
        let hits = r.what & 3;
        if r.class == 1 && hits != 0 {
            return u32::from(!r.guard & 1) | 4;
        }
        if r.inflicted[1] != 0 {
            return 8;
        }
        if r.inflicted[2] != 0 {
            return 0xb;
        }
        if r.removed[2] != 0 {
            return 0xc;
        }
        let me = r.user_id == r.target_id || r.asleep != 0;
        if r.removed[3] != 0 {
            return 2;
        }
        if min < 0 {
            return if me { 0 } else { 2 };
        }
        if r.class == 2 && hits != 0 && r.guard & 2 != 0 {
            return 3;
        }
        if hits != 0 || d2 != 0 {
            return 0;
        }
        if r.removed[1] != 0 {
            return if me { 0 } else { 7 };
        }
        if r.inflicted[0] != 0 {
            9
        } else if r.removed[0] != 0 {
            0xa
        } else {
            0
        }
    }

    /// `DmgCalc_ResultType(user_id, target_id, class, armored, defense,
    /// magic_defense, damage, inflicted, removed, command, status, asleep,
    /// what, _, nulls)` (`0x0078c270`): [`result_type`]; with nothing
    /// changed, a failed status counts in `nulls[1]`, an immune one in
    /// `nulls[0]`.
    #[allow(clippy::too_many_arguments)]
    pub fn dmg_calc_result_type(
        mem: &mut impl Mem,
        user_id: u32,
        target_id: u32,
        class: i32,
        armored: i32,
        defense: i32,
        magic_defense: i32,
        damage: u32,
        inflicted: u32,
        removed: u32,
        status: u32,
        asleep: i32,
        what: u32,
        nulls: u32,
    ) -> u32 {
        let inflicted_n: [i32; 8] = mem.read(inflicted);
        if inflicted_n[4] == 0 {
            if inflicted_n[6] != 0 {
                add_u32(mem, nulls, 4, 1);
            } else if inflicted_n[5] != 0 {
                add_u32(mem, nulls, 0, 1);
            }
        }
        let r = ResultInputs {
            user_id,
            target_id,
            class,
            armored,
            defense,
            magic_defense,
            damage: mem.read(damage),
            inflicted: inflicted_n,
            removed: mem.read(removed),
            status: status.to_le_bytes()[0],
            asleep,
            what,
            guard: u8_at(mem, ms_get_chr(mem, target_id), 0x447),
        };
        result_type(&r)
    }

    /// `DmgCalc_Bribe(target_id, target, command, counts, info, lang)`
    /// (`0x0078bdd0`): for a bribe command (`+0x1c` bit 31), the gil
    /// offered so far (target `+0x704` plus [`GIL_AMOUNT`]) against the
    /// target's max HP: chance `256·gil / max_hp / 20 − 64` against a byte
    /// of the target's status roll; a bribed target leaves (`+0x701` 2,
    /// `+0x708` the chance, `info` `+0x16` bit 0x100). Not for targets
    /// immune to it (`+0x5b8` bit 0x400) or asleep. In the Korean game
    /// (`lang` 0xb, `TOGetFFXLang`, which the caller supplies) monster 0x10ef
    /// needs 157000 gil first and then counts 18000 more. The offer adds up
    /// (to 999999999). Returns 1 when bribed. (The original faults on a max
    /// HP of 0; this divides by 1.)
    pub fn dmg_calc_bribe<M: Mem>(
        mem: &mut M,
        target_id: u32,
        target: u32,
        command: u32,
        counts: u32,
        info: u32,
        lang: impl FnOnce(&mut M) -> u32,
    ) -> u32 {
        if u32_at(mem, command, 0x1c) & 0x8000_0000 == 0 {
            return 0;
        }
        let gil = GIL_AMOUNT.get(mem);
        let offered = u32_at(mem, target, 0x704);
        let x = offered.wrapping_add(gil.cast_unsigned()) << 8;
        let chance = (x / u32_at(mem, target, 0x594).max(1) / 20)
            .wrapping_sub(0x40)
            .cast_signed();
        let r = (roll(mem, target_id, 2) & 0xff).cast_signed();
        if lang(mem) == 0xb && u16_at(mem, target, 0xe) == 0x10ef {
            if gil.wrapping_add(offered.cast_signed()) < 0x26548 {
                add_u32(mem, counts, 0x18, 1);
                return 0;
            }
            mem.write(target.wrapping_add(0x704), offered.wrapping_add(18000));
        }
        let mut bribed = 0;
        if u16_at(mem, target, 0x5b8) & 0x400 != 0 || u8_at(mem, info, 7) != 0 {
            add_u32(mem, counts, 0x14, 1);
        } else if (r < chance && gil > 0) || DEBUG_ALWAYS_HIT.get(mem) != 0 {
            mem.write(target.wrapping_add(0x701), 2u8);
            mem.write(target.wrapping_add(0x708), chance);
            set_u16(mem, info, 0x16, u16_at(mem, info, 0x16) | 0x100);
            bribed = 1;
            add_u32(mem, counts, 0x10, 1);
        } else {
            add_u32(mem, counts, 0x18, 1);
        }
        let total = u32_at(mem, target, 0x704).cast_signed().wrapping_add(gil);
        let v = ffx_kernel::bridge::ms_check_range(total, 0, 999_999_999);
        mem.write(target.wrapping_add(0x704), v);
        bribed
    }

    /// Debug: no statuses inflicted (`DmgCalc`).
    pub const DEBUG_NO_STATUS: Global<u8> = Global::new(0x0112_a906);
    /// Debug: every hit does 1, 10000 or 100000 (`DmgCalc`).
    pub const DEBUG_DAMAGE_1: Global<u8> = Global::new(0x0112_a90e);
    pub const DEBUG_DAMAGE_10000: Global<u8> = Global::new(0x0112_a90f);
    pub const DEBUG_DAMAGE_100000: Global<u8> = Global::new(0x0112_a910);

    /// `DmgCalc`'s locals that its callees take pointers to, in a
    /// [`Frame`] at [`FRAME`].
    struct Locals {
        damage: u32,
        inflicted: u32,
        removed: u32,
        nulls: u32,
        what: u32,
        what2: u32,
        hit_kind: u32,
        armored: u32,
        delayed: u32,
        temp: u32,
        def: u32,
        mdef: u32,
    }

    impl Locals {
        fn at<M: Mem>(f: &Frame<'_, M>) -> Self {
            Self {
                damage: f.at(0),
                inflicted: f.at(0x10),
                removed: f.at(0x30),
                nulls: f.at(0x50),
                what: f.at(0x58),
                what2: f.at(0x5c),
                hit_kind: f.at(0x60),
                armored: f.at(0x64),
                delayed: f.at(0x68),
                temp: f.at(0x6c),
                def: f.at(0x70),
                mdef: f.at(0x74),
            }
        }
    }

    /// Where [`dmg_calc`] keeps its locals.
    pub const FRAME: u32 = 0xfff0_0000;

    /// Where `DmgCalc`'s caller wants its counts.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct DmgCalcOut {
        /// Three `i32` the damage is taken from, or null.
        pub damage: u32,
        /// Counts of hits and of misses (`i32`).
        pub hits: u32,
        pub misses: u32,
        /// Receives the statuses changed (`inflicted[4]`), or null.
        pub changed: u32,
    }

    /// `DmgCalc(user_id, user, target_id, target, command, command_id,
    /// info, &damage, &hits, &misses, &changed)` (`0x0078e630`): one command
    /// on one target, filling the result `info`: Nul statuses, the hit roll,
    /// the HP damage chain (wake-up, formula, shield/boost, shell, protect,
    /// critical, berserk, magic booster, alchemy, damage rates, gravity
    /// immunity, zombie drains, elements, armor, defend, breaks, immunities,
    /// Wakka's reels), MP and CTB damage, delays, statuses, petrified
    /// targets, stat buffs, Bribe, the reaction, counters; then the damage
    /// capped (9999, or 99999 with the user's `+0x6be` bit 0x800 or the
    /// command's `+0x20` bit 0x80) and taken from the target's pending HP,
    /// MP and CTB (`+0x6e4`) into `+0xd34`; overkill (`+0x5a4`) flags 0x80.
    /// Returns the HP damage. `lang` is `TOGetFFXLang` (for Bribe).
    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::many_single_char_names
    )]
    pub fn dmg_calc<M: Mem>(
        mem: &mut M,
        user_id: u32,
        user: u32,
        target_id: u32,
        target: u32,
        command: u32,
        command_id: u32,
        info: u32,
        out: DmgCalcOut,
        lang: impl FnOnce(&mut M) -> u32,
    ) -> i32 {
        // the locals the callees take pointers to (on the original's stack)
        let mut f = Frame::new(mem, FRAME, 0x80);
        let mem = &mut f;
        let l = Locals::at(mem);
        let threshold = u32_at(mem, target, 0x5a4).cast_signed();
        let mut def = u32::from(u8_at(mem, target, 0x5a9));
        let mut mdef = u32::from(u8_at(mem, target, 0x5ab));
        let weapon = u32_at(mem, command, 0x1c) & 0x40000 != 0;
        let (formula, power, elements, status2) = if weapon {
            let mut s2 = u32::from(u16_at(mem, user, 0x604));
            let c54 = u16_at(mem, command, 0x54);
            if c54 & 0x3e != 0 {
                s2 &= 0xffff_ffc1;
            }
            (
                i32::from(u8_at(mem, user, 0x5c1)),
                i32::from(u8_at(mem, user, 0x5c7)),
                u32::from(u8_at(mem, user, 0x5d9) | u8_at(mem, command, 0x2d)),
                s2 | u32::from(c54),
            )
        } else {
            (
                i32::from(u8_at(mem, command, 0x28)),
                i32::from(u8_at(mem, command, 0x2a)),
                u32::from(u8_at(mem, command, 0x2d)),
                u32::from(u16_at(mem, command, 0x54)),
            )
        };
        let class = i32::from(u16_at(mem, command, 0x20) & 3);
        let vary = 1 - i32::from(DEBUG_NO_VARIANCE.get(mem));
        let mut what = 0u32; // local_5c: what is hit, then the flags
        let mut what2 = 0u32; // local_64
        let mut flags = 0u32; // local_74
        let mut hit_kind = 0u32; // local_6c
        let mut miss_kind = 0u32; // local_8c
        let mut armored = 0u32;
        let mut delayed = 0u32;
        let result; // local_60
        let counter = dmg_calc_check_counter(
            mem,
            0,
            user_id.cast_signed(),
            user,
            target_id.cast_signed(),
            target,
            command,
        );
        let mut apply_counter = false;
        if dmg_calc_check_nuls(mem, elements, info) != 0 {
            what = u32::from(u8_at(mem, command, 0x23));
            mem.write(l.nulls, [1u32, 0]);
            miss_kind = 2;
            result = dmg_calc_result_type(
                mem,
                user_id,
                target_id,
                class,
                0,
                0,
                0,
                l.damage,
                l.inflicted,
                l.removed,
                0,
                i32::from(u8_at(mem, info, 7)),
                what,
                l.nulls,
            );
            apply_counter = counter != 0;
        } else {
            match dmg_calc_check_hit(mem, user, target, command, info, counter) {
                1 => {
                    add_u32(mem, out.misses, 0, 1);
                    miss_kind = 1;
                    result = if user_id == target_id && class != 2 {
                        0x11
                    } else {
                        1
                    };
                    mem.write(l.nulls, [0u32, 1]);
                    apply_counter = true;
                }
                2 => {
                    add_u32(mem, out.misses, 0, 1);
                    result = 0;
                    mem.write(l.nulls, [0u32, 1]);
                }
                _ => {
                    add_u32(mem, out.hits, 0, 1);
                    what = u32::from(u8_at(mem, command, 0x23));
                    what2 = what;
                    mem.write(l.what, what);
                    mem.write(l.what2, what2);
                    if power != 0 {
                        if what & 1 != 0 {
                            dmg_calc_queue_remove_status_on_hit(
                                mem, target, command, l.removed, info,
                            );
                            let status = u32::from(u16_at(mem, info, 0x14));
                            mem.write(l.def, def);
                            mem.write(l.mdef, mdef);
                            let mut d = ms_calc_damage_command(
                                mem, user, target, command, formula, power, status, 1, vary, l.def,
                                l.mdef, 0,
                            );
                            def = mem.read(l.def);
                            mdef = mem.read(l.mdef);
                            d = dmg_calc_shield_boost(mem, target, l.what, d);
                            d = dmg_calc_shell(mem, command, l.what, l.hit_kind, info, d);
                            d = dmg_calc_protect(mem, command, l.what, l.hit_kind, info, d);
                            if DEBUG_NO_CRIT.get(mem) == 0 {
                                d = dmg_calc_crit(mem, user, target, command, l.what, d);
                            }
                            d = dmg_calc_berserk(mem, user, d);
                            mem.write(l.temp, d);
                            dmg_calc_magic_booster(
                                mem,
                                user,
                                command,
                                command_id.cast_signed(),
                                l.temp,
                            );
                            let t: i32 = mem.read(l.temp);
                            d = dmg_calc_alchemy(mem, user, command, command_id, formula, t);
                            d = dmg_calc_stat_boost_ability(mem, user_id, target_id, command, d);
                            d = dmg_calc_grav_immunity(
                                mem, target, formula, l.what2, 1, l.nulls, d,
                            );
                            d = dmg_calc_apply_zombie_drain_interaction(
                                mem, user, command, info, d,
                            );
                            d = dmg_calc_elem(mem, target, elements, d);
                            d = dmg_calc_armored(mem, user, target, command, info, l.armored, d);
                            d = dmg_calc_defend(mem, target, command, l.what, info, d);
                            d = dmg_calc_power_magic_break(mem, user, command, d);
                            d = dmg_calc_dmg_immunity(mem, target, command, l.what2, 1, l.nulls, d);
                            let d0 = dmg_calc_reel_bonus(mem, user, d);
                            mem.write(l.damage, d0);
                            hit_kind = mem.read(l.hit_kind);
                            miss_kind = hit_kind;
                        }
                        what = mem.read(l.what);
                        if what & 2 != 0 {
                            let status = u32::from(u16_at(mem, info, 0x14));
                            let d = ms_calc_damage_command(
                                mem, user, target, command, formula, power, status, 2, vary, 0, 0,
                                0,
                            );
                            mem.write(l.temp, d);
                            dmg_calc_magic_booster(
                                mem,
                                user,
                                command,
                                command_id.cast_signed(),
                                l.temp,
                            );
                            let t: i32 = mem.read(l.temp);
                            let mut d =
                                dmg_calc_alchemy(mem, user, command, command_id, formula, t);
                            let mp = u32_at(mem, target, 0x6e8).cast_signed();
                            if d > 0 && mp < d {
                                d = mp;
                            }
                            let d1 = dmg_calc_apply_zombie_drain_interaction(
                                mem, user, command, info, d,
                            );
                            mem.write(l.damage + 4, d1);
                        }
                        if what & 4 != 0 {
                            let w2: u32 = mem.read(l.what2);
                            mem.write(l.what2, w2 & !4);
                            let status = u32::from(u16_at(mem, info, 0x14));
                            let d = ms_calc_damage_command(
                                mem, user, target, command, formula, power, status, 4, vary, 0, 0,
                                0,
                            );
                            let d2 = dmg_calc_apply_zombie_drain_interaction(
                                mem, user, command, info, d,
                            );
                            mem.write(l.damage + 8, d2);
                        }
                    }
                    let f = dmg_calc_delay(mem, target, command, what, l.damage);
                    flags = dmg_calc_threaten(mem, info, l.what2, 4, f, l.nulls, l.damage);
                    let status_before = u32::from(u16_at(mem, info, 0x14));
                    hit_kind = status_before;
                    if DEBUG_NO_STATUS.get(mem) == 0 {
                        let f = dmg_calc_inflict_status(
                            mem,
                            user_id,
                            user,
                            target_id,
                            target,
                            command,
                            l.inflicted,
                            l.removed,
                            info,
                            flags,
                            l.damage,
                            l.delayed,
                        );
                        flags = dmg_calc_inflict_status_extra(
                            mem,
                            user_id,
                            user,
                            target,
                            command,
                            l.inflicted,
                            l.removed,
                            info,
                            f,
                            status2,
                        );
                        if u8_at(mem, info, 0x14) & 4 == 0 {
                            let v = u8_at(mem, info, 6) | u8_at(mem, command, 0x5a);
                            mem.write(info.wrapping_add(6), v);
                        }
                    }
                    dmg_calc_shatter_fail(mem, info, hit_kind, l.what2, l.inflicted, l.damage);
                    delayed = mem.read(l.delayed);
                    dmg_calc_delay_immunity(
                        mem,
                        target,
                        l.damage,
                        l.inflicted,
                        flags,
                        delayed.cast_signed(),
                    );
                    what = dmg_calc_no_damage_death(mem, info, hit_kind, flags, l.damage);
                    dmg_calc_apply_stat_up_buffs(mem, target, command);
                    dmg_calc_bribe(
                        mem,
                        target_id,
                        target,
                        command,
                        l.inflicted,
                        info,
                        |f: &mut Frame<M>| lang(f.inner()),
                    );
                    what2 = mem.read(l.what2);
                    armored = mem.read(l.armored);
                    result = dmg_calc_result_type(
                        mem,
                        user_id,
                        target_id,
                        class,
                        armored.cast_signed(),
                        def.cast_signed(),
                        mdef.cast_signed(),
                        l.damage,
                        l.inflicted,
                        l.removed,
                        u32::from(u16_at(mem, info, 0x14)),
                        i32::from(u8_at(mem, info, 7)),
                        what,
                        l.nulls,
                    );
                    apply_counter = true;
                }
            }
        }
        let _ = (hit_kind, delayed, armored);
        if apply_counter {
            dmg_calc_check_counter(
                mem,
                1,
                user_id.cast_signed(),
                user,
                target_id.cast_signed(),
                target,
                command,
            );
        }
        let mut damage: [i32; 3] = mem.read(l.damage);
        for (flag, value) in [
            (DEBUG_DAMAGE_1, 1),
            (DEBUG_DAMAGE_10000, 10000),
            (DEBUG_DAMAGE_100000, 100_000),
        ] {
            if flag.get(mem) != 0 {
                if what & 1 != 0 {
                    damage[0] = value;
                }
                if what & 2 != 0 {
                    damage[1] = value;
                }
            }
        }
        let k = u8_at(mem, command, 0x20);
        let cap = if k & 0x80 != 0 {
            99_999
        } else if k & 0x40 != 0 {
            9999
        } else if u16_at(mem, user, 0x6be) & 0x800 != 0 {
            99_999
        } else {
            9999
        };
        if u8_at(mem, user, 0x640) & 8 != 0 && what & 1 != 0 {
            let d = damage[0];
            if d.wrapping_sub(1).cast_unsigned() <= 0x270d {
                damage[0] = 9999;
            } else if d.wrapping_add(0x270e).cast_unsigned() <= 0x270d {
                damage[0] = -9999;
            }
        }
        for (k, &d) in (0u32..).zip(&damage) {
            let v = d.clamp(-cap, cap);
            mem.write(info + 0x20 + 4 * k, v);
            let total = target.wrapping_add(0xd34 + 4 * k);
            let t: i32 = mem.read(total);
            mem.write(total, t.wrapping_add(v));
            let pending = target.wrapping_add(0x6e4 + 4 * k);
            let p: i32 = mem.read(pending);
            let p = p.wrapping_sub(v);
            if out.damage != 0 {
                let at = out.damage + 4 * k;
                let o: i32 = mem.read(at);
                mem.write(at, o.wrapping_sub(v));
            }
            mem.write(pending, p.max(0));
        }
        let dealt: i32 = mem.read(info + 0x20);
        if threshold.wrapping_sub(dealt) <= 0 {
            what |= 0x80;
        }
        mem.write(info, result.to_le_bytes()[0]);
        let counts: [i32; 2] = mem.read(l.nulls);
        let kind = if counts[1] == 0 && counts[0] != 0 {
            2
        } else {
            u32::from(what2 == 0 && counts[1] != 0)
        };
        mem.write(info + 1, kind.to_le_bytes()[0]);
        mem.write(info + 0x18, low16(what));
        mem.write(info + 0x1a, low16(flags));
        mem.write(info + 2, what2.to_le_bytes()[0]);
        mem.write(info + 3, miss_kind.to_le_bytes()[0]);
        let base =
            ms_calc_damage_command(mem, user, target, command, formula, power, 0, 1, 0, 0, 0, 0);
        mem.write(info + 0x1c, base);
        let inflicted: [i32; 8] = mem.read(l.inflicted);
        mem.write(info + 4, inflicted[7].to_le_bytes()[0]);
        dmg_calc_mark_damaged(mem, user, command, info + 0x20, inflicted[4]);
        let removed: [i32; 8] = mem.read(l.removed);
        if removed[3] != 0 {
            mem.write(user.wrapping_add(0xdea), 7u8);
        }
        if out.changed != 0 {
            mem.write(out.changed, inflicted[4]);
        }
        dealt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_weak_first_then_neutral_half_immune_absorb() {
        // fire (1) against: weak, neutral, half, immune, absorb
        assert_eq!(element_damage(1, 0, 0, 0, 1, 100), 150);
        assert_eq!(element_damage(1, 0, 0, 0, 0, 100), 100);
        assert_eq!(element_damage(1, 0, 0, 1, 0, 100), 50);
        assert_eq!(element_damage(1, 0, 1, 0, 0, 100), 0);
        assert_eq!(element_damage(1, 1, 0, 0, 0, 100), -100);
        // fire and ice, ice neutral: unchanged even though fire is absorbed
        assert_eq!(element_damage(3, 1, 0, 0, 0, 100), 100);
    }

    #[test]
    fn physical_formula() {
        let user = FormulaStats {
            strength: 15,
            ..FormulaStats::default()
        };
        let f = Formula {
            formula: 1,
            power: 16,
            variance: 256,
            ..Formula::default()
        };
        // 15³/32 + 30 = 135, no defense: 135 · 730 / 730 · 15 / 15 · 16 / 16
        assert_eq!(
            formula_damage(&user, &FormulaStats::default(), &f),
            (135, false)
        );
    }
}
