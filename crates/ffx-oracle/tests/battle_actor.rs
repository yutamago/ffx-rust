//! The actor and battle getters (`ffx_battle::actor`, `ffx_battle::state`)
//! against the original, as in `battle.rs`, on the shared battle world.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::*;
use battle_common::*;
use ffx_battle::{actor::bridge as act, state::bridge as st};
use ffx_oracle::Oracle;

/// A getter by actor index: name, `ffx.sym` address, the Rust.
type ByIndex = (&'static str, u32, fn(&Oracle, u32) -> u32);

const BY_INDEX: [ByIndex; 21] = [
    ("MsGetRamChrHP", 0x0079_add0, |m, i| {
        act::ms_get_ram_chr_hp(m, i)
    }),
    ("MsGetRamChrHP_0079adf0", 0x0079_adf0, |m, i| {
        act::ms_get_ram_chr_hp_max(m, i)
    }),
    ("MsGetRamChrMP", 0x0079_ae50, |m, i| {
        act::ms_get_ram_chr_mp(m, i)
    }),
    ("MsGetRamChrMPmax", 0x0079_ae70, |m, i| {
        act::ms_get_ram_chr_mp_max(m, i)
    }),
    ("MsGetRamChrMag", 0x0079_ae90, |m, i| {
        act::ms_get_ram_chr_mag(m, i)
    }),
    ("MsGetRamChrName", 0x0079_af10, |m, i| {
        act::ms_get_ram_chr_name(m, i)
    }),
    ("MsGetRamChrIconNumber", 0x0079_ae10, |m, i| {
        act::ms_get_ram_chr_icon_number(m, i)
    }),
    ("MsGetRamChrIconType", 0x0079_ae30, |m, i| {
        act::ms_get_ram_chr_icon_type(m, i)
    }),
    ("MsGetRamChrMegas", 0x0079_aed0, |m, i| {
        act::ms_get_ram_chr_megas(m, i)
    }),
    ("MsGetRamChrDisableZan", 0x0079_ad90, |m, i| {
        act::ms_get_ram_chr_disable_zan(m, i)
    }),
    ("MsGetRamChrCTBmax", 0x0079_ad10, |m, i| {
        act::ms_get_ram_chr_ctb_max(m, i)
    }),
    ("MsGetChrStatDeath", 0x0079_5780, |m, i| {
        act::ms_get_chr_stat_death(m, i)
    }),
    ("MsGetChrStatStone", 0x0079_5800, |m, i| {
        act::ms_get_chr_stat_stone(m, i)
    }),
    ("MsGetChrStatWeak", 0x0079_5840, |m, i| {
        act::ms_get_chr_stat_weak(m, i)
    }),
    ("MsGetChrStatZombie", 0x0079_5880, |m, i| {
        act::ms_get_chr_stat_zombie(m, i)
    }),
    ("MsGetChrStatPoison", 0x0079_57c0, |m, i| {
        act::ms_get_chr_stat_poison(m, i)
    }),
    ("MsGetChrLimitGauge", 0x0079_5550, |m, i| {
        act::ms_get_chr_limit_gauge(m, i)
    }),
    ("MsGetChrLimitGaugeMax", 0x0079_5590, |m, i| {
        act::ms_get_chr_limit_gauge_max(m, i)
    }),
    ("MsGetChrHandle", 0x0079_5080, |m, i| {
        act::ms_get_chr_handle(m, i)
    }),
    ("MsGetChrGroup", 0x0079_5040, |m, i| {
        act::ms_get_chr_group(m, i)
    }),
    ("MsGetChrCursorFlag", 0x0079_4ca0, |m, i| {
        act::ms_get_chr_cursor_flag(m, i)
    }),
];

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let records = scratch(a, b, 14 * 0x100);
    let group = scratch(a, b, 4);
    for round in 0..300 {
        w.shuffle(a, b, rng, round);
        if w.synthetic || round > 0 {
            for i in 0..ACTORS {
                let p = w.actor(i);
                for (at, len) in [(0, 4), (0x500, 8), (0xdc8, 0x20), (0x6c0, 0x40)] {
                    fill(a, b, rng, p + at, len);
                }
            }
            fill(a, b, rng, records, 14 * 0x100);
            fill(a, b, rng, group, 4);
            set(a, b, st::FIELD_RECORDS, records);
            set(a, b, st::SCENE_GROUP, group);
            set(
                a,
                b,
                st::FIELD_INDEX,
                i16::try_from(rng.below(0x100)).unwrap(),
            );
            for g in [st::STATUS, st::EXE] {
                let v = [0, 0, 1, -1][rng.below(4) as usize];
                set(a, b, g, v);
            }
        }
        for _ in 0..8 {
            let i = match rng.below(4) {
                0 => rng.next(),
                1 => rng.below(0x100),
                _ => rng.below(ACTORS),
            };
            for (name, sym, rust) in BY_INDEX {
                twin(name, a, b, sym, &[i], ALL, |m| rust(m, i));
            }
            twin("MsGetChrCommand", a, b, 0x007b_09b0, &[i], ALL, |m| {
                act::ms_get_chr_command(m, i)
            });
            twin("MsGetChrCommand2", a, b, 0x007b_09e0, &[i], ALL, |m| {
                act::ms_get_chr_command2(m, i)
            });
        }
        twin("MsBattleCheck", a, b, 0x0078_0c90, &[], ALL, |m| {
            st::ms_battle_check(m)
        });
        twin("MsBattleChrStart", a, b, 0x0078_0cb0, &[], ALL, |m| {
            st::ms_battle_chr_start(m)
        });
        twin("MsBattleGetNum", a, b, 0x0078_16b0, &[], ALL, |m| {
            st::ms_battle_get_num(m)
        });
        set(a, b, act::CHR_TOP, w.actors);
        same("getters", a, b);
    }
}

#[test]
fn actor_getters_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7777_1111), true);
}

#[test]
fn actor_getters_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x1111_7777), false);
}
