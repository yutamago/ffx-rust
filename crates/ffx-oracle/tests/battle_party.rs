//! A party actor from the save (`ffx_battle::save_param`, round 22:
//! `MsSetRamChrParam`) and party swaps (`ffx_battle::save`:
//! `MsExchangeSaveParty` and its `MsGetSavePlyFix` bridge; round 23:
//! `SwapSaveGear`, with the characters wearing the two pieces or near
//! them; round 34: `MsSetRamChr`, party and monster actors) against the
//! original, as in `battle.rs`: the shared battle world, random save records, equipment
//! (pieces with no, empty and real auto-abilities) and party slots, on the
//! image alone and on both snapshots.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::{World, world};
use battle_common::*;
use ffx_battle::save::bridge::{EQUIPMENT, PLY_SAVE};
use ffx_battle::save_param::bridge as sp;
use ffx_oracle::Oracle;

/// Character `chr`'s save record, its two pieces of equipment and the
/// actor's fields the port reads.
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, chr: u32) {
    let save = PLY_SAVE.ptr(b).addr + chr * sp::PLY_SAVE_SIZE;
    fill(a, b, rng, save, sp::PLY_SAVE_SIZE);
    let pick = |rng: &mut Xorshift| -> u8 {
        match rng.below(5) {
            0 => 0xff,
            1 => 200 + rng.byte() % 56,
            _ => u8::try_from(rng.below(200)).unwrap(),
        }
    };
    let pieces = [pick(rng), pick(rng)];
    poke(a, b, save + 0x2d, &pieces);
    let equipment = EQUIPMENT.ptr(b).addr;
    for id in pieces {
        let slot = if id < 200 { u32::from(id) } else { 0 };
        let at = equipment + slot * 0x16;
        fill(a, b, rng, at, 0x16);
        for k in 0..4 {
            let ability = match rng.below(6) {
                0 => 0,
                1 => 0xff,
                2 => rng.word().to_le_bytes()[0].into(),
                _ => u16::try_from(0x8000 + rng.below(0x81)).unwrap(),
            };
            poke(a, b, at + 0xe + 2 * k, &ability.to_le_bytes());
        }
    }
    let actor = w.actor(chr);
    for (o, n) in [(0x590, 0x140), (0x62a, 0x40), (0x6bc, 8)] {
        fill(a, b, rng, actor + o, n);
    }
    for o in [0x5d0, 0x5d4] {
        poke(a, b, actor + o, &rng.range(-100, 100_000).to_le_bytes());
    }
}

/// A party swap: the 20 party slots (characters 0-9 and 0xff), the first
/// ten characters' join and fixed bits, the character and slot at random.
fn exchange(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_battle::save::bridge as sv;
    let party = sv::IN_PARTY.ptr(b).addr;
    let slots: Vec<u8> = (0..0x14)
        .map(|_| match rng.below(4) {
            0 | 1 => 0xff,
            _ => u8::try_from(rng.below(10)).unwrap(),
        })
        .collect();
    poke(a, b, party, &slots);
    for chr in 0..10 {
        let flags = u8::try_from(rng.below(8)).unwrap() | (rng.byte() & 0xf8);
        poke(
            a,
            b,
            PLY_SAVE.ptr(b).addr + chr * sp::PLY_SAVE_SIZE + 0x2c,
            &[flags],
        );
    }
    let chr = [0xffff_ffff, 0xff, 0x105, rng.below(10), rng.below(10)][rng.below(5) as usize];
    let slot = [-1, rng.range(0, 0x14), rng.range(0, 3), rng.range(0, 0x14)][rng.below(4) as usize];
    twin(
        "MsExchangeSaveParty",
        a,
        b,
        0x0078_4d00,
        &[chr, slot.cast_unsigned()],
        ALL,
        |m| sv::ms_exchange_save_party(m, chr, slot),
    );
    same("MsExchangeSaveParty", a, b);
    let c = rng.below(10) | rng.next() << 8;
    twin("MsGetSavePlyFix", a, b, 0x0078_5360, &[c], ALL, |m| {
        sv::ms_get_save_ply_fix(m, c)
    });
}

/// An equipment ID: the inventory's (in range or not), the 0x7000 and
/// 0xb000 slots', 0xff, or anything.
fn gear_id(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => 0x7000 | rng.below(10),
        1 => 0xb000 | rng.below(10),
        2 => 0xff,
        3 => rng.next(),
        4 => 0x5000 | rng.below(0x1000),
        _ => [0x5000, 0, 0x2000, 0x7_5000][rng.below(4) as usize] | rng.below(8),
    }
}

/// `SwapSaveGear`: two IDs, the characters' weapons and armor often one of
/// their low bytes.
fn swap(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_battle::save::bridge as sv;
    let first = gear_id(rng);
    let second = if rng.below(8) == 0 {
        first
    } else {
        gear_id(rng)
    };
    for chr in 0..0x12 {
        let at = PLY_SAVE.ptr(b).addr + chr * sp::PLY_SAVE_SIZE + 0x2d;
        let piece = |rng: &mut Xorshift| -> u8 {
            match rng.below(4) {
                0 => first.to_le_bytes()[0],
                1 => second.to_le_bytes()[0],
                2 => 0xff,
                _ => u8::try_from(rng.below(10)).unwrap(),
            }
        };
        let pieces = [piece(rng), piece(rng)];
        poke(a, b, at, &pieces);
    }
    for id in [first, second] {
        if id != 0xff {
            let slot = sv::ms_get_save_weapon(b, id, 0);
            fill(a, b, rng, slot, 0x16);
        }
    }
    twin(
        "SwapSaveGear",
        a,
        b,
        0x007a_b9f0,
        &[first, second],
        ALL,
        |m| sv::swap_save_gear(m, first, second).cast_unsigned(),
    );
    same("SwapSaveGear", a, b);
}

