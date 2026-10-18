//! The battle menu's window lists (`ffx_battle::menu`, round 22:
//! `MsGetSaveWindow`, `MsGetChrStatusBits_0078f930`) against the original,
//! as in `battle.rs`: the shared battle world with the fields the lists
//! test drawn at random (often 0), random party and reserve slots, on the
//! image alone and on both snapshots.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTOR, ACTORS, World, world};
use battle_common::*;
use ffx_battle::menu::bridge as m;
use ffx_oracle::Oracle;

/// The actors' fields the lists read: in (`+0x10`), alive and able to act
/// (`+0xdc8`, `+0xdca`), out (`+0xdcd`), able to switch (`+0xdd8`), aeon
/// and sister bits (`+0x590`), status bits (`+0x606`); the party's and the
/// reserve's slots; sometimes every monster living.
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        for o in [0x10, 0xdc8, 0xdca, 0xdcd, 0xdd8] {
            let v = if rng.below(2) == 0 { 0 } else { rng.byte() | 1 };
            poke(a, b, p + o, &[v]);
        }
        let bits = [0u8, 4, 0xc, 8, rng.byte()][rng.below(5) as usize];
        poke(a, b, p + 0x590, &[bits]);
        fill(a, b, rng, p + 0x606, 2);
    }
    // the spare actor MsGetChr hands out for index 0xff (an empty slot's)
    let spare = ffx_battle::actor::bridge::CHR_SPARE.ptr(b).addr;
    for o in [0x10, 0xdc8, 0xdca, 0xdcd, 0xdd8] {
        let v = if rng.below(2) == 0 { 0 } else { rng.byte() | 1 };
        poke(a, b, spare + o, &[v]);
    }
    // now and then all eight monsters living (a full monster list)
    if rng.below(6) == 0 {
        for i in 0..8 {
            let p = ffx_battle::actor::bridge::ms_get_mon(b, i);
            poke(a, b, p + 0x10, &[1]);
            poke(a, b, p + 0xdc8, &[1]);
            poke(a, b, p + 0xdcd, &[0]);
        }
    }
    let slot = |rng: &mut Xorshift| -> u8 {
        match rng.below(4) {
            0 => 0xff,
            1 => u8::try_from(rng.below(ACTORS)).unwrap(),
            _ => u8::try_from(rng.below(0x12)).unwrap(),
        }
    };
    let party: Vec<u8> = (0..7).map(|_| slot(rng)).collect();
    poke(a, b, m::PARTY.ptr(b).addr, &party);
    let reserve: Vec<u8> = (0..17).map(|_| slot(rng)).collect();
    poke(
        a,
        b,
        ffx_battle::ability::bridge::PARTY_EXTRA.ptr(b).addr,
        &reserve,
    );
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    if w.synthetic || !readable(a, ffx_battle::actor::bridge::MON_TOP.get(b), ACTOR as usize) {
        set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    }
    let lists = ffx_battle::misc::bridge::ITEM_MEM2;
    if w.synthetic || !readable(a, lists.get(b), 0x478) {
        let t = scratch(a, b, 0x478 * 0x20);
        set(a, b, lists, t);
    }
    let count = w.out;
    for _ in 0..4000 {
        dress(a, b, rng, &w);
        // the window lists: their 0xff tails cover a stale list's
        for at in [m::SUMMON_LIST, m::MONSTER_LIST] {
            fill(a, b, rng, b.rebase(at), 0x100);
        }
        let chr = [rng.below(0x20), rng.below(0x100), rng.next()][rng.below(3) as usize];
        let kind = [rng.below(0x18), rng.below(0x18), rng.next()][rng.below(3) as usize];
        let out = if rng.below(5) == 0 { 0 } else { count };
        twin(
            "MsGetSaveWindow",
            a,
            b,
            0x0078_5410,
            &[chr, kind, out],
            ALL,
            |o| m::ms_get_save_window(o, chr, kind, out),
        );
        same("MsGetSaveWindow", a, b);
        // an actor's low byte (the original reads whatever actor it gets)
        let chr = rng.below(ACTORS) | rng.next() << 8;
        twin("MsGetChrStatusBits", a, b, 0x0078_f930, &[chr], ALL, |o| {
            m::ms_get_chr_status_bits(o, chr)
        });
    }
}

#[test]
fn windows_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x22b1_0001), true);
}

#[test]
fn windows_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x22b1_0002), false);
}
