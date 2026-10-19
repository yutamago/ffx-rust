//! A party actor from the save (`ffx_battle::save_param`, round 22:
//! `MsSetRamChrParam`) and party swaps (`ffx_battle::save`:
//! `MsExchangeSaveParty` and its `MsGetSavePlyFix` bridge; round 23:
//! `SwapSaveGear`, with the characters wearing the two pieces or near
//! them) against the original, as in
//! `battle.rs`: the shared battle world, random save records, equipment
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

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let aab = ffx_kernel::bridge::A_ABILITY;
    if w.synthetic || !readable(a, aab.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x100, 0x80);
        set(a, b, aab, t);
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
