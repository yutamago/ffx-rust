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
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
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
    same("cue", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    if !w.synthetic {
        for _ in 0..30 {
            calls(a, b, rng);
        }
    }
    for _ in 0..2000 {
        randomize(a, b, rng, w.actors);
        calls(a, b, rng);
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
