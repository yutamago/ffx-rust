//! A monster's rewards and gear names (`ffx_battle::reward`,
//! `ffx_battle::gear`, round 22) against the original, as in `battle.rs`:
//! the shared battle world, a reward record of our own and the battle's
//! spoils, on the image alone and on both snapshots. `TOGetFFXLang` is
//! stubbed in both oracles (language 1 or 0xb); everything else runs as
//! the originals and the ports.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTORS, TO_GET_FFX_LANG, World, world};
use battle_common::*;
use ffx_battle::gear::ability as ab;
use ffx_battle::gear::bridge as g;
use ffx_battle::reward::bridge as r;
use ffx_oracle::{Conv, Oracle};

fn game_call(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

/// Ability families the naming rules count: drawing a gear's abilities
/// from one or two of them reaches the rules that need three or four of a
/// kind.
fn families() -> Vec<Vec<u16>> {
    let run = |first: u16, n: u16, step: u16| (0..n).map(|k| first + k * step).collect::<Vec<_>>();
    vec![
        run(ab::FIRESTRIKE, 4, 4),
        run(ab::FIRESTRIKE + 1, 4, 4),
        run(ab::FIRESTRIKE + 2, 4, 4),
        run(ab::FIRESTRIKE + 3, 4, 4),
        run(ab::DEATHSTRIKE, 8, 4),
        run(ab::DEATHSTRIKE + 1, 8, 4),
        run(ab::DEATHSTRIKE + 2, 8, 4),
        run(ab::DEATHSTRIKE + 3, 8, 4),
        vec![
            ab::CONFUSEPROOF,
            ab::BERSERKPROOF,
            ab::CURSEPROOF,
            ab::CURSEPROOF + 1,
        ],
        run(ab::AUTO_SHELL, 5, 1),
        run(ab::SOS_SHELL, 5, 1),
        run(ab::SOS_NULTIDE, 4, 1),
        run(ab::STRENGTH_3, 4, 1),
        run(ab::MAGIC_3, 4, 1),
        run(ab::DEFENSE_3, 4, 1),
        run(ab::MAGIC_DEF_3, 4, 1),
        run(ab::HP_5, 4, 1),
        run(ab::MP_5, 4, 1),
        vec![
            ab::AUTO_POTION,
            ab::AUTO_MED,
            ab::AUTO_PHOENIX,
            ab::MAGIC_BOOSTER,
        ],
        vec![
            ab::DOUBLE_OVERDRIVE,
            ab::TRIPLE_OVERDRIVE,
            ab::SOS_OVERDRIVE,
            ab::OVERDRIVE_TO_AP,
            ab::DOUBLE_AP,
            ab::TRIPLE_AP,
        ],
        vec![
            ab::COUNTERATTACK,
            ab::EVADE_COUNTER,
            ab::MAGIC_COUNTER,
            ab::MAGIC_BOOSTER,
            ab::ALCHEMY,
        ],
        vec![
            ab::BREAK_HP_LIMIT,
            ab::BREAK_MP_LIMIT,
            ab::RIBBON,
            ab::HP_STROLL,
            ab::MP_STROLL,
            ab::PICKPOCKET,
            ab::MASTER_THIEF,
        ],
        run(ab::DISTILL_POWER, 4, 1),
        vec![ab::SENSOR, ab::FIRST_STRIKE, ab::INITIATIVE, ab::PIERCING],
    ]
}

/// A family's members in a random order.
fn shuffled(rng: &mut Xorshift, families: &[Vec<u16>]) -> Vec<u16> {
    let mut f = families[rng.below(u32::try_from(families.len()).unwrap()) as usize].clone();
    for i in (1..f.len()).rev() {
        f.swap(i, rng.below(u32::try_from(i + 1).unwrap()) as usize);
    }
    f
}

/// A gear's four abilities: four of one family or two, three or four
/// different ones of one and the rest of another (the rules that count
/// several of a kind), else empty slots, one or two families, or any.
fn abilities(rng: &mut Xorshift, families: &[Vec<u16>]) -> [u16; 4] {
    if rng.below(3) == 0 {
        let (f, g) = (shuffled(rng, families), shuffled(rng, families));
        let k = (2 + rng.below(3) as usize).min(f.len());
        let mut v: Vec<u16> = f[..k].iter().chain(g.iter()).copied().collect();
        v.resize(4, 0xff);
        return [v[0], v[1], v[2], v[3]];
    }
    let mut pool: Vec<u16> = Vec::new();
    for _ in 0..=rng.below(2) {
        pool.extend(&families[rng.below(u32::try_from(families.len()).unwrap()) as usize]);
    }
    std::array::from_fn(|_| match rng.below(10) {
        0..=2 => 0xff,
        3..=5 => u16::try_from(0x8000 + rng.below(0x81)).unwrap(),
        6 => rng.word().to_le_bytes()[0].into(),
        _ => pool[rng.below(u32::try_from(pool.len()).unwrap()) as usize],
    })
}

/// The gear names on random gear (`gear`, 0x16 bytes).
fn names(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, gear: u32, families: &[Vec<u16>]) {
    fill(a, b, rng, gear, 0x16);
    let flags = [0u8, 0, 0, 4, 8, 0xc, rng.byte()][rng.below(7) as usize] & !3 | rng.byte() & 3;
    poke(a, b, gear + 3, &[flags]);
    poke(
        a,
        b,
        gear + 5,
        &[[0, 0, 1, rng.byte()][rng.below(4) as usize]],
    );
    poke(a, b, gear + 0xb, &[u8::try_from(rng.below(6)).unwrap()]);
    let ids: Vec<u8> = abilities(rng, families)
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    poke(a, b, gear + 0xe, &ids);
    twin("MsWeaponNameNum", a, b, 0x007a_0cf0, &[gear], ALL, |m| {
        g::ms_weapon_name_num(m, gear)
    });
    twin(
        "MsWeaponNameNum_Weapon",
        a,
        b,
        0x007a_1c00,
        &[gear],
        ALL,
        |m| g::ms_weapon_name_num_weapon(m, gear),
    );
    twin(
        "MsWeaponNameNum_Armor",
        a,
        b,
        0x007a_0d10,
        &[gear],
        ALL,
        |m| g::ms_weapon_name_num_armor(m, gear),
    );
    let id = match rng.below(3) {
        0 => u32::from(u16::from_le_bytes([ids[2 * (rng.below(4) as usize)], 0x80])),
        1 => 0x8000 + rng.below(0x81) + (rng.next() << 16),
        _ => rng.next(),
    };
    twin(
        "MsGearHasAbility",
        a,
        b,
        0x007a_0c20,
        &[gear, id],
        ALL,
        |m| g::ms_gear_has_ability(m, gear, id),
    );
}

/// The reward record at `rec` (0x200 bytes): random, with chances of
/// none, any and all, drops, the bribe's item and factor, the gear's
/// bytes and the characters' ability lists from the ability pool (0 now
/// and then).
fn reward_record(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32) {
    fill(a, b, rng, rec, 0x200);
    for o in [8, 9, 0xb] {
        poke(
            a,
            b,
            rec + o,
            &[[0, 0xff, 0x80, rng.byte()][rng.below(4) as usize]],
        );
    }
    // drops (normal, rare per slot) at +0xc and, with overkill, +0x18;
    // their counts at +0x14 and +0x20
    for (items, counts) in [(0xc, 0x14), (0x18, 0x20)] {
        for k in 0..4 {
            let item = [
                0x2065u16,
                0x2065,
                0x2001,
                rng.word().to_le_bytes()[0].into(),
            ][rng.below(4) as usize]
                | 0x2000;
            poke(a, b, rec + items + 2 * k, &item.to_le_bytes());
            poke(
                a,
                b,
                rec + counts + k,
                &[u8::try_from(rng.below(5)).unwrap()],
            );
        }
    }
    let bribe = [0x2065u16, 0x2048, 0x2010][rng.below(3) as usize];
    poke(a, b, rec + 0x2a, &bribe.to_le_bytes());
    for o in [0x2c, 0x2d, 0x31] {
        poke(
            a,
            b,
            rec + o,
            &[[0, 1, 9, 30, rng.byte()][rng.below(5) as usize]],
        );
    }
    for k in 0..14 * 8 {
        let id = match rng.below(8) {
            0 => 0,
            // other high bits (the group lookup keeps the low 12)
            1 => u16::try_from(0x9000 + rng.below(0x81)).unwrap(),
            2 => rng.word().to_le_bytes()[0].into(),
            _ => u16::try_from(0x8000 + rng.below(0x81)).unwrap(),
        };
        poke(a, b, rec + 0x32 + 2 * k, &id.to_le_bytes());
    }
}

/// Bribe factors and gil whose count depends on the x87 rounding the
/// square root or the sum to `f32` (found by search: the count differs
/// from one kept as a double, for one of the scales 20-30).
const BRIBE_EDGES: [(u8, i32); 17] = [
    (19, 3251),
    (23, 3666),
    (27, 3666),
    (7, 33_238),
    (9, 32_994),
    (4, 74_571),
    (5, 100_186),
    (2, 298_284),
    (3, 296_946),
    (3, 88_191),
    (2, 142_071),
    (3, 163_975),
    (3, 213_356),
    (2, 246_310),
    (2, 281_716),
    (1, 351_166),
    (1, 395_102),
];

/// `MsGetItem` and its parts for a monster killed by `chr`.
fn rewards(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, rec: u32) {
    let mut call = game_call;
    reward_record(a, b, rng, rec);
    let spoils = b.rebase(r::SPOILS);
    fill(a, b, rng, spoils, 0x1b0);
    poke(a, b, spoils + 0xd0, &[u8::try_from(rng.below(10)).unwrap()]);
    let gears = [0u8, 0, 3, 7, 8, 0xff, 0x80][rng.below(7) as usize];
    poke(a, b, spoils + 0xd2, &[gears]);
    // who joined (ply_save +0x2c bit 4): at least one of the seven, as the
    // gear's owner draw divides by their count
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    for i in 0..7 {
        let v = rng.byte() | if i == 0 && rng.below(2) == 0 { 0x10 } else { 0 };
        poke(
            a,
            b,
            ply + i * ffx_battle::save_param::bridge::PLY_SAVE_SIZE + 0x2c,
            &[v],
        );
    }
    let any = (0..7).any(|i| {
        let at = ply + i * ffx_battle::save_param::bridge::PLY_SAVE_SIZE + 0x2c;
        a.read(at, 1).unwrap()[0] & 0x10 != 0
    });
    if !any {
        poke(a, b, ply + 0x2c, &[0x10]);
    }
    for gl in [
        r::DEBUG_ALWAYS_HIT,
        r::DEBUG_RARE_STEAL,
        r::DEBUG_AP_X100,
        r::DEBUG_GIL_X100,
    ] {
        set(a, b, gl, u8::from(rng.below(6) == 0));
    }
    set(
        a,
        b,
        r::GIL_WON,
        [0, 999_999_990, rng.next()][rng.below(3) as usize],
    );
    if rng.below(4) == 0 {
        set(a, b, ffx_bridge::x87::SSE2_AVAILABLE, rng.below(2));
    }
    let lang = [1, 0xb][rng.below(2) as usize];
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(TO_GET_FFX_LANG);
        o.stub_return(at, lang).unwrap();
    }
    let victim = w.actor(rng.below(ACTORS));
    poke(
        a,
        b,
        victim + 0x701,
        &[[2, 2, 0, rng.byte()][rng.below(4) as usize]],
    );
    let paid = [0, 1, 1000, 123_456, -5, rng.next().cast_signed()][rng.below(6) as usize];
    poke(a, b, victim + 0x708, &paid.to_le_bytes());
    // a bribe's factor and gil where the f32 roundings of the square root
    // or the sum decide the count (for one of the 11 scales each)
    if rng.below(2) == 0 {
        let (factor, gil) = BRIBE_EDGES[rng.below(17) as usize];
        poke(a, b, rec + 0x2c, &[factor]);
        poke(a, b, victim + 0x708, &gil.to_le_bytes());
    }
    let chr = [0, 3, 6, 7, 8, 0x13, rng.below(0x20)][rng.below(7) as usize];
    let overkill = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
    let flags = rng.below(0x40) * [1, 1, 0x101][rng.below(3) as usize];
    let reward = if rng.below(10) == 0 { 0 } else { rec };
    twin(
        "MsGetItem",
        a,
        b,
        0x0079_90d0,
        &[chr, victim, reward, overkill, flags],
        0,
        |m| {
            r::ms_get_item(m, &mut call, chr, victim, reward, overkill, flags);
            0
        },
    );
    same("MsGetItem", a, b);
    twin(
        "BattleRewards_AddGear",
        a,
        b,
        0x0079_8c10,
        &[chr, rec, spoils],
        ALL,
        |m| r::battle_rewards_add_gear(m, chr, rec, spoils),
    );
    same("BattleRewards_AddGear", a, b);
    twin(
        "BattleRewards_NewGear",
        a,
        b,
        0x0079_8bd0,
        &[spoils],
        ALL,
        |m| r::new_gear(m, spoils).cast_unsigned(),
    );
    same("BattleRewards_NewGear", a, b);
    let id = [0, 0x8000 + rng.below(0x81), rng.below(0x81), rng.word()][rng.below(4) as usize];
    twin("MsGetAbilityGroup", a, b, 0x0079_8a90, &[id], ALL, |m| {
        r::ability_group(m, id)
    });
}