/// Where `MsSetRamChr`'s monster reads go: a monster record (0x80 bytes)
/// and monster data whose `+0x14` points at its 0x46 words of item memory.
struct MonsterData {
    record: u32,
    data: u32,
}

fn monster_data(a: &mut Oracle, b: &mut Oracle) -> MonsterData {
    MonsterData {
        record: scratch(a, b, 0x80),
        data: scratch(a, b, 0x200),
    }
}

/// `MsSetRamChr(_, chr)`: a party character (its save record dressed) or
/// a monster (its record random, its data there or null, its 16 commands
/// often zero), under water or not, the steal rule and the two debug HP
/// toggles at random.
fn ram_chr(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, m: &MonsterData) {
    let monster = rng.below(2) == 0;
    let chr = if monster {
        0x14 + rng.below(8)
    } else {
        rng.below(0x12)
    };
    let actor = w.actor(chr);
    if monster {
        let real: u32 = ffx_bridge::Mem::read(b, actor + 0xf80);
        if w.synthetic || !readable(a, real, 0x80) || rng.below(3) != 0 {
            fill(a, b, rng, m.record, 0x80);
            for k in 0..16 {
                if rng.below(3) == 0 {
                    poke(a, b, m.record + 0x50 + 2 * k, &[0, 0]);
                }
            }
            poke(a, b, actor + 0xf80, &m.record.to_le_bytes());
        }
        let data = match rng.below(3) {
            0 => 0,
            _ => {
                fill(a, b, rng, m.data, 0x200);
                let at = 0x20 + 4 * rng.below(0x20);
                poke(a, b, m.data + 0x14, &at.to_le_bytes());
                m.data
            }
        };
        let real: u32 = ffx_bridge::Mem::read(b, actor + 0x48);
        let usable = real == 0 || readable(a, real, 0x18);
        if w.synthetic || !usable || rng.below(2) == 0 {
            poke(a, b, actor + 0x48, &data.to_le_bytes());
        }
        for (o, n) in [(0x50, 0x28), (0x641, 0x19)] {
            fill(a, b, rng, actor + o, n);
        }
    } else {
        dress(a, b, rng, w, chr);
    }
    let flag = |rng: &mut Xorshift| -> u8 {
        match rng.below(4) {
            0 => 1,
            1 => rng.byte(),
            _ => 0,
        }
    };
    let v = [flag(rng), flag(rng)];
    poke(a, b, sp::DEBUG_PARTY_HP1.ptr(b).addr, &v);
    let v = [flag(rng)];
    poke(a, b, ffx_battle::state::bridge::UNDERWATER.ptr(b).addr, &v);
    let v = [[0, 1, 2, 2][rng.below(4) as usize]];
    poke(a, b, ffx_battle::save::bridge::STEAL_RULE.ptr(b).addr, &v);
    for o in [0x3b, 0x606, 0x607] {
        let v = [flag(rng)];
        poke(a, b, actor + o, &v);
    }
    // the cleared bytes and the one after (+0x72c)
    if rng.below(4) == 0 {
        fill(a, b, rng, actor + 0x540, 0x1ed);
    }
    let junk = rng.next();
    twin("MsSetRamChr", a, b, 0x0079_b4f0, &[junk, chr], 0, |m| {
        sp::ms_set_ram_chr(m, chr);
        0
    });
    same("MsSetRamChr", a, b);
}

/// The tables `MsSetRamChr` reads: the characters' kernel records and the
/// item memories, fake where there are none.
fn ram_chr_tables(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    use ffx_battle::misc::bridge::{ITEM_MEM, ITEM_MEM2};
    let rom = ffx_battle::save::bridge::KERNEL_PLY_ROM;
    if w.synthetic || !readable(a, rom.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x14, 0x30);
        set(a, b, rom, t);
    }
    let n = 0x1f;
    if w.synthetic || !readable(a, ITEM_MEM.get(b), (n * sp::ITEM_MEM_SIZE) as usize) {
        let t = scratch(a, b, (n * sp::ITEM_MEM_SIZE) as usize);
        set(a, b, ITEM_MEM, t);
    }
    if w.synthetic || !readable(a, ITEM_MEM2.get(b), (n * sp::ITEM_MEM2_SIZE) as usize) {
        let t = scratch(a, b, (n * sp::ITEM_MEM2_SIZE) as usize);
        set(a, b, ITEM_MEM2, t);
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let aab = ffx_kernel::bridge::A_ABILITY;
    if w.synthetic || !readable(a, aab.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x100, 0x80);
        set(a, b, aab, t);
    }
    ram_chr_tables(a, b, rng, &w);
    let m = monster_data(a, b);
    for _ in 0..1500 {
        ram_chr(a, b, rng, &w, &m);
    }
    for _ in 0..3000 {
        let chr = rng.below(0x12);
        dress(a, b, rng, &w, chr);
        twin("MsSetRamChrParam", a, b, 0x0079_c5f0, &[chr], ALL, |m| {
            sp::ms_set_ram_chr_param(m, chr).cast_unsigned()
        });
        same("MsSetRamChrParam", a, b);
        exchange(a, b, rng);
        swap(a, b, rng);
    }
}

#[test]
fn party_actors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x22c1_0001), true);
}

#[test]
fn party_actors_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x22c1_0002), false);
}
