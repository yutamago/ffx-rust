//! The battle's input and command cues (`ffx_battle::cue`) against the
//! original, as in `battle.rs`: random cues and actors' cue fields on the
//! image alone and on both snapshots (the live cues first on a battle).

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTORS, world};
use battle_common::*;
use ffx_battle::cue::bridge as cue;
use ffx_battle::state::bridge as st;
use ffx_oracle::Oracle;

/// An actor index, mostly a real one.
fn chr(rng: &mut Xorshift) -> u32 {
    match rng.below(10) {
        0 => rng.next(),
        1 => rng.below(0x60),
        _ => rng.below(ACTORS),
    }
}

fn randomize(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actors: u32) {
    let n = rng.range(-2, 0x40);
    set(a, b, st::INPUT_CUE_COUNT, i8::try_from(n).unwrap());
    let n = rng.range(-2, 0x30);
    set(a, b, st::COMMAND_CUE_COUNT, i8::try_from(n).unwrap());
    let input: Vec<u8> = (0..0x40 * 8)
        .map(|k| match k % 8 {
            0 => u8::try_from(rng.below(ACTORS + 2)).unwrap(),
            3 => u8::from(rng.below(3) == 0),
            _ => rng.byte(),
        })
        .collect();
    poke(a, b, st::INPUT_CUE.ptr(b).addr, &input);
    let commands: Vec<u8> = (0..0x30 * st::CUE_SIZE)
        .map(|k| match k % st::CUE_SIZE {
            0 => u8::try_from(rng.below(ACTORS + 2)).unwrap(),
            _ => rng.byte(),
        })
        .collect();
    poke(a, b, st::COMMAND_CUE.ptr(b).addr, &commands);
    for i in 0..ACTORS {
        let p = actors + i * ffx_battle::actor::ACTOR_SIZE;
        let own = |rng: &mut Xorshift, n: u32| {
            if rng.below(4) == 0 {
                0xff
            } else {
                u8::try_from(rng.below(n)).unwrap()
            }
        };
        let fields = [own(rng, 0x40), own(rng, 0x30), rng.byte(), rng.byte()];
        poke(a, b, p + 0xde4, &fields);
        fill(a, b, rng, p + 0x65c, 1);
        fill(a, b, rng, p + 0xd40, 0x20);
        fill(a, b, rng, p + 0xdc0, 4);
        // the damage buffers: next, count, user, command; results
        for o in [0x774, 0xa4c] {
            let head = [
                u8::try_from(rng.below(6)).unwrap(),
                u8::try_from(rng.below(6)).unwrap(),
                u8::try_from(rng.below(ACTORS + 1)).unwrap(),
                u8::try_from(rng.below(4)).unwrap(),
            ];
            poke(a, b, p + o, &head);
            fill(a, b, rng, p + o + 4, 0x2c * 6);
        }
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, out: u32) {
    let c = chr(rng);
    twin("MsCheckInputCueExe", a, b, 0x007b_1fd0, &[c], ALL, |m| {
        cue::ms_check_input_cue_exe(m, c)
    });
    twin("MsCheckCommandCueExe", a, b, 0x007b_07d0, &[c], ALL, |m| {
        cue::ms_check_command_cue_exe(m, c)
    });
    twin("MsCheckInputCuePos", a, b, 0x007b_2020, &[c], 0, |m| {
        cue::ms_check_input_cue_pos(m, c);
        0
    });
    twin("MsCheckTurnInputCue", a, b, 0x007b_2060, &[c], ALL, |m| {
        cue::ms_check_turn_input_cue(m, c)
    });
    let on = rng.next();
    twin("MsExeInputCue", a, b, 0x007b_2290, &[c, on], ALL, |m| {
        cue::ms_exe_input_cue(m, c, on.to_le_bytes()[0])
    });
    twin("MsExeComandCue", a, b, 0x007b_0960, &[c, on], ALL, |m| {
        cue::ms_exe_comand_cue(m, c, on.to_le_bytes()[0])
    });
    let (p, t, f) = (rng.next(), rng.next(), rng.next());
    twin(
        "MsRegistInputCue",
        a,
        b,
        0x007b_2430,
        &[c, p, t, f],
        ALL,
        |m| {
            cue::ms_regist_input_cue(
                m,
                c,
                p.to_le_bytes()[0],
                t.to_le_bytes()[0],
                f.to_le_bytes()[0],
            )
            .cast_unsigned()
        },
    );
    let prio = match rng.below(4) {
        0 => rng.next(),
        1 => rng.below(0x10),
        _ => rng.below(0x100),
    };
    let flag = if rng.below(3) == 0 { 0 } else { rng.next() };
    let after = chr(rng);
    let c = rng.below(ACTORS);
    twin(
        "MsInsertInputCue",
        a,
        b,
        0x007b_2300,
        &[c, prio, t, flag, after, on],
        ALL,
        |m| {
            cue::ms_insert_input_cue(
                m,
                c,
                prio,
                t.to_le_bytes()[0],
                flag,
                after,
                on.to_le_bytes()[0],
            )
            .cast_unsigned()
        },
    );
    let slot = if rng.below(8) == 0 {
        0xff
    } else {
        rng.below(0x32)
    };
    let keep = rng.below(2);
    let c = chr(rng);
    twin(
        "MsDeleteCommandCue",
        a,
        b,
        0x007b_0860,
        &[c, slot, keep],
        ALL,
        |m| cue::ms_delete_command_cue(m, c, slot, keep).cast_unsigned(),
    );
    // the damage buffers and target lists
    let (c, t) = (chr(rng), chr(rng));
    let outs = [
        if rng.below(4) == 0 { 0 } else { out },
        if rng.below(4) == 0 { 0 } else { out + 4 },
        if rng.below(4) == 0 { 0 } else { out + 8 },
    ];
    twin(
        "MsGetDamageResult",
        a,
        b,
        0x0079_a920,
        &[c, t, outs[0], outs[1], outs[2]],
        ALL,
        |m| cue::ms_get_damage_result(m, c, t, outs[0], outs[1], outs[2]).cast_unsigned(),
    );
    twin(
        "MsGetDamageInfo",
        a,
        b,
        0x0079_a680,
        &[c, t, outs[0], outs[1], outs[2]],
        ALL,
        |m| cue::ms_get_damage_info(m, c, t, outs).cast_unsigned(),
    );
    twin(
        "MsGetDamageInfoNext",
        a,
        b,
        0x0079_a760,
        &[c, t, outs[0], outs[1], outs[2]],
        ALL,
        |m| cue::ms_get_damage_info_next(m, c, t, outs).cast_unsigned(),
    );
    // a target buffer of the actor's running command now and then, its
    // results' hit kinds 0-2
    let (cc, tt) = (c & 0xff, t & 0xff);
    if cc < ACTORS && tt < ACTORS && rng.below(2) == 0 {
        let id = b
            .read(ffx_battle::actor::bridge::ms_get_chr_command(b, cc) + 2, 1)
            .unwrap()[0];
        // one buffer, or (round 27) both
        let bufs: &[u32] = [&[0x774u32][..], &[0xa4c], &[0x774, 0xa4c]][rng.below(3) as usize];
        for &o in bufs {
            let buf = ffx_battle::actor::bridge::ms_get_chr(b, tt) + o;
            let head = [
                u8::try_from(rng.below(4)).unwrap(),
                u8::try_from(rng.below(6)).unwrap(),
                u8::try_from(cc).unwrap(),
                id,
            ];
            poke(a, b, buf, &head);
            for k in 0..6 {
                poke(
                    a,
                    b,
                    buf + 0x18 + 0x2c * k + 1,
                    &[u8::try_from(rng.below(3)).unwrap()],
                );
            }
        }
    }
    twin("MsMergeDamage", a, b, 0x0079_aff0, &[c, t], ALL, |m| {
        cue::ms_merge_damage(m, c, t).cast_unsigned()
    });
    same("MsMergeDamage", a, b);
    twin(
        "MsGetTargetList",
        a,
        b,
        0x007b_7090,
        &[c, outs[1]],
        ALL,
        |m| cue::ms_get_target_list(m, c, outs[1]),
    );
    same("cue", a, b);
    // the weighted target list (round 22), unless it would pass 0x40
    // entries: the original then hangs in MsBattleError. The total from
    // the same list and hits (both read-only here).
    let list = cue::ms_get_target_list(b, c, 0);
    if list != 0 {
        let targets = b.read(list, 0x1f).unwrap();
        let total: i32 = targets
            .iter()
            .take_while(|&&t| t != 0xff)
            .map(|&t| cue::ms_get_damage_info(b, c, t.into(), [0; 3]).max(0))
            .sum();
        if total > 0x40 {
            return;
        }
    }
    let shuffle = [0, 1, rng.next()][rng.below(3) as usize];
    twin(
        "MsGetTargetRandomList",
        a,
        b,
        0x007b_7170,
        &[c, outs[1], shuffle],
        ALL,
        |m| {
            let mut call = |m: &mut Oracle, f: u32, args: &[u32]| {
                m.call(f, ffx_oracle::Conv::Cdecl, args).unwrap().eax
            };
            cue::ms_get_target_random_list(m, &mut call, c, outs[1], shuffle)
        },
    );
    same("MsGetTargetRandomList", a, b);
}

fn commands(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, cmd: u32) {
    fill(a, b, rng, cmd, 0x48);
    poke(a, b, cmd + 3, &[u8::try_from(rng.below(4)).unwrap()]);
    for k in 0..4 {
        let id = u16::try_from(0x2000 + rng.below(0x20)).unwrap();
        poke(a, b, cmd + 8 + 0x10 * k, &id.to_le_bytes());
    }
    let c = rng.below(ACTORS);
    let (t, k, f, on) = (rng.next(), rng.next(), rng.next(), rng.next());
    twin(
        "MsRegistCommandCue",
        a,
        b,
        0x007b_0b90,
        &[c, cmd, t, k, f],
        ALL,
        |m| {
            let [t, k, f] = [t, k, f].map(|x| x.to_le_bytes()[0]);
            cue::ms_regist_command_cue(m, c, cmd, t, k, f).cast_unsigned()
        },
    );
    let kind = match rng.below(3) {
        0 => rng.next(),
        1 => rng.below(0x10),
        _ => rng.below(0x100),
    };
    let after = chr(rng);
    twin(
        "MsInsertCommandCue",
        a,
        b,
        0x007b_0a30,
        &[c, cmd, t, kind, f, on, after],
        ALL,
        |m| {
            let [t, f, on] = [t, f, on].map(|x| x.to_le_bytes()[0]);
            cue::ms_insert_command_cue(m, c, cmd, t, kind, f, on, after).cast_unsigned()
        },
    );
    let (c, keep) = (chr(rng), rng.below(2));
    twin(
        "MsCleanCommandCue",
        a,
        b,
        0x007b_0830,
        &[c, keep],
        ALL,
        |m| cue::ms_clean_command_cue(m, c, keep).cast_unsigned(),
    );
    same("command cue", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let out = w.out;
    // the party, sometimes with bytes past the actors
    let party: Vec<u8> = (0..7)
        .map(|_| match rng.below(6) {
            0 => 0xff,
            1 => rng.byte(),
            _ => u8::try_from(rng.below(ACTORS)).unwrap(),
        })
        .collect();
    if w.synthetic {
        poke(a, b, ffx_battle::menu::bridge::PARTY.ptr(b).addr, &party);
    }
    if !w.synthetic {
        for _ in 0..30 {
            calls(a, b, rng, out);
        }
    }
    for _ in 0..2000 {
        randomize(a, b, rng, w.actors);
        calls(a, b, rng, out);
        commands(a, b, rng, w.command);
    }
}

#[test]
fn cues_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0c0e_0c0e), true);
}

#[test]
fn cues_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0e0c_0e0c), false);
}