/// `msFieldItemGet` (round 22) for treasure `i` into `spoils` (0x200
/// bytes): the treasure table's records (kind, amount, ID) and the
/// equipment table's, fake ones redrawn on the image; the gear count
/// sometimes full; the language 1 or 0xb.
fn field_items(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool, spoils: u32) {
    let mut call = game_call;
    if synthetic {
        let t = r::FIELD_ITEMS.get(b);
        for k in 0..0x40 {
            let kind = [0u8, 2, 5, 10, rng.byte()][rng.below(5) as usize];
            let id = match kind {
                5 => u16::try_from(rng.below(0x40)).unwrap(),
                _ => [
                    0x2065u16,
                    0x2010,
                    0xa000,
                    rng.word().to_le_bytes()[0].into(),
                ][rng.below(4) as usize],
            };
            let rec = [[kind, rng.byte()], id.to_le_bytes()].concat();
            poke(a, b, t + 0x20 + 4 * k, &rec);
        }
        let e = r::FIELD_EQUIPMENT.get(b);
        for k in 0..0x40 {
            fill(a, b, rng, e + 0x20 + 0x10 * k, 8);
            poke(
                a,
                b,
                e + 0x20 + 0x10 * k + 7,
                &[u8::try_from(rng.below(6)).unwrap()],
            );
            for slot in 0..4 {
                let ability = match rng.below(4) {
                    0 => 0,
                    1 => 0xff,
                    _ => u16::try_from(0x8000 + rng.below(0x81)).unwrap(),
                };
                poke(a, b, e + 0x28 + 0x10 * k + 2 * slot, &ability.to_le_bytes());
            }
        }
    }
    fill(a, b, rng, spoils, 0x200);
    let gears = [0u8, 3, 7, 8, 0x80][rng.below(5) as usize];
    poke(a, b, spoils + 0xd2, &[gears]);
    poke(a, b, spoils + 0xd0, &[u8::try_from(rng.below(10)).unwrap()]);
    let lang = [1, 0xb][rng.below(2) as usize];
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(TO_GET_FFX_LANG);
        o.stub_return(at, lang).unwrap();
    }
    let i = rng.below(0x40);
    twin("msFieldItemGet", a, b, 0x0079_9420, &[i, spoils], 0, |m| {
        r::ms_field_item_get(m, &mut call, i, spoils);
        0
    });
    same("msFieldItemGet", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    // the auto-abilities (groups from a few, so gear meets the same group)
    // and the weapon names, fake on the image alone
    let aab = ffx_kernel::bridge::A_ABILITY;
    if w.synthetic || !readable(a, aab.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x100, 0x80);
        for k in 0..0x100 {
            poke(
                a,
                b,
                t + 0x20 + 0x80 * k + 0x69,
                &[u8::try_from(rng.below(6)).unwrap()],
            );
        }
        set(a, b, aab, t);
    }
    let names_table = ffx_battle::menu::bridge::WEAPON_NAMES;
    if w.synthetic || !readable(a, names_table.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x100, 0x50);
        set(a, b, names_table, t);
    }
    // the field's treasure and equipment tables, fake on the image
    for (t, stride) in [(r::FIELD_ITEMS, 4), (r::FIELD_EQUIPMENT, 0x10)] {
        if w.synthetic || !readable(a, t.get(b), 0x20) {
            let f = battle_common::world::fake_records(a, b, rng, 0x40, stride);
            set(a, b, t, f);
        }
    }
    let rec = scratch(a, b, 0x200);
    let gear = scratch(a, b, 0x20);
    let spoils = scratch(a, b, 0x200);
    let families = families();
    for _ in 0..30_000 {
        names(a, b, rng, gear, &families);
    }
    same("names", a, b);
    for round in 0..1500 {
        w.shuffle(a, b, rng, round);
        rewards(a, b, rng, &w, rec);
        field_items(a, b, rng, w.synthetic, spoils);
    }
}

#[test]
fn rewards_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x22a1_0001), true);
}

#[test]
fn rewards_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x22a1_0002), false);
}
