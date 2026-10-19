//! A script's actor playing a motion (`ffx_battle::motion`, round 23:
//! `Battle_ScriptPlayMotion`) against the original, as in `battle.rs`: the
//! shared battle world with a fake model (zeros, which `MsCalcMotionSpeed`'s
//! chara calls read alike) or none, the actor's motion fields drawn at
//! random, on the image alone and on both snapshots. The script and chara
//! callees are logging stubs in both oracles (`AtelPopStackInteger` gives
//! the motion, `AtelGetBattleIdAdrs2` the actor's index byte,
//! `Ch_GetSeqFrame` the frames); `MsCalcMotionSpeed`, `ms_motion_trans`
//! (no stopped models), `MsMotBinFrame` and `msMoveReset` run as the
//! originals and the ports.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::{ACTORS, world};
use battle_common::*;
use ffx_battle::motion::bridge as mo;
use ffx_oracle::{Conv, Oracle};

/// `Ms_Gui_BattleChr2Proc` (`0x007de790`), code no test runs: the stubs.
const GUI_STUBS: u32 = 0x007d_e790;

/// The stubbed callees and their argument counts.
const CALLEES: [(u32, u8); 7] = [
    (mo::ATEL_POP_STACK_INTEGER, 2),
    (mo::ATEL_GET_BATTLE_ID_ADRS2, 1),
    (mo::CH_RESET_MOTION, 1),
    (mo::CH_SET_SYS_MOTION, 3),
    (mo::CH_MOTION_SET_NEXT_HOKAN, 2),
    (mo::CH_MOTION_SET_HOKAN, 2),
    (mo::CH_GET_SEQ_FRAME, 1),
];
const POP: u32 = 0;
const ID: u32 = 1;
const FRAMES: u32 = 6;

fn game_call(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let recs = scratch(a, b, 0x100 * CALLEES.len());
    let rec = |k: u32| recs + 0x100 * k;
    for o in [&mut *a, &mut *b] {
        for (k, &(f, n)) in (0u32..).zip(&CALLEES) {
            logging_stub(o, f, GUI_STUBS + 0x40 * k, rec(k), n, 0);
        }
    }
    let model = scratch(a, b, 0x800);
    let id = scratch(a, b, 4);
    set(a, b, ffx_battle::effect::bridge::MOTION_STOP_CHRS, 0);
    for _ in 0..4000 {
        let i = match rng.below(6) {
            0 => 0xff,
            1 => rng.below(0x100),
            _ => rng.below(ACTORS),
        };
        poke(a, b, id, &[u8::try_from(i).unwrap()]);
        let p = w.actor(i.min(ACTORS - 1));
        if i < ACTORS {
            let chr = match rng.below(6) {
                0 => 0,
                // the snapshot's own model (its motion speed is plain memory)
                1 if !w.synthetic => ffx_bridge::Mem::read(&*b, p),
                _ => model,
            };
            poke(a, b, p, &chr.to_le_bytes());
            let flag = |rng: &mut Xorshift| [0u8, 0, 1, rng.byte()][rng.below(4) as usize];
            for o in [0x38, 0x3f3, 0x404, 0x405, 0x4ac, 0x12, 0xdce, 0x613, 0x614] {
                poke(a, b, p + o, &[flag(rng)]);
            }
            poke(a, b, p + 0x38, &[[0, 1, 1, 1][rng.below(4) as usize]]);
            poke(a, b, p + 0x3f3, &[[0, 0, 0, 1][rng.below(4) as usize]]);
            for o in [0x20, 0x3f0, 0x3f1, 0x406, 0x408, 0x409, 0x434] {
                poke(a, b, p + o, &[rng.byte()]);
            }
            for o in [0x3fc, 0x400, 0x402] {
                let v = [0u16, 0x100, 0x80, 1, rng.word().to_le_bytes()[0].into()]
                    [rng.below(5) as usize];
                poke(a, b, p + o, &v.to_le_bytes());
            }
            // no motion table (MsMotBinFrame 0)
            poke(a, b, p + 0x4f8, &[0; 4]);
        }
        let motion = match rng.below(5) {
            0 => rng.next(),
            1 => 0x8000 | rng.below(0x100),
            2 => [1, 2, 3, 4, 5, 6, 0xc, 0xd, 0x82][rng.below(9) as usize],
            _ => rng.below(0x100),
        };
        let frames = [0, 1, -1, 30, rng.next().cast_signed()][rng.below(5) as usize];
        for o in [&mut *a, &mut *b] {
            o.write(rec(POP) + 4, &motion.to_le_bytes()).unwrap();
            o.write(rec(ID) + 4, &id.to_le_bytes()).unwrap();
            o.write(rec(FRAMES) + 4, &frames.to_le_bytes()).unwrap();
        }
        let worker = rng.next();
        let stack = rng.next();
        let flag = rng.next();
        let args = [worker, rng.next(), stack, flag];
        twin(
            "Battle_ScriptPlayMotion",
            a,
            b,
            0x007a_8fc0,
            &args,
            ALL,
            |m| {
                let mut call = game_call;
                mo::battle_script_play_motion(m, &mut call, worker, stack, flag.to_le_bytes()[0])
            },
        );
        same("Battle_ScriptPlayMotion", a, b);
    }
}

#[test]
fn script_motions_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23f1_0001), true);
}

#[test]
fn script_motions_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23f1_0002), false);
}
