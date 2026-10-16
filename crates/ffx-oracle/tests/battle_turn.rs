//! The turn bookkeeping (`ffx_battle::turn`) against the original, as in
//! `battle.rs`: the shared battle world with random actor fields, on the
//! image alone and on both snapshots.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTOR, ACTORS, World, world};
use battle_common::*;
use ffx_battle::turn::bridge as t;
use ffx_oracle::Oracle;

fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        for (at, len) in [
            (0x10, 1),
            (0x594, 8),
            (0x5d0, 8),
            (0x5ac, 1),
            (0x606, 0x14),
            (0x613, 2),
            (0x65c, 2),
            (0x6bc, 4),
            (0xdc8, 8),
            (0x640, 1),
            (0x6be, 2),
        ] {
            fill(a, b, rng, p + at, len);
        }
        for o in [0x774, 0xa4c] {
            let head = [
                u8::try_from(rng.below(5)).unwrap(),
                u8::try_from(rng.below(5)).unwrap(),
                u8::try_from(rng.below(ACTORS + 1)).unwrap(),
                u8::try_from(rng.below(4)).unwrap(),
            ];
            poke(a, b, p + o, &head);
            fill(a, b, rng, p + o + 4, 0x2c * 5);
        }
    }
    // HP and maxima sometimes small and negative
    for i in 0..ACTORS {
        let p = w.actor(i);
        if rng.below(2) == 0 {
            for at in [0x594, 0x598, 0x5d0, 0x5d4] {
                poke(a, b, p + at, &rng.range(-50, 200).to_le_bytes());
            }
        }
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let mode = rng.range(0, 4);
    twin(
        "MsSetFirstCTB",
        a,
        b,
        0x0078_ded0,
        &[mode.cast_unsigned()],
        ALL,
        |m| t::ms_set_first_ctb(m, mode).cast_unsigned(),
    );
    let chr = rng.below(ACTORS);
    let cmd = w.command;
    fill(a, b, rng, cmd, 0x60);
    poke(a, b, cmd + 3, &[u8::try_from(rng.below(5)).unwrap()]);
    for k in 0..4 {
        let ids: [u16; 2] = std::array::from_fn(|_| match rng.below(6) {
            0 => 0x311e,
            1 => 0xff,
            2 | 3 => u16::try_from(0x2000 + rng.below(0x40)).unwrap(),
            _ => u16::try_from(0x3000 + rng.below(0x40)).unwrap(),
        });
        poke(
            a,
            b,
            cmd + 8 + 0x10 * k,
            &[ids[0].to_le_bytes(), ids[1].to_le_bytes()].concat(),
        );
    }
    let n = rng.range(-2, 3);
    twin(
        "MsUseItem",
        a,
        b,
        0x007b_0c20,
        &[chr, cmd, n.cast_unsigned()],
        ALL,
        |m| t::ms_use_item(m, chr, cmd, n).cast_unsigned(),
    );
    twin("MsCheckStatus", a, b, 0x0079_a0e0, &[], 0, |m| {
        t::ms_check_status(m);
        0
    });
    let (c, tg) = (rng.below(ACTORS + 2), rng.below(ACTORS));
    let out: [u32; 6] = std::array::from_fn(|k| {
        if rng.below(5) == 0 {
            0
        } else {
            w.out + 4 * u32::try_from(k).unwrap()
        }
    });
    let mut args = vec![c, tg];
    args.extend(out);
    twin("MsGetDamageStatus", a, b, 0x0079_aa50, &args, ALL, |m| {
        t::ms_get_damage_status(m, c, tg, out)
    });
    let (who, actor, ap, ret) = (
        rng.below(18),
        w.actor(rng.below(ACTORS)),
        rng.word(),
        rng.next(),
    );
    twin(
        "MsApUp",
        a,
        b,
        0x0079_8a00,
        &[who, actor, ap, ret],
        ALL,
        |m| t::ms_ap_up(m, who, actor, ap, ret),
    );
    // the overdrive gauge (MsLimitUp): a maximum of at least 1 (the AP
    // path divides by it) and rates that are ordinary floats
    let actor = w.actor(rng.below(ACTORS));
    poke(
        a,
        b,
        actor + 0x5bc,
        &[rng.byte(), u8::try_from(1 + rng.below(255)).unwrap()],
    );
    let rate = (f32::from(i16::try_from(rng.range(1, 3000)).unwrap()) / 100.0).to_bits();
    poke(a, b, actor + 0x70c, &rate.to_le_bytes());
    let (who, n) = (rng.below(18), rng.range(-10, 300));
    twin(
        "MsLimitUp",
        a,
        b,
        0x007b_1590,
        &[who, actor, n.cast_unsigned()],
        ALL,
        |m| t::ms_limit_up(m, who, actor, n).cast_unsigned(),
    );
    let spoils = w.out + 0x20;
    let item = if rng.below(6) == 0 {
        0
    } else {
        0x2000 + rng.below(12)
    };
    let n = rng.range(-1, 120);
    twin(
        "BattleRewards_AddItem",
        a,
        b,
        0x0079_8ac0,
        &[item, n.cast_unsigned(), spoils],
        0,
        |m| {
            t::battle_rewards_add_item(m, item, n, spoils);
            0
        },
    );
    same("turn", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    if w.synthetic || !readable(a, ffx_battle::actor::bridge::MON_TOP.get(b), ACTOR as usize) {
        set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    }
    let rom = ffx_battle::menu::bridge::PLY_ROM;
    if w.synthetic || !readable(a, rom.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x20, 0x40);
        set(a, b, rom, t);
    }
    for round in 0..800 {
        if w.synthetic || round % 2 == 1 {
            dress(a, b, rng, &w);
        }
        fill(a, b, rng, w.out, 0x100);
        // the spoils' kind count
        poke(
            a,
            b,
            w.out + 0x20 + 0xd0,
            &[u8::try_from(rng.below(10)).unwrap()],
        );
        for k in 0..8 {
            poke(
                a,
                b,
                w.out + 0x20 + 0xd4 + 2 * k,
                &u16::try_from(0x2000 + rng.below(12)).unwrap().to_le_bytes(),
            );
        }
        calls(a, b, rng, &w);
    }
}

#[test]
fn turns_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7777_1111), true);
}

#[test]
fn turns_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x1111_7777), false);
}
