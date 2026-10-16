//! The damage code (`ffx_battle::damage`: the `DmgCalc_*` steps, the
//! formulas, hits, criticals, elements, counters) against the original, as
//! in `battle.rs`: two oracles, the original in one and the Rust in the
//! other, on synthetic actors and commands and on the snapshots' actors and
//! kernel commands; results and every written byte must agree.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::*;
use ffx_battle::actor::bridge::{CHR_TOP, WEAPON_TOP};
use ffx_battle::damage::bridge as d;
use ffx_oracle::{Conv, Oracle};

/// `TOGetFFXLang`, which `DmgCalc_Bribe` asks (another module's).
const TO_GET_FFX_LANG: u32 = 0x0064_10e0;

/// Where the cases live: the actors, a command, a result and scratch for
/// the out values.
struct World {
    actors: u32,
    command: u32,
    info: u32,
    out: u32,
    /// The kernel's commands (on a snapshot), else empty.
    commands: Vec<u32>,
    synthetic: bool,
}

const ACTOR: u32 = ffx_battle::actor::ACTOR_SIZE;
const ACTORS: u32 = ffx_battle::actor::ACTORS;

/// A float's bits with signalling NaNs made quiet (the emulator keeps them
/// signalling, the CPU does not: `docs/porting.md`).
fn quiet(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// A fake kernel table (`MsGetExcelData`'s layout) of one segment holding
/// `len` random bytes, records 0..`len` of stride 1.
fn fake_table(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, len: u32) -> u32 {
    let t = scratch(a, b, 0x20 + len as usize);
    let mut head = vec![0u8; 0x20];
    head[0] = 1;
    head[8..10].copy_from_slice(&0u16.to_le_bytes());
    head[10..12].copy_from_slice(&u16::try_from(len - 1).unwrap().to_le_bytes());
    head[12..14].copy_from_slice(&1u16.to_le_bytes());
    head[16..20].copy_from_slice(&0x20u32.to_le_bytes());
    poke(a, b, t, &head);
    fill(a, b, rng, t + 0x20, len);
    t
}

fn world(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) -> World {
    let live = !synthetic
        && readable(a, CHR_TOP.get(b), (ACTORS * ACTOR) as usize)
        && readable(a, WEAPON_TOP.get(b), 0x390)
        && readable(a, ffx_kernel::bridge::CTB_BASE.get(b), 0x20);
    // without a battle running, actors of our own on the snapshot's globals
    let actors = if live {
        CHR_TOP.get(b)
    } else {
        let actors = scratch(a, b, (ACTORS * ACTOR) as usize);
        set(a, b, CHR_TOP, actors);
        let weapons = scratch(a, b, (2 * ACTORS * ffx_battle::actor::WEAPON_SIZE) as usize);
        set(a, b, WEAPON_TOP, weapons);
        let t = fake_table(a, b, rng, 0x100);
        set(a, b, ffx_kernel::bridge::CTB_BASE, t);
        actors
    };
    let mut commands = Vec::new();
    if !synthetic && readable(a, ffx_kernel::bridge::COMMAND.get(b), 0x20) {
        for id in (0x3000..0x3100).step_by(3) {
            let p = ffx_kernel::bridge::rom(b, ffx_kernel::bridge::COMMAND, id, 0);
            let q = ffx_kernel::bridge::rom(a, ffx_kernel::bridge::COMMAND, id, 0);
            assert_eq!(p, q);
            if readable(a, p, 0x60) {
                commands.push(p);
            }
        }
    }
    let w = World {
        actors,
        command: scratch(a, b, 0x60),
        info: scratch(a, b, 0x2c),
        out: scratch(a, b, 0x100),
        commands,
        synthetic: !live,
    };
    same("world", a, b);
    w
}

impl World {
    fn actor(&self, i: u32) -> u32 {
        self.actors + i * ACTOR
    }

    /// A command: the scratch one with random bytes, or a kernel one.
    fn command(&self, a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
        if !self.commands.is_empty() && rng.below(2) == 0 {
            return self.commands[rng.below(u32::try_from(self.commands.len()).unwrap()) as usize];
        }
        fill(a, b, rng, self.command, 0x60);
        self.command
    }

    /// Random values over the actors' battle fields (each round on the
    /// synthetic image, every other round on a snapshot), indices sane
    /// most of the time.
    fn shuffle(&self, a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, round: u32) {
        if !self.synthetic && round.is_multiple_of(2) {
            return;
        }
        for i in 0..ACTORS {
            let p = self.actor(i);
            for (at, len) in [
                (0x3b, 1),
                (0x590, 0x1a0),
                (0xd20, 0x20),
                (0xdf0, 8),
                (0xf5c, 4),
            ] {
                fill(a, b, rng, p + at, len);
            }
            let id = if rng.below(8) == 0 {
                rng.byte()
            } else {
                i.to_le_bytes()[0]
            };
            poke(a, b, p + 0xc, &[id]);
            for at in [0xd2c, 0xd30] {
                let f = match rng.below(4) {
                    0 => 1.0f32.to_bits() + rng.below(0x80_0000),
                    1 => quiet(rng.next()),
                    _ => (f32::from(i16::try_from(rng.range(-500, 500)).unwrap()) / 7.0).to_bits(),
                };
                poke(a, b, p + at, &f.to_le_bytes());
            }
        }
        fill(a, b, rng, self.info, 0x2c);
        fill(a, b, rng, self.out, 0x100);
        for g in [
            d::DEBUG_ALWAYS_CRIT,
            d::DEBUG_ALWAYS_HIT,
            d::DEBUG_ALL_MISS,
            d::COUNTER_ON,
            d::DEBUG_NO_DAMAGE_MONSTER,
            d::DEBUG_NO_DAMAGE_PARTY,
        ] {
            let v = u8::from(rng.below(5) == 0);
            set(a, b, g, v);
        }
        set(
            a,
            b,
            d::ESCAPE_RULE,
            rng.range(-1, 4).to_le_bytes()[0].cast_signed(),
        );
        set(a, b, d::GIL_AMOUNT, rng.word().cast_signed());
        if self.synthetic {
            set(a, b, ffx_bridge::x87::SSE2_AVAILABLE, rng.below(2));
        }
    }
}

/// A damage value: small, large, negative or at a limit.
fn damage(rng: &mut Xorshift) -> u32 {
    match rng.below(5) {
        0 => rng.word(),
        1 => rng.range(-10000, 0).cast_unsigned(),
        _ => rng.below(100_000),
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    for round in 0..2000 {
        w.shuffle(a, b, rng, round);
        let (i, j) = (rng.below(ACTORS), rng.below(ACTORS));
        let (user, target) = (w.actor(i), w.actor(j));
        let cmd = w.command(a, b, rng);
        let (info, out) = (w.info, w.out);
        let chr = if rng.below(6) == 0 {
            rng.below(0x100)
        } else {
            i
        };
        let dmg = damage(rng);
        twin("MsGetRamChrSummon", a, b, 0x0079_af60, &[chr], ALL, |m| {
            d::ms_get_ram_chr_summon(m, chr)
        });
        let kind = rng.range(-1, 4).cast_unsigned();
        twin("MsGetRndChr", a, b, 0x0078_d210, &[chr, kind], ALL, |m| {
            d::ms_get_rnd_chr(m, chr, kind.cast_signed())
        });
        // the formulas
        let formula = rng.range(-1, 0x19).cast_unsigned();
        let power = match rng.below(3) {
            0 => rng.word(),
            _ => rng.below(0x40),
        };
        let status = rng.next();
        let stat = [0, 1, 2, 4, 3][rng.below(5) as usize];
        // formulas 0x11-0x13 divide by the user's max HP or MP (a fault on 0)
        for at in [0x594, 0x598] {
            if u32::from_le_bytes(a.read(user + at, 4).unwrap().try_into().unwrap()) == 0 {
                poke(a, b, user + at, &1u32.to_le_bytes());
            }
        }
        let vary = rng.below(2);
        let outs = [[0, 0], [out, out + 4], [0, out + 8]][rng.below(3) as usize];
        let args = [
            user, target, cmd, formula, power, status, stat, vary, outs[0], outs[1], dmg,
        ];
        twin("MsCalcDamageCommand", a, b, 0x0078_9bf0, &args, ALL, |m| {
            d::ms_calc_damage_command(
                m,
                user,
                target,
                cmd,
                formula.cast_signed(),
                power.cast_signed(),
                status,
                stat.cast_signed(),
                vary.cast_signed(),
                outs[0],
                outs[1],
                dmg.cast_signed(),
            )
            .cast_unsigned()
        });
        if rng.below(4) == 0 {
            twin(
                "MsCalcDamageCommand",
                a,
                b,
                0x0078_9bf0,
                &[
                    user, target, 0, formula, power, status, stat, vary, 0, 0, dmg,
                ],
                ALL,
                |m| {
                    d::ms_calc_damage_command(
                        m,
                        user,
                        target,
                        0,
                        formula.cast_signed(),
                        power.cast_signed(),
                        status,
                        stat.cast_signed(),
                        vary.cast_signed(),
                        0,
                        0,
                        dmg.cast_signed(),
                    )
                    .cast_unsigned()
                },
            );
        }
        twin("MsCalcEscape", a, b, 0x0078_a780, &[chr], ALL, |m| {
            d::ms_calc_escape(m, chr)
        });
        let ids = [rng.below(0x20), rng.below(0x20)];
        twin(
            "DmgCalc_StatBoostAbility",
            a,
            b,
            0x0078_91e0,
            &[ids[0], ids[1], cmd, dmg],
            ALL,
            |m| {
                d::dmg_calc_stat_boost_ability(m, ids[0], ids[1], cmd, dmg.cast_signed())
                    .cast_unsigned()
            },
        );
        twin(
            "DmgCalc_PowerMagicBreak",
            a,
            b,
            0x0078_9290,
            &[user, cmd, dmg],
            ALL,
            |m| d::dmg_calc_power_magic_break(m, user, cmd, dmg.cast_signed()).cast_unsigned(),
        );
        twin(
            "DmgCalc_Crit",
            a,
            b,
            0x0078_9690,
            &[user, target, cmd, out, dmg],
            ALL,
            |m| d::dmg_calc_crit(m, user, target, cmd, out, dmg.cast_signed()).cast_unsigned(),
        );
        twin(
            "DmgCalc_ApplyZombieDrainInteraction",
            a,
            b,
            0x0078_a200,
            &[user, cmd, info, dmg],
            ALL,
            |m| {
                d::dmg_calc_apply_zombie_drain_interaction(m, user, cmd, info, dmg.cast_signed())
                    .cast_unsigned()
            },
        );
        let elements = if rng.below(8) == 0 {
            rng.next()
        } else {
            rng.below(0x100)
        };
        twin(
            "DmgCalc_Elem",
            a,
            b,
            0x0078_a360,
            &[target, cmd, elements, dmg],
            ALL,
            |m| d::dmg_calc_elem(m, target, elements, dmg.cast_signed()).cast_unsigned(),
        );
        twin(
            "DmgCalc_Defend",
            a,
            b,
            0x0078_a7d0,
            &[target, cmd, out, info, dmg],
            ALL,
            |m| d::dmg_calc_defend(m, target, cmd, out, info, dmg.cast_signed()).cast_unsigned(),
        );
        twin(
            "DmgCalc_Armored",
            a,
            b,
            0x0078_a830,
            &[user, target, cmd, info, out + 4, dmg],
            ALL,
            |m| {
                d::dmg_calc_armored(m, user, target, cmd, info, out + 4, dmg.cast_signed())
                    .cast_unsigned()
            },
        );
        let counter = rng.range(-1, 4).cast_unsigned();
        twin(
            "DmgCalc_CheckHit",
            a,
            b,
            0x0078_a890,
            &[user, target, cmd, info, counter],
            ALL,
            |m| d::dmg_calc_check_hit(m, user, target, cmd, info, counter.cast_signed()),
        );
        let mask = rng.next();
        twin(
            "DmgCalc_DmgImmunity",
            a,
            b,
            0x0078_aac0,
            &[target, cmd, out, mask, out + 8, dmg],
            ALL,
            |m| {
                d::dmg_calc_dmg_immunity(m, target, cmd, out, mask, out + 8, dmg.cast_signed())
                    .cast_unsigned()
            },
        );
        twin(
            "DmgCalc_GravImmunity",
            a,
            b,
            0x0078_ad80,
            &[target, formula, out, mask, out + 8, dmg],
            ALL,
            |m| {
                d::dmg_calc_grav_immunity(
                    m,
                    target,
                    formula.cast_signed(),
                    out,
                    mask,
                    out + 8,
                    dmg.cast_signed(),
                )
                .cast_unsigned()
            },
        );
        twin(
            "DmgCalc_Protect",
            a,
            b,
            0x0078_ad40,
            &[cmd, out, out + 4, info, dmg],
            ALL,
            |m| d::dmg_calc_protect(m, cmd, out, out + 4, info, dmg.cast_signed()).cast_unsigned(),
        );
        twin(
            "DmgCalc_Shell",
            a,
            b,
            0x0078_adc0,
            &[cmd, out, out + 4, info, dmg],
            ALL,
            |m| d::dmg_calc_shell(m, cmd, out, out + 4, info, dmg.cast_signed()).cast_unsigned(),
        );
        let flags = rng.next();
        twin(
            "DmgCalc_ShatterFail",
            a,
            b,
            0x0078_ba30,
            &[target, info, flags, out, out + 0x10, out + 0x40],
            0,
            |m| {
                d::dmg_calc_shatter_fail(m, info, flags, out, out + 0x10, out + 0x40);
                0
            },
        );
        twin(
            "DmgCalc_Threaten",
            a,
            b,
            0x0078_bd50,
            &[target, info, out, mask, flags, out + 0x10, out + 0x40],
            ALL,
            |m| d::dmg_calc_threaten(m, info, out, mask, flags, out + 0x10, out + 0x40),
        );
        twin(
            "DmgCalc_ReelBonus",
            a,
            b,
            0x0078_bd90,
            &[user, dmg],
            ALL,
            |m| d::dmg_calc_reel_bonus(m, user, dmg.cast_signed()),
        );
        let always = u32::from(rng.below(4) == 0);
        twin(
            "DmgCalc_MarkDamaged",
            a,
            b,
            0x0078_bf50,
            &[i, user, cmd, out + 0x40, always],
            ALL,
            |m| d::dmg_calc_mark_damaged(m, user, cmd, out + 0x40, always.cast_signed()),
        );
        twin(
            "DmgCalc_CheckNuls",
            a,
            b,
            0x0078_bfb0,
            &[cmd, elements, info],
            ALL,
            |m| d::dmg_calc_check_nuls(m, elements, info).cast_unsigned(),
        );
        twin(
            "DmgCalc_Berserk",
            a,
            b,
            0x0078_c0b0,
            &[user, dmg],
            ALL,
            |m| d::dmg_calc_berserk(m, user, dmg.cast_signed()).cast_unsigned(),
        );
        twin(
            "DmgCalc_QueueRemoveStatusOnHit",
            a,
            b,
            0x0078_c0f0,
            &[j, target, cmd, out, out + 0x10, info],
            0,
            |m| {
                d::dmg_calc_queue_remove_status_on_hit(m, target, cmd, out + 0x10, info);
                0
            },
        );
        let delayed = rng.below(2);
        twin(
            "DmgCalc_DelayImmunity",
            a,
            b,
            0x0078_c180,
            &[target, out + 0x40, out + 0x10, mask, flags, delayed],
            ALL,
            |m| {
                d::dmg_calc_delay_immunity(
                    m,
                    target,
                    out + 0x40,
                    out + 0x10,
                    flags,
                    delayed.cast_signed(),
                )
            },
        );
        let apply = rng.below(2);
        let (ui, ti) = if rng.below(4) == 0 { (i, i) } else { (i, j) };
        twin(
            "DmgCalc_CheckCounter",
            a,
            b,
            0x0078_c1d0,
            &[apply, ui, user, ti, target, cmd],
            ALL,
            |m| {
                d::dmg_calc_check_counter(
                    m,
                    apply.cast_signed(),
                    ui.cast_signed(),
                    user,
                    ti.cast_signed(),
                    target,
                    cmd,
                )
                .cast_unsigned()
            },
        );
        twin(
            "DmgCalc_NoDamageDeath",
            a,
            b,
            0x0078_c480,
            &[target, info, flags, mask, out + 0x40],
            ALL,
            |m| d::dmg_calc_no_damage_death(m, info, flags, mask, out + 0x40),
        );
        twin(
            "DmgCalc_ShieldBoost",
            a,
            b,
            0x0078_c530,
            &[target, out, dmg],
            ALL,
            |m| d::dmg_calc_shield_boost(m, target, out, dmg.cast_signed()).cast_unsigned(),
        );
        let id = match rng.below(4) {
            0 => 0x311f,
            1 => 0x2000 + rng.below(0x1000),
            _ => rng.next(),
        };
        let p = if rng.below(4) == 0 { 0 } else { out + 0x20 };
        twin(
            "DmgCalc_MagicBooster",
            a,
            b,
            0x0078_c580,
            &[user, cmd, id, p],
            ALL,
            |m| d::dmg_calc_magic_booster(m, user, cmd, id.cast_signed(), p).cast_unsigned(),
        );
        twin(
            "DmgCalc_Alchemy",
            a,
            b,
            0x0078_c5f0,
            &[user, cmd, id, formula, dmg],
            ALL,
            |m| {
                d::dmg_calc_alchemy(m, user, cmd, id, formula.cast_signed(), dmg.cast_signed())
                    .cast_unsigned()
            },
        );
        let missed = rng.below(3);
        let counts = [0, 0, 1, rng.next()];
        poke(
            a,
            b,
            out + 0x60,
            &[
                counts[rng.below(4) as usize].to_le_bytes(),
                counts[rng.below(4) as usize].to_le_bytes(),
            ]
            .concat(),
        );
        twin(
            "DmgCalc_HitKind",
            a,
            b,
            0x0078_c640,
            &[missed, out + 0x60],
            ALL,
            |m| d::dmg_calc_hit_kind(m, missed.cast_signed(), out + 0x60),
        );
        twin(
            "DmgCalc_IsDistill",
            a,
            b,
            0x0078_c670,
            &[flags],
            ALL,
            |_| d::dmg_calc_is_distill(flags).cast_unsigned(),
        );
        let (haste, slow) = (rng.below(2), rng.below(2));
        let ctb = rng.word();
        twin(
            "MsAdjustCTB",
            a,
            b,
            0x0078_c150,
            &[user, ctb, haste, slow],
            ALL,
            |_| {
                d::ms_adjust_ctb(ctb.cast_signed(), haste.cast_signed(), slow.cast_signed())
                    .cast_unsigned()
            },
        );
        let rank = rng.range(-2, 6).cast_unsigned();
        twin(
            "MsGetNextCTB",
            a,
            b,
            0x0078_d1d0,
            &[user, rank, haste, slow],
            ALL,
            |m| {
                d::ms_get_next_ctb(
                    m,
                    user,
                    rank.cast_signed(),
                    haste.cast_signed(),
                    slow.cast_signed(),
                )
                .cast_unsigned()
            },
        );
        twin("DebugCanTakeDamage", a, b, 0x0078_d3a0, &[chr], ALL, |m| {
            d::debug_can_take_damage(m, chr)
        });
        twin(
            "DmgCalc_ApplyStatUpBuffs",
            a,
            b,
            0x0078_d730,
            &[user, target, cmd],
            0,
            |m| {
                d::dmg_calc_apply_stat_up_buffs(m, target, cmd);
                0
            },
        );
        let parent = if rng.below(2) == 0 { 0 } else { out + 0x80 };
        twin(
            "SetUpDamageInfo",
            a,
            b,
            0x0078_dae0,
            &[info, parent, target],
            0,
            |m| {
                d::set_up_damage_info(m, info, parent, target);
                0
            },
        );
        twin(
            "DmgCalc_Delay",
            a,
            b,
            0x0078_e0f0,
            &[target, cmd, flags, out + 0x40],
            ALL,
            |m| d::dmg_calc_delay(m, target, cmd, flags, out + 0x40),
        );
        let s = rng.range(-2, 16);
        twin(
            "MsCheckStatCount",
            a,
            b,
            0x0079_e330,
            &[s.cast_unsigned()],
            ALL,
            |m| d::ms_check_stat_count(m, s),
        );
        twin("MsCheckWrongStatus", a, b, 0x0079_e340, &[cmd], ALL, |m| {
            d::ms_check_wrong_status(m, cmd).cast_unsigned()
        });
        let (inflicted, removed, delayed) = (out + 0x80, out + 0xa0, out + 0xc0);
        let ids = if rng.below(4) == 0 { [i, i] } else { [i, j] };
        twin(
            "DmgCalc_InflictStatus",
            a,
            b,
            0x0078_ae00,
            &[
                ids[0],
                user,
                ids[1],
                target,
                cmd,
                inflicted,
                removed,
                info,
                flags,
                out + 0x40,
                delayed,
            ],
            ALL,
            |m| {
                d::dmg_calc_inflict_status(
                    m,
                    ids[0],
                    user,
                    ids[1],
                    target,
                    cmd,
                    inflicted,
                    removed,
                    info,
                    flags,
                    out + 0x40,
                    delayed,
                )
            },
        );
        let s2 = if rng.below(2) == 0 {
            rng.next()
        } else {
            rng.below(0x1_0000)
        };
        twin(
            "DmgCalc_InflictStatus_Extra",
            a,
            b,
            0x0078_b4e0,
            &[
                ids[0],
                user,
                ids[1],
                target,
                cmd,
                inflicted,
                removed,
                info,
                flags,
                out + 0x40,
                s2,
            ],
            ALL,
            |m| {
                d::dmg_calc_inflict_status_extra(
                    m, ids[0], user, target, cmd, inflicted, removed, info, flags, s2,
                )
            },
        );
        // the result type: small counts and damage of every sign
        let small = |rng: &mut Xorshift| -> [u8; 4] {
            [0u32, 0, 1, 2][rng.below(4) as usize].to_le_bytes()
        };
        for at in (0x80..0xc0).step_by(4) {
            let v = small(rng);
            poke(a, b, out + at, &v);
        }
        for at in [0x40, 0x44, 0x48] {
            let v = [0, 0, 5, (-3i32).cast_unsigned(), 200][rng.below(5) as usize];
            poke(a, b, out + at, &v.to_le_bytes());
        }
        let args = [
            ids[0],
            ids[1],
            rng.below(4),
            rng.below(2),
            rng.range(90, 110).cast_unsigned(),
            rng.range(90, 110).cast_unsigned(),
            out + 0x40,
            inflicted,
            removed,
            cmd,
            rng.next(),
            [0, 0, 1, rng.next()][rng.below(4) as usize],
            if rng.below(2) == 0 {
                rng.below(8)
            } else {
                rng.next()
            },
            rng.next(),
            out + 0xd0,
        ];
        twin("DmgCalc_ResultType", a, b, 0x0078_c270, &args, ALL, |m| {
            d::dmg_calc_result_type(
                m,
                args[0],
                args[1],
                args[2].cast_signed(),
                args[3].cast_signed(),
                args[4].cast_signed(),
                args[5].cast_signed(),
                args[6],
                args[7],
                args[8],
                args[10],
                args[11].cast_signed(),
                args[12],
                args[14],
            )
        });
        // Bribe: the language from TOGetFFXLang (fixed on the image alone,
        // where the settings it reads do not exist)
        if w.synthetic {
            let lang = [1, 0xb][rng.below(2) as usize];
            for o in [&mut *a, &mut *b] {
                let at = o.rebase(TO_GET_FFX_LANG);
                o.stub_return(at, lang).unwrap();
            }
        }
        if rng.below(4) == 0 {
            poke(a, b, target + 0xe, &0x10efu16.to_le_bytes());
        }
        if u32::from_le_bytes(a.read(target + 0x594, 4).unwrap().try_into().unwrap()) == 0 {
            poke(a, b, target + 0x594, &1u32.to_le_bytes());
        }
        twin(
            "DmgCalc_Bribe",
            a,
            b,
            0x0078_bdd0,
            &[j, target, cmd, inflicted, info],
            ALL,
            |m| {
                d::dmg_calc_bribe(m, j, target, cmd, inflicted, info, |m: &mut Oracle| {
                    let at = m.rebase(TO_GET_FFX_LANG);
                    m.call(at, Conv::Cdecl, &[]).unwrap().eax
                })
            },
        );
        // the whole step for one target
        fill(a, b, rng, info, 0x2c);
        let id = match rng.below(3) {
            0 => 0x311f,
            1 => 0x2000 + rng.below(0x10),
            _ => 0x3000 + rng.below(0x100),
        };
        let o = d::DmgCalcOut {
            damage: if rng.below(2) == 0 { 0 } else { out + 0xe0 },
            hits: out + 0xf0,
            misses: out + 0xf4,
            changed: if rng.below(2) == 0 { 0 } else { out + 0xf8 },
        };
        let args = [
            ids[0], user, ids[1], target, cmd, id, info, o.damage, o.hits, o.misses, o.changed,
        ];
        twin("DmgCalc", a, b, 0x0078_e630, &args, ALL, |m| {
            d::dmg_calc(
                m,
                ids[0],
                user,
                ids[1],
                target,
                cmd,
                id,
                info,
                o,
                |m: &mut Oracle| {
                    let at = m.rebase(TO_GET_FFX_LANG);
                    m.call(at, Conv::Cdecl, &[]).unwrap().eax
                },
            )
            .cast_unsigned()
        });
        same("damage steps", a, b);
    }
}

#[test]
fn damage_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0bad_cafe), true);
}

#[test]
fn damage_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5eed_d00d), false);
}
